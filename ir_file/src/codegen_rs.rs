//! Generates Rust code to use an AST from Rust code.
//!
//! # Example
//!
//! The following lisp code
//!
//! ```lisp
//! ((:1.0 CallX (value))
//!  ((call X :value)))
//! ((:1.1 X (value))
//!  ((return :value)))
//! ```
//!
//! would generate the following Rust code:
//!
//! ```norun
//! pub struct Methods {
//!     pub CallX: FnSignature<1>,
//!     pub X: FnSignature<1>,
//! }
//!
//! impl Methods {
//!     pub fn new(program: &mut ProgramBuilder) -> Self {
//!         let CallX = program.start_function();
//!         let X = program.start_function();
//!
//!         let signature_CallX = CallX.0.signature();
//!         let signature_X = X.0.signature();
//!
//!         let methods = Methods {
//!             CallX: signature_CallX,
//!             X: signature_X,
//!         };
//!
//!         methods.CallX(Emitter::new(program, CallX.0), CallX.1);
//!         methods.X(Emitter::new(program, X.0), X.1);
//!
//!         methods
//!     }
//!
//!     fn CallX(&self, e: Emitter<1>, [value]: [RegisterId; 1]) -> FnSignature<1> {
//!         e.comment("1.0 CallX ( value )");
//!         let result = e.call(self.X);
//!         e.finish(Some(result))
//!     }
//!
//!     fn X(&self, e: Emitter<1>, [value]: [RegisterId; 1]) -> FnSignature<1> {
//!         e.comment("1.1 X ( value )");
//!         e.finish(Some(value))
//!     }
//! }
//! ```

use std::collections::HashMap;

use codegen::{Block, Field, Formatter, Function, Impl, Scope};
use rustc_hash::FxHashSet;

use crate::{Assign, Expression, Section, Statement, Visitor, AST};

pub fn gen(name: &str, mut ast: AST) -> String {
    let mut scope = Scope::new();

    let r#struct = scope.new_struct(name);
    r#struct.vis("pub");

    r#struct.push_field(Field::new("pub atoms", format!("{}Atoms", name)));

    for method in ast.sections.iter() {
        r#struct.push_field(Field::new(
            &format!("pub {}", method.header.method_name.replace(':', "_")),
            format!("FnSignature<{}>", method.header.parameters.len()),
        ));
    }

    let atoms = scope.new_struct(format!("{}Atoms", name).as_str());
    atoms.vis("pub");

    let mut visitor = AtomVisitor {
        atoms: FxHashSet::default(),
    };
    visitor.visit_ast(&mut ast);

    for atom in &visitor.atoms {
        atoms.field(&format!("pub {}", atom.as_str()), "Atom");
    }

    let atoms_impl = scope.new_impl(format!("{}Atoms", name).as_str());
    let new = atoms_impl
        .new_fn("new")
        .arg("dealer", "&mut AtomDealer")
        .ret("Self");

    new.line(format!("{}Atoms", name).as_str());

    let mut fields = Block::new("");
    for atom in &visitor.atoms {
        fields.line(format!("{}: dealer.deal({:?}),", atom, atom));
    }
    new.push_block(fields);

    let r#impl = scope.new_impl(name);

    for method in ast.sections.iter() {
        emit_method(r#impl, method);
    }

    let f = emit_method_new(name, &ast);
    r#impl.push_fn(f);

    format!(
        "#![allow(non_snake_case)]
#![allow(unused_variables)]

use jssat_ir::{{
    frontend::{{
        builder::{{FnSignature, RegisterId, ProgramBuilder}},
        emitter::{{ControlFlow, Emitter, LoopControlFlow}},
    }},
    isa::{{Atom, AtomDealer, ValueType}},
}};

{}",
        scope.to_string()
    )
}

fn emit_method_new(name: &str, ast: &AST) -> Function {
    let mut f = Function::new("new");
    f.vis("pub(crate)");
    f.arg("program", "&mut ProgramBuilder");
    f.ret("Self");

    for method in ast.sections.iter() {
        let fn_name = &method.header.method_name.replace(':', "_");

        f.line(format!(
            "let (mut {0}, {0}_args) = program.start_function();",
            fn_name
        ));

        f.line(format!(
            r#"{}.with_name("{}".to_string());"#,
            fn_name, method.header.method_name,
        ));

        f.line(format!("let signature_{0} = {0}.signature();", fn_name));
    }

    let mut fields = Block::new("");
    fields.line(format!("atoms: {}Atoms::new(&mut program.dealer),", name));

    for method in ast.sections.iter() {
        fields.line(format!(
            "{}: signature_{0},",
            &method.header.method_name.replace(':', "_")
        ));
    }

    f.line(format!("let methods = {} {};", name, blk_to_s(fields)));

    for method in ast.sections.iter() {
        f.line(format!(
            "methods.{}(Emitter::new(program, {0}), {0}_args);",
            &method.header.method_name.replace(':', "_")
        ));
    }

    f.line("methods");

    f
}

pub fn emit_method(scope: &mut Impl, section: &Section) {
    let mut f = Function::new(&section.header.method_name.replace(':', "_"));

    f.arg_ref_self();

    let params = &section.header.parameters;
    f.arg("mut e", format!("Emitter<{}>", params.len()));
    f.arg(
        &format!(
            "[{}]",
            params
                .iter()
                .map(|p| format!("r#var_{}", p))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!("[RegisterId; {}]", params.len()),
    );
    f.ret(format!("FnSignature<{}>", params.len()));

    // comment the method name
    let mut code = Block::new("");

    let idx = &section.header.document_index;
    let name = &section.header.method_name.replace(':', "_");
    code.line(format!(
        r#"e.comment("{} {} ( {} )");"#,
        idx,
        name,
        params.join(", ")
    ));

    let mut ret_expr = None;

    let stmts_to_emit = match section.body.last() {
        Some(Statement::Return { expr }) => {
            ret_expr = Some(expr);
            &section.body[..section.body.len() - 1]
        }
        _ => &section.body[..],
    };

    let mut counter = 0;

    emit_stmts(&mut counter, &mut code, stmts_to_emit, false, false);

    match ret_expr {
        Some(Some(expr)) => {
            let value = emit_expr(&mut counter, &mut code, expr);
            code.line(format!("e.finish(Some({}))", value));
        }
        Some(None) => {
            code.line("e.finish(None)");
        }
        None => panic!("expected return to finish off block"),
    }

    f.push_block(code);

    scope.push_fn(f);
}

fn emit_stmts(
    counter: &mut usize,
    block: &mut Block,
    stmts: &[Statement],
    emit_fallthrough: bool,
    emit_loop: bool,
) -> bool {
    let varname = |x: &str| format!("r#var_{}", x.replace("-", "_"));

    // emit statements
    let mut returned = false;
    for stmt in stmts.iter() {
        if returned {
            panic!(
                "already returned yet more instructions?, {:#?} at {:#?}",
                stmts, stmt
            );
        }

        match stmt {
            crate::Statement::Assign(x) => {
                let Assign { variable, value } = x;
                let value = emit_expr(counter, block, value);
                block.line(format!("let {} = {};", varname(variable), value));
            }
            crate::Statement::If {
                condition,
                then,
                r#else,
            } => {
                let mut cond_expr = Block::new("");
                let condition = emit_expr(counter, &mut cond_expr, condition);
                cond_expr.line(condition);

                let mut then_blk = Block::new("");
                emit_stmts(counter, &mut then_blk, then, true, false);

                match r#else {
                    None => {
                        block.line(format!(
                            "e.if_then(|e| {}, |e| {});",
                            blk_to_s(cond_expr),
                            blk_to_s(then_blk)
                        ));
                    }
                    Some(stmts) => {
                        let mut else_blk = Block::new("");
                        emit_stmts(counter, &mut else_blk, stmts, true, false);

                        block.line(format!(
                            "e.if_then(|e| {}, |e| {})",
                            blk_to_s(cond_expr),
                            blk_to_s(then_blk)
                        ));

                        block.line(format!(".else_then(|e| {});", blk_to_s(else_blk)));
                    }
                }
            }
            crate::Statement::RecordSetProp {
                record,
                prop,
                value,
            } => {
                let record = emit_expr(counter, block, record);
                let prop = emit_expr(counter, block, prop);

                match value.as_ref().map(|expr| emit_expr(counter, block, expr)) {
                    Some(value) => block.line(format!(
                        "e.record_set_prop({}, {}, {});",
                        record, prop, value
                    )),
                    None => block.line(format!("e.record_del_prop({}, {});", record, prop)),
                };
            }
            crate::Statement::RecordSetSlot {
                record,
                slot,
                value,
            } => {
                let record = emit_expr(counter, block, record);
                match value {
                    Some(value) => {
                        let value = emit_expr(counter, block, value);
                        block.line(format!(
                            "e.record_set_atom({}, self.atoms.{}, {});",
                            record, slot, value
                        ));
                    }
                    None => {
                        block.line(format!(
                            "e.record_del_atom({}, self.atoms.{});",
                            record, slot
                        ));
                    }
                }
            }
            crate::Statement::Return { expr } => {
                returned = true;

                let cf = match emit_loop {
                    true => "LoopControlFlow",
                    false => "ControlFlow",
                };

                // we are most likely being called from within a nested set of stmts
                // the most outer layer has already handled return stmts for us
                let line = match expr {
                    Some(expr) => {
                        format!("{}::Return(Some({}))", cf, emit_expr(counter, block, expr))
                    }
                    None => format!("{}::Return(None)", cf),
                };

                block.line(line);
            }
            crate::Statement::CallStatic {
                function_name,
                args,
            } => {
                let args = args
                    .iter()
                    .map(|e| emit_expr(counter, block, e))
                    .collect::<Vec<_>>()
                    .join(", ");

                block.line(format!(
                    "e.call(self.{}, [{}]);",
                    function_name.replace(':', "_"),
                    args
                ));
            }
            crate::Statement::CallVirt { fn_ptr, args } => {
                let fn_ptr = emit_expr(counter, block, fn_ptr);
                let args = args
                    .iter()
                    .map(|e| emit_expr(counter, block, e))
                    .collect::<Vec<_>>()
                    .join(", ");

                block.line(format!("e.call_virt_dynargs({}, vec![{}]);", fn_ptr, args));
            }
            crate::Statement::Assert { expr, message } => {
                let assertion = emit_expr(counter, block, expr);
                block.line(format!("e.assert({}, {:?});", assertion, message));
            }
            crate::Statement::ListSet { list, prop, value } => {
                let list = emit_expr(counter, block, list);
                let prop = emit_expr(counter, block, prop);

                match value.as_ref().map(|expr| emit_expr(counter, block, expr)) {
                    Some(value) => {
                        block.line(format!("e.list_set({}, {}, {});", list, prop, value))
                    }
                    None => block.line(format!("e.list_del({}, {});", list, prop)),
                };
            }
            Statement::Loop {
                init,
                cond,
                body,
                next,
            } => {
                let mut vars = HashMap::new();
                for i in init {
                    vars.insert(i.variable.clone(), &i.value);
                }

                let mut vars2 = HashMap::new();
                for i in next {
                    let init = *vars
                        .get(&i.variable)
                        .expect("should be same assignments in next as init");
                    vars2.insert(i.variable.clone(), (init, &i.value));
                }

                // ensure we have same num of inits and nexts
                assert_eq!(vars.len(), vars2.len());

                let mut vars = vars2
                    .into_iter()
                    .map(|(name, (init, next))| (name, init, next))
                    .collect::<Vec<_>>();

                // UGLY CODE BEGIN
                // we are going to sort `vars` based on the order of `init`

                let mut new_vars = Vec::new();

                for i in init {
                    let mut tidx = None;
                    for (idx, j) in vars.iter().enumerate() {
                        if i.variable == j.0 {
                            tidx = Some(idx);
                            break;
                        }
                    }

                    let v = vars.remove(tidx.expect("expected to find var"));
                    new_vars.push(v);
                }

                let vars = new_vars;

                // UGLY CODE END

                let init_exprs = vars
                    .iter()
                    .map(|(_, init, _)| {
                        let mut block = Block::new("");
                        let name = emit_expr(counter, &mut block, *init);
                        block.line(name);
                        blk_to_s(block)
                    })
                    .map(|s| format!("Box::new(move |e| {})", s))
                    .collect::<Vec<_>>()
                    .join(", ");

                let names = vars
                    .iter()
                    .map(|(name, _, _)| varname(name.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ");

                let cond_expr = {
                    let mut block = Block::new("");
                    let expr_name = emit_expr(counter, &mut block, cond);
                    block.line(expr_name);
                    blk_to_s(block)
                };

                let body_stmts = {
                    let mut block = Block::new("");
                    let exited = emit_stmts(counter, &mut block, body, false, true);

                    if !exited {
                        let names = vars
                            .iter()
                            .map(|(_, _, next)| emit_expr(counter, &mut block, *next))
                            .collect::<Vec<_>>()
                            .join(", ");

                        block.line(format!("LoopControlFlow::Next([{}])", names));
                    }

                    blk_to_s(block)
                };

                block.line(format!(
                    "e.do_loop([{}], |e, [{}]| {}, |e, [{}]| {});",
                    init_exprs, names, cond_expr, names, body_stmts,
                ));

                assert_eq!(init.len(), next.len());
            }
        }
    }

    if !returned && emit_fallthrough {
        block.line("ControlFlow::Fallthrough");
    }

    returned
}

/// Emits an expression to the block, and a string identifier used to refer to
/// the result of the computation.
fn emit_expr(counter: &mut usize, block: &mut Block, expr: &Expression) -> String {
    let name = |c: &mut usize| {
        format!("tmp{}", {
            *c += 1;
            c
        })
    };

    let varname = |x: &str| format!("r#var_{}", x.replace("-", "_"));

    let result = name(counter);

    match expr {
        Expression::GetGlobal => {
            panic!("get global instructions should automatically be replaced by threaded state");
        }
        Expression::If {
            condition,
            then: (then, thene),
            r#else: (els, els_e),
        } => {
            let mut cond_scope = Block::new("");
            let condition = emit_expr(counter, &mut cond_scope, condition);
            cond_scope.line(condition);

            let mut then_scope = Block::new("");
            let retd = emit_stmts(counter, &mut then_scope, then, false, false);
            if !retd {
                let then_expr = emit_expr(counter, &mut then_scope, thene);
                then_scope.line(format!("ControlFlow::Carry({})", then_expr));
            }

            let mut else_scope = Block::new("");
            let retd = emit_stmts(counter, &mut else_scope, els, false, false);
            if !retd {
                let else_expr = emit_expr(counter, &mut else_scope, els_e);
                else_scope.line(format!("ControlFlow::Carry({})", else_expr));
            }

            block.line(format!(
                "let {} = e.if_then(|e| {}, |e| {}).else_then(|e| {}).end().unwrap();",
                result,
                blk_to_s(cond_scope),
                blk_to_s(then_scope),
                blk_to_s(else_scope)
            ));
        }
        Expression::VarReference { variable } => {
            *counter -= 1;
            return varname(variable);
        }
        Expression::LetIn {
            variable,
            be_bound_to,
            r#in: (stmts, expr),
        } => {
            *counter -= 1;

            let value = emit_expr(counter, block, be_bound_to);
            block.line(format!("let {} = {};", varname(variable), value));

            emit_stmts(counter, block, stmts, false, false);

            return emit_expr(counter, block, expr);
        }
        Expression::RecordNew => {
            block.line(format!("let {} = e.record_new();", result));
        }
        Expression::Unreachable => {
            block.line(format!("let {} = e.unreachable();", result));
        }
        Expression::RecordGetProp { record, property } => {
            let record = emit_expr(counter, block, record);
            let property = emit_expr(counter, block, property);
            block.line(format!(
                "let {} = e.record_get_prop({}, {});",
                result, record, property
            ));
        }
        Expression::RecordGetSlot { record, slot } => {
            let record = emit_expr(counter, block, record);
            block.line(format!(
                "let {} = e.record_get_atom({}, self.atoms.{});",
                result, record, slot
            ));
        }
        Expression::RecordHasProp { record, property } => {
            let record = emit_expr(counter, block, record);
            let property = emit_expr(counter, block, property);
            block.line(format!(
                "let {} = e.record_has_prop({}, {});",
                result, record, property
            ));
        }
        Expression::RecordHasSlot { record, slot } => {
            let expr = emit_expr(counter, block, record);
            block.line(format!(
                "let {} = e.record_has_atom({}, self.atoms.{});",
                result, expr, slot
            ));
        }
        Expression::GetFnPtr { function_name } => {
            block.line(format!(
                "let {} = e.make_fnptr(self.{}.id);",
                result,
                function_name.replace(':', "_")
            ));
        }
        Expression::CallStatic {
            function_name,
            args,
        } => {
            let args = args
                .iter()
                .map(|e| emit_expr(counter, block, e))
                .collect::<Vec<_>>()
                .join(", ");

            block.line(format!(
                "let {} = e.call_with_result(self.{}, [{}]);",
                result,
                function_name.replace(':', "_"),
                args
            ));
        }
        Expression::CallVirt { fn_ptr, args } => {
            let fn_ptr = emit_expr(counter, block, fn_ptr);

            let args = args
                .iter()
                .map(|e| emit_expr(counter, block, e))
                .collect::<Vec<_>>()
                .join(", ");

            block.line(format!(
                "let {} = e.call_virt_dynargs_with_result({}, vec![{}]);",
                result, fn_ptr, args
            ));
        }
        Expression::MakeAtom { atom } => {
            block.line(format!(
                "let {} = e.make_atom(self.atoms.{});",
                result, atom
            ));
        }
        Expression::MakeBytes { bytes } => {
            block.line(format!(
                "let {} = e.load_constant((&{:?}).to_vec());",
                result,
                bytes.as_slice()
            ));
        }
        Expression::MakeInteger { value } => {
            block.line(format!(
                "let {} = e.make_number_decimal({});",
                result, value
            ));
        }
        Expression::MakeBoolean { value } => {
            block.line(format!("let {} = e.make_bool({});", result, value));
        }
        Expression::BinOp { kind, lhs, rhs } => {
            let lhs = emit_expr(counter, block, lhs);
            let rhs = emit_expr(counter, block, rhs);

            match kind {
                crate::BinOpKind::Add => {
                    block.line(format!("let {} = e.add({}, {});", result, lhs, rhs));
                }
                crate::BinOpKind::And => {
                    block.line(format!("let {} = e.and({}, {});", result, lhs, rhs));
                }
                crate::BinOpKind::Or => {
                    block.line(format!("let {} = e.or({}, {});", result, lhs, rhs));
                }
                crate::BinOpKind::Eq => {
                    block.line(format!(
                        "let {} = e.compare_equal({}, {});",
                        result, lhs, rhs
                    ));
                }
                crate::BinOpKind::Lt => {
                    block.line(format!(
                        "let {} = e.compare_less_than({}, {});",
                        result, lhs, rhs
                    ));
                }
            };
        }
        Expression::Negate { expr } => {
            let expr = emit_expr(counter, block, expr);
            block.line(format!("let {} = e.negate({});", result, expr));
        }
        Expression::IsTypeOf { expr, kind } => {
            let expr = emit_expr(counter, block, expr);
            block.line(format!(
                "let {} = e.is_type_of({}, ValueType::{});",
                result, expr, kind
            ));
        }
        Expression::IsTypeAs { lhs, rhs } => {
            let lhs = emit_expr(counter, block, lhs);
            let rhs = emit_expr(counter, block, rhs);
            block.line(format!("let {} = e.is_type_as({}, {});", result, lhs, rhs));
        }
        Expression::ListNew => {
            block.line(format!("let {} = e.list_new();", result));
        }
        Expression::ListGet { list, property } => {
            let record = emit_expr(counter, block, list);
            let property = emit_expr(counter, block, property);
            block.line(format!(
                "let {} = e.list_get({}, {});",
                result, record, property
            ));
        }
        Expression::ListHas { list, property } => {
            let record = emit_expr(counter, block, list);
            let property = emit_expr(counter, block, property);
            block.line(format!(
                "let {} = e.list_has({}, {});",
                result, record, property
            ));
        }
        Expression::ListLen { list } => {
            let list = emit_expr(counter, block, list);
            block.line(format!("let {} = e.list_len({});", result, list));
        }
    };
    result
}

fn blk_to_s(b: Block) -> String {
    let mut s = String::new();
    let mut f = Formatter::new(&mut s);
    b.fmt(&mut f).unwrap();
    s
}

struct AtomVisitor {
    atoms: FxHashSet<String>,
}

impl Visitor for AtomVisitor {
    fn visit_expr(&mut self, expr: &mut Expression) {
        if let Expression::MakeAtom { atom } = expr {
            self.atoms.insert(atom.clone());
        }

        self.visit_expr_impl(expr);
    }

    fn visit_slot(&mut self, slot: &mut String) {
        self.atoms.insert(slot.clone());
    }
}
