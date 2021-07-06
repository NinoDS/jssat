//! Assembles together the informatino from the original IR, and type_annotator
//! passes into a fully conherent IR, which will then be passed to `skeleton`.

use std::collections::VecDeque;

use rustc_hash::FxHashMap;

use super::{
    conv_only_bb::{self, Block as BBlock},
    ir,
    ir::{FFIValueType, IR},
    type_annotater::{BlockKey, SymbolicEngine, ValueType},
};
use crate::{
    frontend::{
        ir::BasicBlockJump,
        type_annotater::{BlockExecutionKey, ExplorationBranch},
    },
    id::*,
};

#[derive(Clone, Debug)]
pub struct Program {
    constants: FxHashMap<ConstantId<AssemblerCtx>, Vec<u8>>,
    external_functions: FxHashMap<ExternalFunctionId<AssemblerCtx>, ExternalFunction>,
    functions: FxHashMap<FunctionId<AssemblerCtx>, Function>,
}

#[derive(Clone, Debug)]
pub struct ExternalFunction {
    name: String,
    parameters: Vec<Type>,
    returns: ReturnType,
}

#[derive(Clone, Debug)]
pub struct Function {
    entry_block: BlockId<AssemblerCtx>,
    blocks: FxHashMap<BlockId<AssemblerCtx>, Block>,
}

#[derive(Clone, Debug)]
pub struct Block {
    parameters: Vec<Parameter>,
    instructions: Vec<Instruction>,
    end: EndInstruction,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    typ: Type,
    // TODO: maybe parameters should implicitly get the register accoridng to
    // their index? it makes sense not to do this in the other IRs because of
    // mangling parameters, but here we have pretty much all the information
    // necessary to craft a final product
    register: RegisterId<AssemblerCtx>,
}

#[derive(Clone, Debug)]
pub enum Instruction {
    Call(
        Option<RegisterId<AssemblerCtx>>,
        Callable,
        Vec<RegisterId<AssemblerCtx>>,
    ),
    GetRuntime(RegisterId<AssemblerCtx>),
    MakeString(RegisterId<AssemblerCtx>, ConstantId<AssemblerCtx>),
    MakeNumber(RegisterId<AssemblerCtx>, f64),
    MakeBoolean(RegisterId<AssemblerCtx>, bool),
    Unreachable,
}

#[derive(Clone, Debug)]
pub enum Callable {
    Extern(ExternalFunctionId<IrCtx>),
    Static(FunctionId<AssemblerCtx>),
}

#[derive(Clone, Debug)]
pub enum EndInstruction {
    Unreachable,
    Jump(BlockJump),
    JumpIf {
        condition: RegisterId<AssemblerCtx>,
        true_path: BlockJump,
        false_path: BlockJump,
    },
    Return(Option<RegisterId<AssemblerCtx>>),
}

#[derive(Clone, Debug)]
pub struct BlockJump(BlockId<AssemblerCtx>, Vec<RegisterId<AssemblerCtx>>);

#[derive(Clone, Debug)]
pub enum ReturnType {
    Void,
    Value(Type),
}

#[derive(Clone, Debug)]
pub enum Type {
    FFI(FFIValueType),
    Val(ValueType),
}

pub fn assemble(ir: IR, blocks: Vec<BBlock>, engine: SymbolicEngine) -> Program {
    Assembler::new(ir, blocks, engine).assemble()
}

fn map_ext_fn(exernal_function: ir::ExternalFunction) -> ExternalFunction {
    ExternalFunction {
        name: exernal_function.name,
        parameters: exernal_function
            .parameters
            .into_iter()
            .map(ir::FFIValueType::to_type)
            .collect(),
        returns: exernal_function.return_type.to_type(),
    }
}

impl ir::FFIReturnType {
    fn to_type(self) -> ReturnType {
        match self {
            ir::FFIReturnType::Void => ReturnType::Void,
            ir::FFIReturnType::Value(v) => ReturnType::Value(v.to_type()),
        }
    }
}

impl ir::FFIValueType {
    fn to_type(self) -> Type {
        Type::FFI(self)
    }
}

struct Assembler {
    ir: IR,
    blocks: Vec<BBlock>,
    engine: SymbolicEngine,
    //
    constants: FxHashMap<ConstantId<AssemblerCtx>, Vec<u8>>,
    constant_id_gen: Counter<ConstantId<AssemblerCtx>>,
    external_functions: FxHashMap<ExternalFunctionId<AssemblerCtx>, ExternalFunction>,
    functions: FxHashMap<FunctionId<AssemblerCtx>, Function>,
    //
    function_ids: FxHashMap<(FunctionId<IrCtx>, BlockId<IrCtx>), FunctionId<AssemblerCtx>>,
}

impl Assembler {
    pub fn new(mut ir: IR, blocks: Vec<BBlock>, engine: SymbolicEngine) -> Self {
        let external_functions = (ir.external_functions.into_iter())
            .map(|(k, v)| (k.map_context::<AssemblerCtx>(), map_ext_fn(v)))
            .collect::<FxHashMap<_, _>>();

        // hack so we can still have a owned `ir` everywhere
        ir.external_functions = Default::default();

        Self {
            ir,
            blocks,
            engine,
            constants: Default::default(),
            constant_id_gen: Default::default(),
            external_functions,
            functions: Default::default(),
            function_ids: Default::default(),
        }
    }

    pub fn assemble(mut self) -> Program {
        self.assign_fn_ids();

        let mut built_fns = Vec::new();

        // go through every function in the program and assemble it
        for (fn_id, entry_blk, args, cntrl_flw) in self
            .engine
            .executions
            .all_fn_invocations()
            .filter(|(fn_id, blk_id, _, _)| {
                // only allow blocks that are the entry block
                self.ir.functions.get(fn_id).unwrap().entry_block == *blk_id
            })
        {
            // at this point, we will have only function ids, their entry blocks,
            // and the pairs of arguments passed to the function to invoke
            let fn_assembler =
                FnAssembler::new(&self, fn_id, entry_blk, args.clone(), cntrl_flw.key());

            let assembled_fn = fn_assembler.assemble();
            built_fns.push((fn_id, entry_blk, assembled_fn));
        }

        for (fn_id, blk_id, func) in built_fns {
            self.functions.insert(self.get_fn_id(fn_id, blk_id), func);
        }

        Program {
            constants: self.constants,
            external_functions: self.external_functions,
            functions: self.functions,
        }
    }

    fn get_fn_id(
        &self,
        fn_id: FunctionId<IrCtx>,
        blk_id: BlockId<IrCtx>,
    ) -> FunctionId<AssemblerCtx> {
        *self.function_ids.get(&(fn_id, blk_id)).unwrap()
    }

    fn assign_fn_ids(&mut self) {
        let id_gen = Counter::new();

        let functions = &self.ir.functions;
        for (fn_id, entry_blk, _, _) in
            self.engine
                .executions
                .all_fn_invocations()
                .filter(|(fn_id, blk_id, _, _)| {
                    // only allow blocks that are the entry block
                    functions.get(fn_id).unwrap().entry_block == *blk_id
                })
        {
            self.function_ids.insert((fn_id, entry_blk), id_gen.next());
        }
    }

    fn find_block(&self, fn_id: FunctionId<IrCtx>, blk_id: BlockId<IrCtx>) -> &BBlock {
        let matches = self
            .blocks
            .iter()
            .filter(|b| b.derived_from == (fn_id, blk_id))
            .collect::<Vec<_>>();

        debug_assert_eq!(matches.len(), 1);
        matches[0]
    }

    fn find_what(
        &self,
        orig_fn_id: FunctionId<IrCtx>,
        orig_arg_types: Vec<&ValueType>,
    ) -> BlockId<IrCtx> {
        for (fn_id, blk_id, arg_types, t) in self.engine.executions.all_fn_invocations() {
            let arg_types = arg_types.iter().map(|a| a).collect::<Vec<_>>();
            if orig_fn_id == fn_id && orig_arg_types == arg_types {
                return blk_id;
            }
        }
        unreachable!("how he how")
    }
}

struct FnAssembler<'duration> {
    assembler: &'duration Assembler,
    /// ~~**original** function ID, present in IR~~ not true?????
    function_id: FunctionId<IrCtx>,
    /// ~~**original** entry block, present in IR~~ not true?????
    entry_block: BlockId<IrCtx>,
    invocation_args: Vec<ValueType>,
    /// **modified** key, present in basic block
    block_key: BlockKey,
    block_id_map: BlockIdMap<IrCtx, AssemblerCtx>,
    /// we can ONLY use this because we know that the `conv_only_bb` step will
    /// only generate registers abve all IR registers, which means that there
    /// will never be a clash. this might have to be reconsidered later
    register_map: RegIdMap<PureBbCtx, AssemblerCtx>,
    constants: FxHashMap<ConstantId<AssemblerCtx>, Vec<u8>>,
}

impl<'d> FnAssembler<'d> {
    pub fn new(
        assembler: &'d Assembler,
        function_id: FunctionId<IrCtx>,
        entry_block: BlockId<IrCtx>,
        invocation_args: Vec<ValueType>,
        block_key: BlockKey,
    ) -> Self {
        Self {
            assembler,
            function_id,
            entry_block,
            invocation_args,
            block_key,
            block_id_map: Default::default(),
            register_map: Default::default(),
            constants: Default::default(),
        }
    }

    pub fn assemble(mut self) -> Function {
        let mut blocks = FxHashMap::default();

        let mut blocks_to_assemble = VecDeque::new();

        blocks_to_assemble.push_back((
            // entry block
            // TODO: is the context of this the OG IR or the function?
            self.entry_block,
            std::mem::take(&mut self.invocation_args),
        ));

        let typed_fn = (self.assembler.engine.typed_blocks)
            .get(&self.block_key)
            .unwrap();

        let mut iterations = 0;
        while let Some((block, args)) = blocks_to_assemble.pop_front() {
            iterations += 1;
            let id = self.id_of(block);

            let (branch, register_types) = typed_fn.find(&block, &args);
            let block_src = self.assembler.find_block(self.function_id, block);

            println!("----");
            println!("current bn;ocks: {:?}", self.assembler.blocks);
            debug_assert_eq!(
                args.len(),
                block_src.parameters.len(),
                "fn.block {:?}.{:?} on {:?}th time",
                self.function_id,
                block,
                iterations
            );

            // LOGIC(param_dropping): this is the logic that determines which
            // parameters are dropped, it may need to be encapsulated somewhere
            // else for easier refactoring
            let parameters = (args.iter())
                .zip(block_src.parameters.iter())
                // we can ignore simple parameters, as reconstructing those
                // params is cheaper than passing them along
                //
                // this comes at the cost of us needing to fill in the constant
                // values anytime we use these parameters
                .filter(|(typ, _)| !typ.is_simple())
                .map(|(typ, register)| Parameter {
                    typ: typ.to_type(),
                    register: register.map_context::<AssemblerCtx>(),
                })
                .collect::<Vec<_>>();

            let block_id_map = &mut self.block_id_map;

            let assembler_id_gen = &self.assembler.constant_id_gen;
            let constants = &mut self.constants;
            let ir = &self.assembler.ir;
            let intern_cnst = |orig_id| {
                let id = assembler_id_gen.next();
                let payload = ir.constants.get(&orig_id).unwrap();
                constants.insert(id, payload.payload.clone());
                id
            };

            let mut inst_writer = InstWriter::new(
                parameters,
                &mut self.register_map,
                register_types,
                |id| block_id_map.map(id),
                intern_cnst,
                &self.assembler,
            );

            for inst in block_src.instructions.iter() {
                let should_keep_translating = inst_writer.translate(inst);
                if !should_keep_translating {
                    break;
                }
            }

            blocks.insert(id, inst_writer.complete(&block_src.end));

            match branch {
                ExplorationBranch::Branch(branches) => {
                    // explore those branches!
                    for BlockExecutionKey {
                        function,
                        block,
                        parameters,
                    } in branches
                    {
                        blocks_to_assemble.push_back((
                            // entry block
                            // TODO: is the context of this the OG IR or the function?
                            *block,
                            parameters.clone(),
                        ));
                    }
                }
                ExplorationBranch::Complete(_) => {
                    // do nothing, we're done
                }
            };
        }

        Function {
            entry_block: self.id_of(self.entry_block),
            blocks,
        }
    }

    fn id_of(&mut self, blk_id: BlockId<IrCtx>) -> BlockId<AssemblerCtx> {
        self.block_id_map.map(blk_id)
    }
}

struct InstWriter<'assembler, F, I> {
    parameters: Vec<Parameter>,
    instructions: Vec<Instruction>,
    register_map: &'assembler mut RegIdMap<PureBbCtx, AssemblerCtx>,
    map_block: F,
    intern_constant: I,
    register_types: &'assembler FxHashMap<RegisterId<PureBbCtx>, ValueType>,
    never_infected: bool,
    assembler: &'assembler Assembler,
}

impl<'a, F, I> InstWriter<'a, F, I>
where
    F: FnMut(BlockId<IrCtx>) -> BlockId<AssemblerCtx>,
    I: FnMut(ConstantId<IrCtx>) -> ConstantId<AssemblerCtx>,
{
    pub fn new(
        parameters: Vec<Parameter>,
        register_map: &'a mut RegIdMap<PureBbCtx, AssemblerCtx>,
        register_types: &'a FxHashMap<RegisterId<PureBbCtx>, ValueType>,
        map_block: F,
        intern_constant: I,
        assembler: &'a Assembler,
    ) -> Self {
        Self {
            instructions: Default::default(),
            parameters,
            register_map,
            register_types,
            never_infected: false,
            map_block,
            assembler,
            intern_constant,
        }
    }

    fn is_simple(&self, register: RegisterId<PureBbCtx>) -> bool {
        InstWriter::<'a, F, I>::_is_simple(&self.register_types, register)
    }

    fn _is_simple(
        register_types: &FxHashMap<RegisterId<PureBbCtx>, ValueType>,
        register: RegisterId<PureBbCtx>,
    ) -> bool {
        register_types.get(&register).unwrap().is_simple()
    }

    pub fn translate(&mut self, inst: &ir::Instruction<PureBbCtx>) -> bool {
        match inst {
            // "simple" instructions should be ignored, as we can easily insert
            // these instructions exactly when they are required. with our opt
            // passes, we should be able to completely elide creation of simple
            // values too!

            // dead-simple instructions, always will be simple
            ir::Instruction::MakeString(_, _)
            | ir::Instruction::MakeNumber(_, _)
            | ir::Instruction::GetRuntime(_) => {}

            // calculatably simple instructions
            ir::Instruction::CompareLessThan(result, _, _) | ir::Instruction::Add(result, _, _)
                if self.is_simple(*result) => {}

            // the remaining instructions are decidedly complex

            // the simple cases are handled above
            ir::Instruction::CompareLessThan(result, lhs, rhs) => {
                //
                todo!()
            }

            ir::Instruction::Add(result, lhs, rhs) => {
                //
                todo!()
            }

            // the rest are complicated
            // a `call` function may invoke an external function, whose arguments
            // may not be elided whatsoever. because calling a JSSAT function
            // may invoke external functions as well, by induction, we may not
            // elide the arguments to other JSSAT functions.
            ir::Instruction::Call(result, function, args) => {
                let original_args = args;

                let args = match function {
                    ir::Callable::External(ext_id) => {
                        // in external functions, we can never omit any params
                        for simple_arg in args.iter() {
                            if self.is_simple(*simple_arg) {
                                self.compute_simple(*simple_arg);
                            }
                        }

                        args.iter()
                            .map(|r| self.register_map.map(*r))
                            .collect::<Vec<_>>()
                    }
                    ir::Callable::Static(_) => {
                        // LOGIC(param_dropping): this is the logic that determines which
                        // parameters to pass to a block, as some parameters may be
                        // dropped. the complexity comes from having to ensure that this
                        // piece of logic aligns with the other piece of logic related
                        // to this one

                        // TODO: we know that we can always drop simple arguments
                        // because we never have code in the type_annotator phase that
                        // bails when a function gets too recursive, meaning we will
                        // never have a jump with simple args to a block that accepts
                        // generic/abstract args (i.e. Jmp A(0) -> A(Int) is impossible)
                        let register_types = self.register_types;
                        args.iter()
                            .filter(|r| !InstWriter::<'a, F, I>::_is_simple(register_types, **r))
                            .map(|r| self.register_map.map(*r))
                            .collect::<Vec<_>>()
                    }
                };

                let result = (*result).map(|r| self.register_map.map(r));
                self.instructions.push(Instruction::Call(
                    result,
                    match function {
                        ir::Callable::External(id) => Callable::Extern(*id),
                        ir::Callable::Static(id) => {
                            // find the block id that most matches the args passed to the function
                            let blk_id = {
                                self.assembler.find_what(
                                    *id,
                                    original_args
                                        .iter()
                                        .map(|a| self.register_types.get(a).unwrap())
                                        .collect(),
                                )
                            };

                            Callable::Static(self.assembler.get_fn_id(*id, blk_id))
                        }
                    },
                    args,
                ));
            }
        }

        true
    }

    fn compute_simple(&mut self, arg: RegisterId<PureBbCtx>) -> RegisterId<AssemblerCtx> {
        debug_assert!(self.is_simple(arg));

        let reg = self.register_map.map(arg);

        match self.register_types.get(&arg).unwrap() {
            ValueType::Any
            | ValueType::String
            | ValueType::Number
            | ValueType::BytePointer
            | ValueType::Pointer(_)
            | ValueType::Word
            | ValueType::Boolean => unreachable!("cannot be simple yet get here"),
            ValueType::Runtime => {
                self.instructions.push(Instruction::GetRuntime(reg));
            }
            ValueType::ExactString(id) => {
                self.instructions
                    .push(Instruction::MakeString(reg, (self.intern_constant)(*id)));
            }
            ValueType::ExactNumber(n) => {
                self.instructions.push(Instruction::MakeNumber(reg, *n));
            }
            ValueType::Bool(b) => {
                self.instructions.push(Instruction::MakeBoolean(reg, *b));
            }
        };

        reg
    }

    fn map_basic_block_jump(&mut self, bbjump: &BasicBlockJump<PureBbCtx, IrCtx>) -> BlockJump {
        let BasicBlockJump(id, args) = bbjump;

        // LOGIC(param_dropping): this is the logic that determines which
        // parameters to pass to a block, as some parameters may be
        // dropped. the complexity comes from having to ensure that this
        // piece of logic aligns with the other piece of logic related
        // to this one

        // TODO: we know that we can always drop simple arguments
        // because we never have code in the type_annotator phase that
        // bails when a function gets too recursive, meaning we will
        // never have a jump with simple args to a block that accepts
        // generic/abstract args (i.e. Jmp A(0) -> A(Int) is impossible)
        let register_types = self.register_types;
        let register_map = &mut self.register_map;
        let args = (args.iter())
            .filter(|arg| !InstWriter::<'a, F, I>::_is_simple(register_types, **arg))
            .copied()
            .map(|r| register_map.map(r))
            .collect();

        BlockJump((self.map_block)(*id), args)
    }

    pub fn complete(mut self, end: &ir::ControlFlowInstruction<PureBbCtx, IrCtx>) -> Block {
        let end = if self.never_infected {
            EndInstruction::Unreachable
        } else {
            match end {
                ir::ControlFlowInstruction::Jmp(bbjump) => {
                    EndInstruction::Jump(self.map_basic_block_jump(bbjump))
                }
                ir::ControlFlowInstruction::JmpIf {
                    condition,
                    true_path,
                    false_path,
                } => {
                    // // if this assertion doesn't hold, we can just manually insert
                    // // the constant jump ourselves
                    // debug_assert!(
                    //     !matches!(
                    //         self.register_types.get(condition).unwrap(),
                    //         ValueType::Bool(_)
                    //     ),
                    //     "shouldn't be conditionalling jumping when path is known"
                    // );

                    match self.register_types.get(condition).unwrap() {
                        ValueType::Bool(true) => {
                            EndInstruction::Jump(self.map_basic_block_jump(true_path))
                        }
                        ValueType::Bool(false) => {
                            EndInstruction::Jump(self.map_basic_block_jump(false_path))
                        }
                        ValueType::Boolean => EndInstruction::JumpIf {
                            condition: self.register_map.map(*condition),
                            true_path: self.map_basic_block_jump(true_path),
                            false_path: self.map_basic_block_jump(false_path),
                        },
                        _ => unreachable!("condition register should be only bools"),
                    }
                }
                ir::ControlFlowInstruction::Ret(register) => {
                    EndInstruction::Return((*register).map(|r| self.register_map.map(r)))
                }
            }
        };

        Block {
            parameters: self.parameters,
            instructions: self.instructions,
            end,
        }
    }
}

impl ValueType {
    pub fn to_type(&self) -> Type {
        Type::Val(self.clone())
    }

    pub fn is_const(&self) -> bool {
        match self {
            ValueType::Any
            | ValueType::Runtime
            | ValueType::String
            | ValueType::Number
            | ValueType::BytePointer
            | ValueType::Pointer(_)
            | ValueType::Word
            | ValueType::Boolean => false,
            ValueType::ExactNumber(_) | ValueType::ExactString(_) | ValueType::Bool(_) => true,
        }
    }

    /// States whether the type is "simple" or not. Simple types are extremely
    /// cheap to rebuild at any moment in the IR, and are considered cheaper to
    /// build than to pass around. Passing them around is considered "expensive"
    /// because then we're using more registers than necessary, which cause
    /// performance deficits because the more registers we use the more likely
    /// we'll need to spill onto the stack to generate a function.
    pub fn is_simple(&self) -> bool {
        matches!(
            self,
            ValueType::Runtime
                | ValueType::ExactNumber(_)
                | ValueType::Bool(_)
                | ValueType::ExactString(_)
        )
    }
}
