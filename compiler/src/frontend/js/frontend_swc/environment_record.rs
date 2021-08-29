//! x

use ref_cast::RefCast;
use std::ops::Deref;

use crate::{
    frontend::builder::{DynBlockBuilder, FnSignature, ProgramBuilder, RegisterId},
    isa::InternalSlot,
};

use super::{abstract_operations::EmitterExt, Emitter};

pub struct EnvironmentRecordFactory {
    declarative_environment_vtable: EnvironmentRecordVTable,
    object_environment_vtable: EnvironmentRecordVTable,
    _function_environment_vtable: EnvironmentRecordVTable,
    global_environment_vtable: GlobalEnvironmentRecordVTable,
    // module_environment_vtable: EnvironmentRecordVTable,
}

#[allow(non_snake_case)]
impl EnvironmentRecordFactory {
    pub fn new(writer: &mut ProgramBuilder) -> Self {
        let declarative_environment_vtable =
            EnvironmentRecordFactory::init_vtable_declarative(writer);
        let object_environment_vtable = EnvironmentRecordFactory::init_vtable_object(writer);
        let function_environment_vtable =
            EnvironmentRecordFactory::init_vtable_function(writer, &declarative_environment_vtable);
        let global_environment_vtable = EnvironmentRecordFactory::init_vtable_global(writer);

        Self {
            declarative_environment_vtable,
            object_environment_vtable,
            _function_environment_vtable: function_environment_vtable,
            global_environment_vtable,
        }
    }

    pub fn make_decl_env_rec(&self, block: &mut DynBlockBuilder) -> EnvironmentRecord {
        self.make_env_rec(block, &self.declarative_environment_vtable)
    }

    pub fn make_obj_env_rec(&self, block: &mut DynBlockBuilder) -> EnvironmentRecord {
        self.make_env_rec(block, &self.object_environment_vtable)
    }

    // pub fn make_func_env_rec(&self, block: &mut DynBlockBuilder) -> EnvironmentRecord {
    //     self.make_env_rec(block, &self._function_environment_vtable)
    // }

    pub fn make_global_env_rec(&self, block: &mut DynBlockBuilder) -> GlobalEnvironmentRecord {
        let vtable = &self.global_environment_vtable;
        let env_rec = self.make_env_rec(block, &self.global_environment_vtable.normal);

        let decl_rec = self.make_decl_env_rec(block);
        block.record_set_slot(
            env_rec.register,
            InternalSlot::DeclarativeRecord,
            decl_rec.register,
        );

        let has_var_declaration = block.make_fnptr(vtable.has_var_declaration.id);
        block.record_set_slot(
            env_rec.register,
            InternalSlot::JSSATHasVarDeclaration,
            has_var_declaration,
        );

        let has_lexical_declaration = block.make_fnptr(vtable.has_lexical_declaration.id);
        block.record_set_slot(
            env_rec.register,
            InternalSlot::JSSATHasLexicalDeclaration,
            has_lexical_declaration,
        );

        let has_restricted_global_property =
            block.make_fnptr(vtable.has_restricted_global_property.id);
        block.record_set_slot(
            env_rec.register,
            InternalSlot::JSSATHasRestrictedGlobalProperty,
            has_restricted_global_property,
        );

        let create_global_function_binding =
            block.make_fnptr(vtable.create_global_function_binding.id);
        block.record_set_slot(
            env_rec.register,
            InternalSlot::JSSATCreateGlobalFunctionBinding,
            create_global_function_binding,
        );

        GlobalEnvironmentRecord::new_with_register_unchecked(env_rec.register)
    }

    fn make_env_rec(
        &self,
        block: &mut DynBlockBuilder,
        vtable: &EnvironmentRecordVTable,
    ) -> EnvironmentRecord {
        let record = block.record_new();

        let has_binding = block.make_fnptr(vtable.has_binding.id);
        block.record_set_slot(record, InternalSlot::JSSATHasBinding, has_binding);

        let get_binding_value = block.make_fnptr(vtable.get_binding_value.id);
        block.record_set_slot(
            record,
            InternalSlot::JSSATGetBindingValue,
            get_binding_value,
        );

        EnvironmentRecord::new_with_register_unchecked(record)
    }

    fn init_vtable_declarative(writer: &mut ProgramBuilder) -> EnvironmentRecordVTable {
        let has_binding = {
            let (mut f, [env, N]) = writer.start_function();
            let mut w = f.start_block_main();
            w.comment("DeclarativeEnvironmentRecord::HasBinding");

            //# 1. If envRec has a binding for the name that is the value of N, return true.
            //# 2. Return false.
            let has_binding = w.record_has_prop(env, N);

            f.end_block(w.ret(Some(has_binding)));
            writer.end_function(f)
        };

        let get_binding_value = {
            let (f, [env, N, _S]) = writer.start_function();
            let mut b = Emitter::new(writer, f);

            // https://tc39.es/ecma262/#sec-declarative-environment-records-getbindingvalue-n-s
            b.comment("DeclarativeEnvironmentRecord::GetBindingValue");

            //# 1. Assert: envRec has a binding for N.
            //# 2. If the binding for N in envRec is an uninitialized binding, throw a ReferenceError exception.
            //# 3. Return the value currently bound to N in envRec.
            let result = b.record_get_prop(env, N);
            b.finish(|b| b.ret(Some(result)))
        };

        EnvironmentRecordVTable {
            has_binding,
            get_binding_value,
        }
    }

    fn init_vtable_object(writer: &mut ProgramBuilder) -> EnvironmentRecordVTable {
        let has_binding = {
            let (f, [envRec, N]) = writer.start_function();
            let mut w = Emitter::new(writer, f);
            w.comment("ObjectEnvironmentRecord::HasBinding");

            //# 1. Let bindingObject be envRec.[[BindingObject]].
            let bindingObject = w.record_get_slot(envRec, InternalSlot::BindingObject);

            //# 2. Let foundBinding be ? HasProperty(bindingObject, N).
            // TODO: pull `HasProperty` into a function
            // TODO: care about fallibleness
            let has_prop = w.record_get_slot(bindingObject, InternalSlot::HasProperty);
            let foundBinding = w.call_virt_with_result(has_prop, [bindingObject, N]);

            //# 3. If foundBinding is false, return false.
            let fnd_bnd_false = w.negate(foundBinding);
            let r#false = w.make_bool(false);
            w.if_then_end(fnd_bnd_false, |w| {
                let completion = w.NormalCompletion(r#false);
                move |b| b.ret(Some(completion))
            });

            //# 4. If envRec.[[IsWithEnvironment]] is false, return true.
            let with_env = w.record_get_slot(envRec, InternalSlot::IsWithEnvironment);
            let not_with_env = w.negate(with_env);
            let r#true = w.make_bool(true);
            w.if_then_end(not_with_env, |w| {
                let completion = w.NormalCompletion(r#true);
                move |b| b.ret(Some(completion))
            });

            //# 5. Let unscopables be ? Get(bindingObject, @@unscopables).
            //# 6. If Type(unscopables) is Object, then
            //# a. Let blocked be ! ToBoolean(? Get(unscopables, N)).
            //# b. If blocked is true, return false.
            //# 7. Return true.

            let r#false = w.make_bool(false);
            let completion = w.NormalCompletion(r#false);

            w.finish(|w| w.ret(Some(completion)))
        };

        let get_binding_value = {
            let (f, [envRec, N, S]) = writer.start_function();
            let mut w = Emitter::new(writer, f);

            w.comment("ObjectEnvironmentRecord::GetBindingValue");

            //# 1. Let bindingObject be envRec.[[BindingObject]].
            let bindingObject = w.record_get_slot(envRec, InternalSlot::BindingObject);

            //# 2. Let value be ? HasProperty(bindingObject, N).
            let has_prop = w.record_get_slot(bindingObject, InternalSlot::HasProperty);
            let value = w.call_virt_with_result(has_prop, [bindingObject, N]);

            //# 3. If value is false, then
            let is_false = w.negate(value);
            w.if_then_end(is_false, |w| {
                //# a. If S is false, return the value undefined; otherwise throw a ReferenceError exception.
                let S_false = w.negate(S);
                let result = w.if_then_x_else_y(
                    S_false,
                    |w| w.make_undefined(),
                    |w| {
                        // panic
                        let null = w.make_null();
                        w.negate(null)
                    },
                );
                move |w| w.ret(Some(result))
            });

            //# 4. Return ? Get(bindingObject, N).
            let get = w.record_get_slot(bindingObject, InternalSlot::Get);
            let result = w.call_virt_with_result(get, [bindingObject, N, bindingObject]);

            w.finish(move |w| w.ret(Some(result)))
        };

        EnvironmentRecordVTable {
            has_binding,
            get_binding_value,
        }
    }

    fn init_vtable_function(
        _writer: &mut ProgramBuilder,
        declarative_environment_vtable: &EnvironmentRecordVTable,
    ) -> EnvironmentRecordVTable {
        // function declarative objects inherit most methods from the declarative environment

        EnvironmentRecordVTable {
            has_binding: declarative_environment_vtable.has_binding,
            get_binding_value: declarative_environment_vtable.get_binding_value,
        }
    }

    fn init_vtable_global(writer: &mut ProgramBuilder) -> GlobalEnvironmentRecordVTable {
        let has_binding = {
            let (f, [envRec, N]) = writer.start_function();
            let mut w = Emitter::new(writer, f);

            // <https://tc39.es/ecma262/#sec-global-environment-records-hasbinding-n>
            w.comment("GlobalEnvironmentRecord::HasBinding");

            let r#true = w.make_bool(true);

            //# 1. Let DclRec be envRec.[[DeclarativeRecord]].
            let DclRec = w.record_get_slot(envRec, InternalSlot::DeclarativeRecord);

            //# 2. If DclRec.HasBinding(N) is true, return true.
            let DclRec = EnvironmentRecord::new_with_register_unchecked(DclRec);
            let has_binding = DclRec.HasBinding(&mut w, N);
            w.if_then_end(has_binding, |_| |b| b.ret(Some(r#true)));

            // //# 3. Let ObjRec be envRec.[[ObjectRecord]].
            let ObjRec = w.record_get_slot(envRec, InternalSlot::ObjectRecord);

            // //# 4. Return ? ObjRec.HasBinding(N).
            let ObjRec = EnvironmentRecord::new_with_register_unchecked(ObjRec);
            let result = ObjRec.HasBinding(&mut w, N);

            w.finish(|w| w.ret(Some(result)))
        };

        let has_var_declaration = {
            let (mut f, [envRec, N]) = writer.start_function();
            let mut w = f.start_block_main();

            // <https://tc39.es/ecma262/#sec-hasvardeclaration>
            w.comment("GlobalEnvironmentRecord::HasVarDeclaration");

            //# 1. Let varDeclaredNames be envRec.[[VarNames]].
            let varDeclaredNames = w.record_get_slot(envRec, InternalSlot::VarNames);

            //# 2. If varDeclaredNames contains N, return true.
            //# 3. Return false.
            let result = w.record_has_prop(varDeclaredNames, N);
            f.end_block(w.ret(Some(result)));

            writer.end_function(f)
        };

        let has_lexical_declaration = {
            let (mut f, [envRec, N]) = writer.start_function();
            let mut w = f.start_block_main();

            // <https://tc39.es/ecma262/#sec-haslexicaldeclaration>
            w.comment("GlobalEnvironmentRecord::HasLexicalDeclaration");

            //# 1. Let DclRec be envRec.[[DeclarativeRecord]].
            let DclRec = w.record_get_slot(envRec, InternalSlot::DeclarativeRecord);

            //# 2. Return DclRec.HasBinding(N).
            let DclRec = EnvironmentRecord::new_with_register_unchecked(DclRec);
            let result = DclRec.HasBinding(&mut w, N);

            f.end_block(w.ret(Some(result)));
            writer.end_function(f)
        };

        let has_restricted_global_property = {
            let (mut f, [envRec, _N]) = writer.start_function();
            let mut w = f.start_block_main();

            // <https://tc39.es/ecma262/#sec-hasrestrictedglobalproperty>
            w.comment("GlobalEnvironmentRecord::HasRestrictedGlobalProperty");

            //# 1. Let ObjRec be envRec.[[ObjectRecord]].
            let ObjRec = w.record_get_slot(envRec, InternalSlot::ObjectRecord);

            //# 2. Let globalObject be ObjRec.[[BindingObject]].
            let _globalObject = w.record_get_slot(ObjRec, InternalSlot::BindingObject);

            //# 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
            //# 4. If existingProp is undefined, return false.
            //# 5. If existingProp.[[Configurable]] is true, return false.
            //# 6. Return true.
            let result = w.make_bool(false);

            f.end_block(w.ret(Some(result)));
            writer.end_function(f)
        };

        let create_global_function_binding = {
            let (f, [envRec, N, V, D]) = writer.start_function();
            let mut b = Emitter::new(writer, f);

            // <https://tc39.es/ecma262/#sec-createglobalfunctionbinding>
            b.comment("GlobalEnvironmentRecord::CreateGlobalFunctionBinding");

            //# 1. Let ObjRec be envRec.[[ObjectRecord]].
            let ObjRec = b.record_get_slot(envRec, InternalSlot::ObjectRecord);

            //# 2. Let globalObject be ObjRec.[[BindingObject]].
            let globalObject = b.record_get_slot(ObjRec, InternalSlot::BindingObject);

            //# 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
            // TODO: use `?`-ness
            let get_own_prop = b.record_get_slot(globalObject, InternalSlot::GetOwnProperty);
            let existingProp = b.call_virt_with_result(get_own_prop, [globalObject, N]);

            //# 4. If existingProp is undefined or existingProp.[[Configurable]] is true, then
            let undef = b.make_undefined();
            let is_undef = b.compare_equal(existingProp, undef);

            let cond = b.if_then_x_else_y(
                is_undef,
                |_| is_undef,
                |b| {
                    let configurable = b.record_get_slot(existingProp, InternalSlot::Configurable);
                    let r#true = b.make_bool(true);
                    b.compare_equal(configurable, r#true)
                },
            );

            let desc = b.if_then_x_else_y(
                cond,
                |b| {
                    //# a. Let desc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: D }.
                    let desc = b.record_new();
                    let r#true = b.make_bool(true);
                    b.record_set_slot(desc, InternalSlot::Value, V);
                    b.record_set_slot(desc, InternalSlot::Writable, r#true);
                    b.record_set_slot(desc, InternalSlot::Enumerable, r#true);
                    b.record_set_slot(desc, InternalSlot::Configurable, D);
                    desc
                },
                //# 5. Else,
                |b| {
                    //# a. Let desc be the PropertyDescriptor { [[Value]]: V }.
                    let desc = b.record_new();
                    b.record_set_slot(desc, InternalSlot::Value, V);
                    desc
                },
            );

            //# 6. Perform ? DefinePropertyOrThrow(globalObject, N, desc).
            b.Q(|b| b.DefinePropertyOrThrow(globalObject, N, desc));

            //# 7. Perform ? Set(globalObject, N, V, false).
            // we can ignore the error here (TODO: we should be fallible?)
            b.Set(globalObject, N, V, false);

            //# 8. Let varDeclaredNames be envRec.[[VarNames]].
            //# 9. If varDeclaredNames does not contain N, then
            //# a. Append N to varDeclaredNames.
            //# 10. Return NormalCompletion(empty).
            let argument = b.make_undefined();
            let completion = EmitterExt::NormalCompletion(&mut b, argument);

            b.finish(|b| b.ret(Some(completion)))
        };

        let get_binding_value = {
            let (f, [envRec, N, S]) = writer.start_function();
            let mut b = Emitter::new(writer, f);

            // <https://tc39.es/ecma262/#sec-global-environment-records-getbindingvalue-n-s>
            b.comment("GlobalEnvironmentRecord::GetBindingValue");

            //# 1. Let DclRec be envRec.[[DeclarativeRecord]].
            let DclRec = b.record_get_slot(envRec, InternalSlot::DeclarativeRecord);
            let DclRec = EnvironmentRecord::new_with_register_unchecked(DclRec);

            //# 2. If DclRec.HasBinding(N) is true, then
            let has_binding = DclRec.HasBinding(&mut b.block, N);
            b.if_then_end(has_binding, |b| {
                //# a. Return DclRec.GetBindingValue(N, S).
                let result = DclRec.GetBindingValue(&mut b.block, N, S);
                move |b| b.ret(Some(result))
            });

            //# 3. Let ObjRec be envRec.[[ObjectRecord]].
            let ObjRec = b.record_get_slot(envRec, InternalSlot::ObjectRecord);
            let ObjRec = EnvironmentRecord::new_with_register_unchecked(ObjRec);

            //# 4. Return ? ObjRec.GetBindingValue(N, S).
            let result = ObjRec.GetBindingValue(&mut b.block, N, S);
            b.finish(|b| b.ret(Some(result)))
        };

        GlobalEnvironmentRecordVTable {
            normal: EnvironmentRecordVTable {
                has_binding,
                get_binding_value,
            },
            has_var_declaration,
            has_lexical_declaration,
            has_restricted_global_property,
            create_global_function_binding,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, RefCast)]
#[repr(transparent)]
pub struct EnvironmentRecord {
    pub register: RegisterId,
}

#[allow(non_snake_case)]
impl EnvironmentRecord {
    /// This assumes that you have created an [`EnvironmentRecord`] with the
    /// [`EnvironmentRecordFactory`] before-hand. If you have not done so, this
    /// API will not behave correctly. Use at your own disgression.
    pub fn new_with_register_unchecked(record: RegisterId) -> Self {
        Self { register: record }
    }

    pub fn HasBinding(&self, block: &mut DynBlockBuilder, N: RegisterId) -> RegisterId {
        block.comment("HasBinding");

        let func = block.record_get_slot(self.register, InternalSlot::JSSATHasBinding);
        block.call_virt_with_result(func, [self.register, N])
    }

    pub fn GetBindingValue(
        &self,
        block: &mut DynBlockBuilder,
        N: RegisterId,
        S: RegisterId,
    ) -> RegisterId {
        block.comment("GetBindingValue");

        let func = block.record_get_slot(self.register, InternalSlot::JSSATGetBindingValue);
        block.call_virt_with_result(func, [self.register, N, S])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlobalEnvironmentRecord {
    pub register: RegisterId,
}

impl Deref for GlobalEnvironmentRecord {
    type Target = EnvironmentRecord;

    fn deref(&self) -> &Self::Target {
        EnvironmentRecord::ref_cast(&self.register)
    }
}

#[allow(non_snake_case)]
impl GlobalEnvironmentRecord {
    /// This assumes that you have created an [`EnvironmentRecord`] with the
    /// [`EnvironmentRecordFactory`] before-hand. If you have not done so, this
    /// API will not behave correctly. Use at your own disgression.
    pub fn new_with_register_unchecked(record: RegisterId) -> Self {
        Self { register: record }
    }

    pub fn HasVarDeclaration(&self, block: &mut DynBlockBuilder, N: RegisterId) -> RegisterId {
        block.comment("HasVarDeclaration");

        let fn_ptr = block.record_get_slot(self.register, InternalSlot::JSSATHasVarDeclaration);
        block.call_virt_with_result(fn_ptr, [self.register, N])
    }

    pub fn HasLexicalDeclaration(&self, block: &mut DynBlockBuilder, N: RegisterId) -> RegisterId {
        block.comment("HasLexicalDeclaration");

        let fn_ptr = block.record_get_slot(self.register, InternalSlot::JSSATHasLexicalDeclaration);
        block.call_virt_with_result(fn_ptr, [self.register, N])
    }

    pub fn HasRestrictedGlobalProperty(
        &self,
        block: &mut DynBlockBuilder,
        N: RegisterId,
    ) -> RegisterId {
        block.comment("HasRestrictedGlobalProperty");

        let fn_ptr = block.record_get_slot(
            self.register,
            InternalSlot::JSSATHasRestrictedGlobalProperty,
        );
        block.call_virt_with_result(fn_ptr, [self.register, N])
    }

    pub fn CreateGlobalFunctionBinding(
        &self,
        block: &mut DynBlockBuilder,
        N: RegisterId,
        V: RegisterId,
        D: RegisterId,
    ) -> RegisterId {
        block.comment("CreateGlobalFunctionBinding");

        let fn_ptr = block.record_get_slot(
            self.register,
            InternalSlot::JSSATCreateGlobalFunctionBinding,
        );
        block.call_virt_with_result(fn_ptr, [self.register, N, V, D])
    }
}

/// Has fields for every method specified in
/// <https://tc39.es/ecma262/#table-abstract-methods-of-environment-records>.
///
/// Every function has one parameter for the environment record itself to be
/// passed in, and the remaining parameters are dedicated to what's specified
/// in the specification.
struct EnvironmentRecordVTable {
    has_binding: FnSignature<2>,
    // create_mutable_binding: FnSignature<3>,
    // create_immutable_binding: FnSignature<3>,
    // initialize_binding: FnSignature<3>,
    // set_mutable_binding: FnSignature<4>,
    get_binding_value: FnSignature<3>,
    // delete_binding: FnSignature<2>,
    // has_this_binding: FnSignature<1>,
    // has_super_binding: FnSignature<1>,
    // with_base_object: FnSignature<1>,
}

/// Has fields for every method specified in
/// <https://tc39.es/ecma262/#table-additional-methods-of-global-environment-records>.
///
/// Every function has one parameter for the environment record itself to be
/// passed in, and the remaining parameters are dedicated to what's specified
/// in the specification.
struct GlobalEnvironmentRecordVTable {
    normal: EnvironmentRecordVTable,
    has_var_declaration: FnSignature<2>,
    has_lexical_declaration: FnSignature<2>,
    has_restricted_global_property: FnSignature<2>,
    create_global_function_binding: FnSignature<4>,
}
