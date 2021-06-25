use rustc_hash::FxHashMap;
use std::{hash::Hash, marker::PhantomData};

use super::BuildArtifact;
use crate::id::*;

#[cfg(feature = "link-llvm")]
use inkwell::{
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    targets::{CodeModel, FileType, RelocMode, Target, TargetMachine},
    types::{BasicType, BasicTypeEnum, IntType, StructType},
    values::{BasicValue, BasicValueEnum, FunctionValue, GlobalValue},
    AddressSpace, OptimizationLevel,
};

#[cfg(not(feature = "link-llvm"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionValue<'a>(PhantomData<&'a ()>);

pub struct BackendIR<'name> {
    pub constants: FxHashMap<ConstantId, Constant<'name>>,
    pub opaque_structs: FxHashMap<OpaqueStructId, OpaqueStruct<'name>>,
    pub external_functions: FxHashMap<ExternalFunctionId, ExternalFunction>, // <'name>>,
    pub functions: FxHashMap<FunctionId, Function<'name>>,
}

pub struct Constant<'name> {
    pub name: &'name str,
    pub payload: Vec<u8>,
}

pub struct OpaqueStruct<'name> {
    pub name: &'name str,
}

pub struct ExternalFunction {
    // <'name> {
    // pub name: &'name str,
    // TODO: fix the ugly hack (reason being we add external fns during the
    // translation of IR + TypeAnnotations -> BackendIr)
    pub name: String,
    pub return_type: ReturnType,
    pub parameters: Vec<ValueType>,
}

pub struct Function<'name> {
    pub name: &'name str,
    pub linkage: Option<LLVMLinkage>,
    pub return_type: ReturnType,
    pub parameters: Vec<Parameter>,
    pub entry_block: BlockId,
    pub blocks: FxHashMap<BlockId, Vec<Instruction>>,
}

#[derive(Debug)]
pub struct PartialFunction<'ctx> {
    pub llvm: FunctionValue<'ctx>,
    pub parameters: Vec<RegisterId>,
    pub entry_block: BlockId,
    pub blocks: FxHashMap<BlockId, Vec<Instruction>>,
}

pub struct Parameter {
    pub r#type: ValueType,
    pub register: RegisterId,
}

#[derive(Debug)]
pub enum Instruction {
    /// # [`Instruction::LoadConstantPtr`]
    ///
    /// Loads the value of the constant as a `i8*` into the register specified.
    LoadConstantPtr(RegisterId, ConstantId),
    /// # [`Instruction::LoadConstantLen`]
    ///
    /// Loads the length of the payload of the constant as a word-sized valaue
    /// into the register specified.
    LoadConstantLen(RegisterId, ConstantId),
    Call(Option<RegisterId>, Callable, Vec<RegisterId>),
    /// # [`Instruction::Return`]
    ///
    /// Returns the value in the register to the caller, or returns nothing if
    /// no register is given.
    Return(Option<RegisterId>),
}

#[derive(Debug)]
pub enum Callable {
    External(ExternalFunctionId),
    Static(FunctionId),
    // Virtual(RegisterId),
}

pub enum ReturnType {
    Void,
    Value(ValueType),
}

pub enum ValueType {
    WordSizeBitType,
    BitType(u16),
    Opaque(OpaqueStructId),
    Pointer(Box<ValueType>),
}

pub enum LLVMLinkage {
    External,
}

impl LLVMLinkage {
    #[cfg(feature = "link-llvm")]
    pub fn to_llvm(&self) -> Linkage {
        match self {
            LLVMLinkage::External => Linkage::External,
        }
    }
}

#[cfg(not(feature = "link-llvm"))]
pub fn target_triplet() -> String {
    "not implemented".into()
}

#[cfg(feature = "link-llvm")]
pub fn target_triplet() -> String {
    TargetMachine::get_default_triple()
        .as_str()
        .to_str()
        .unwrap()
        .to_owned()
}

#[cfg(not(feature = "link-llvm"))]
pub fn compile(_ir: BackendIR) -> BuildArtifact {
    panic!("link-llvm not enabled");
}

#[cfg(feature = "link-llvm")]
pub fn compile(ir: BackendIR) -> BuildArtifact {
    let context = Context::create();
    let builder = context.create_builder();
    let module = context.create_module("jssat");

    #[cfg(target_pointer_width = "32")]
    let word_size = context.i32_type();
    #[cfg(target_pointer_width = "64")]
    let word_size = context.i64_type();

    let compiler = BackendCompiler {
        context: &context,
        builder,
        module: &module,
        word_size,
    };

    let mut constants = FxHashMap::default();
    for (id, constant) in ir.constants.into_iter() {
        constants.insert(
            id,
            (constant.payload.len(), compiler.llvm_constant(constant)),
        );
    }
    let constant_resolver = ConstantResolver { things: &constants };

    let mut opaque_structs = FxHashMap::default();
    for (id, opaque_struct) in ir.opaque_structs.into_iter() {
        opaque_structs.insert(id, compiler.llvm_opaque_struct(opaque_struct));
    }
    let opaque_struct_resolver = OpaqueStructResolver {
        things: &opaque_structs,
    };

    let mut external_functions = FxHashMap::default();
    for (id, external_function) in ir.external_functions.into_iter() {
        let external_function =
            compiler.llvm_external_function(external_function, &opaque_struct_resolver);
        external_functions.insert(id, external_function);
    }
    let external_function_resolver = ExternalFunctionResolver {
        things: &external_functions,
    };

    let mut functions = FxHashMap::default();
    for (id, function) in ir.functions.into_iter() {
        let function = compiler.llvm_function_start(function, &opaque_struct_resolver);
        functions.insert(id, function);
    }

    let function_types = functions
        .iter()
        .map(|(k, v)| (*k, v.llvm))
        .collect::<FxHashMap<_, _>>();
    let function_resolver = FunctionResolver {
        things: &function_types,
    };

    for (_, function) in functions {
        compiler.llvm_function_end(
            function,
            &constant_resolver,
            &opaque_struct_resolver,
            &external_function_resolver,
            &function_resolver,
        );
    }

    // do actual LLVM compilation

    #[cfg(debug_assertions)]
    {
        // print llvm ir incase llvm segfaults while compiling
        let text_buff = module.print_to_string().to_string();
        println!(
            "EARLY LLVM IR:\n=== LLVM START\n{}\n=== LLVM END",
            text_buff
        );
    }

    Target::initialize_all(&Default::default());
    let target_triple = TargetMachine::get_default_triple();

    let target = Target::from_triple(&target_triple).unwrap();

    let target_machine = target
        .create_target_machine(
            &target_triple,
            "generic",
            "",
            OptimizationLevel::Aggressive,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .expect("couldn't make target machine");

    let text_buff = module.print_to_string().to_string();

    let obj_buff = target_machine
        .write_to_memory_buffer(&module, FileType::Object)
        .expect("couldn't compile to assembly");

    BuildArtifact {
        llvm_ir: text_buff.to_string(),
        obj: obj_buff.as_slice().to_vec(),
    }
}

#[cfg(feature = "link-llvm")]
type ConstantResolver<'structs, 'ctx> = Resolver<'structs, ConstantId, (usize, GlobalValue<'ctx>)>;

#[cfg(feature = "link-llvm")]
type OpaqueStructResolver<'structs, 'ctx> = Resolver<'structs, OpaqueStructId, StructType<'ctx>>;

#[cfg(feature = "link-llvm")]
type ExternalFunctionResolver<'structs, 'ctx> =
    Resolver<'structs, ExternalFunctionId, FunctionValue<'ctx>>;

#[cfg(feature = "link-llvm")]
type FunctionResolver<'structs, 'ctx> = Resolver<'structs, FunctionId, FunctionValue<'ctx>>;

#[cfg(feature = "link-llvm")]
struct Resolver<'map, K, V> {
    things: &'map FxHashMap<K, V>,
}

#[cfg(feature = "link-llvm")]
impl<K, V> Resolver<'_, K, V>
where
    K: Hash + Eq,
    V: Copy,
{
    pub fn resolve(&self, id: &K) -> V {
        *self.things.get(id).expect("expected thing at key")
    }
}

#[cfg(feature = "link-llvm")]
struct BackendCompiler<'ctx, 'module> {
    context: &'ctx Context,
    builder: Builder<'ctx>,
    module: &'module Module<'ctx>,
    word_size: IntType<'ctx>,
}

#[cfg(feature = "link-llvm")]
impl<'c> BackendCompiler<'c, '_> {
    pub fn llvm_constant(&self, constant: Constant) -> GlobalValue<'c> {
        let raw_const = self.constant_payload_to_basic_value(constant.payload);

        let global = self.module.add_global(
            raw_const.get_type(),
            Some(AddressSpace::Generic),
            constant.name,
        );

        global.set_linkage(Linkage::Private);
        global.set_unnamed_addr(true);
        global.set_constant(true);
        global.set_initializer(&raw_const);

        global
    }

    fn constant_payload_to_basic_value(&self, payload: Vec<u8>) -> BasicValueEnum<'c> {
        match std::str::from_utf8(payload.as_slice()) {
            Ok(str) => self
                .context
                .const_string(str.as_bytes(), false)
                .as_basic_value_enum(),
            Err(_) => {
                let byte_values = payload
                    .into_iter()
                    .map(|n| self.context.i8_type().const_int(n as u64, false))
                    .collect::<Vec<_>>();

                self.context
                    .i8_type()
                    .const_array(byte_values.as_slice())
                    .as_basic_value_enum()
            }
        }
    }

    pub fn llvm_opaque_struct(&self, opaque_struct: OpaqueStruct) -> StructType<'c> {
        self.context.opaque_struct_type(opaque_struct.name)
    }

    pub fn llvm_external_function(
        &self,
        external_function: ExternalFunction,
        opaque_struct_resolver: &OpaqueStructResolver<'_, 'c>,
    ) -> FunctionValue<'c> {
        let parameter_types = external_function
            .parameters
            .into_iter()
            .map(|v| self.llvm_typeify_value(v, &opaque_struct_resolver))
            .collect::<Vec<_>>();

        let parameter_types = parameter_types.as_slice();

        let function = match external_function.return_type {
            ReturnType::Void => self.context.void_type().fn_type(parameter_types, false),
            ReturnType::Value(v) => self
                .llvm_typeify_value(v, &opaque_struct_resolver)
                .fn_type(parameter_types, false),
        };

        self.module.add_function(
            external_function.name.as_str(),
            function,
            Some(Linkage::External),
        )
    }

    pub fn llvm_function_start(
        &self,
        function: Function,
        opaque_struct_resolver: &OpaqueStructResolver<'_, 'c>,
    ) -> PartialFunction<'c> {
        let mut parameter_registers = Vec::with_capacity(function.parameters.len());

        let parameter_types = function
            .parameters
            .into_iter()
            .map(|v| {
                parameter_registers.push(v.register);
                self.llvm_typeify_value(v.r#type, &opaque_struct_resolver)
            })
            .collect::<Vec<_>>();

        let parameter_types = parameter_types.as_slice();

        let llvm_function = match function.return_type {
            ReturnType::Void => self.context.void_type().fn_type(parameter_types, false),
            ReturnType::Value(v) => self
                .llvm_typeify_value(v, &opaque_struct_resolver)
                .fn_type(parameter_types, false),
        };

        PartialFunction {
            llvm: self.module.add_function(
                function.name,
                llvm_function,
                function.linkage.map(|l| l.to_llvm()),
            ),
            parameters: parameter_registers,
            entry_block: function.entry_block,
            blocks: function.blocks,
        }
    }

    pub fn llvm_function_end(
        &self,
        mut function: PartialFunction<'c>,
        constant_resolver: &ConstantResolver<'_, 'c>,
        _opaque_struct_resolver: &OpaqueStructResolver<'_, 'c>,
        external_function_resolver: &ExternalFunctionResolver<'_, 'c>,
        function_resolver: &FunctionResolver<'_, 'c>,
    ) {
        println!("llvm_function_end: {:?} -> {:#?}", function, function);

        let entry_block_id = function.entry_block;
        let entry_block = function
            .blocks
            .remove(&entry_block_id)
            .expect("entry block");
        let non_entry_blocks = function.blocks.into_iter().map(|(_, v)| v);

        let mut register_values = FxHashMap::default();

        for block in std::iter::once(entry_block).chain(non_entry_blocks) {
            let basic_block = self.context.append_basic_block(function.llvm, "");
            self.builder.position_at_end(basic_block);

            for instruction in block.into_iter() {
                match instruction {
                    Instruction::Call(result, function, args) => {
                        let llvm_callable = match function {
                            Callable::External(id) => external_function_resolver.resolve(&id),
                            Callable::Static(id) => function_resolver.resolve(&id),
                            // Callable::Virtual(_) => todo!(),
                        };

                        let o_args = args.clone();
                        let args = args
                            .into_iter()
                            .map(|r| *register_values.get(&r).unwrap())
                            .collect::<Vec<_>>();

                        println!(
                            "calling {:?} with {:?} :: {:?}",
                            llvm_callable, args, o_args
                        );
                        let llvm_result =
                            self.builder.build_call(llvm_callable, args.as_slice(), "");

                        if let Some(result) = result {
                            register_values.insert(
                                result,
                                llvm_result
                                    .try_as_basic_value()
                                    .left()
                                    .expect("expected value"),
                            );
                        }
                    }
                    Instruction::LoadConstantPtr(result, constant) => {
                        let (_, global) = constant_resolver.resolve(&constant);

                        // if you think i know what i'm doing, you'd be absolutely incorrect

                        // BLACK MAGIC START
                        let ptr = global.as_pointer_value();
                        let get_element_ptr = unsafe {
                            ptr.const_in_bounds_gep(&[self.context.i64_type().const_int(0, false)])
                        };

                        let get_element_ptr = self.builder.build_bitcast(
                            get_element_ptr,
                            self.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        );
                        // BLACK MAGIC END

                        register_values.insert(result, get_element_ptr);
                    }
                    Instruction::LoadConstantLen(result, constant) => {
                        let (len, _) = constant_resolver.resolve(&constant);

                        register_values
                            .insert(result, self.word_size.const_int(len as u64, false).into());
                    }
                    Instruction::Return(None) => {
                        self.builder.build_return(None);
                    }
                    Instruction::Return(Some(register)) => {
                        let value = *register_values.get(&register).unwrap();
                        self.builder.build_return(Some(&value));
                    }
                }
            }
        }
    }

    fn llvm_typeify_value(
        &self,
        value_type: ValueType,
        opaque_struct_resolver: &OpaqueStructResolver<'_, 'c>,
    ) -> BasicTypeEnum<'c> {
        match value_type {
            ValueType::Pointer(inner) => self
                .llvm_typeify_value(*inner, &opaque_struct_resolver)
                .ptr_type(AddressSpace::Generic)
                .as_basic_type_enum(),
            ValueType::WordSizeBitType => self.word_size.as_basic_type_enum(),
            ValueType::Opaque(id) => opaque_struct_resolver.resolve(&id).as_basic_type_enum(),
            ValueType::BitType(bits) => self
                .context
                .custom_width_int_type(bits as u32)
                .as_basic_type_enum(),
        }
    }
}