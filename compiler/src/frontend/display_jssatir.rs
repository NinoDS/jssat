use crate::frontend::ir::{BasicBlockJump, ControlFlowInstruction, Instruction};

use super::ir::{FFIValueType, IR};
use std::fmt::Write;

/// Infallible write
macro_rules! iw {
    ($($e:tt)+) => {
        write!($($e)+).unwrap()
    };
}
/// Infallible writeln
macro_rules! iwl {
    ($($e:tt)+) => {
        writeln!($($e)+).unwrap()
    };
}

pub fn display(program: &IR) -> String {
    let mut text = String::new();

    for (id, ext_fn) in program.external_functions.iter() {
        iw!(text, "ext fn @@{}(", id);

        for arg in ext_fn.parameters.iter() {
            iw!(text, "{}, ", display_norecord(arg));
        }

        iw!(text, ");\n\n");
    }

    for (id, f) in program.functions.iter() {
        iw!(text, "fn @{}(", id);

        for p in f.parameters.iter() {
            iw!(text, "%{}, ", p.register);
        }

        iw!(text, ") {{\n");

        let blocks = std::iter::once((&f.entry_block, f.blocks.get(&f.entry_block).unwrap()))
            .chain(f.blocks.iter().filter(|(id, _)| **id != f.entry_block));

        for (id, block) in blocks {
            iw!(text, "  ${}(", id);
            for arg in block.parameters.iter() {
                iw!(text, "%{}, ", arg);
            }
            iw!(text, "):\n");

            for inst in block.instructions.iter() {
                match inst {
                    Instruction::RecordNew(r) => {
                        iwl!(text, "    %{} = RecordNew", r.result(),)
                    }
                    Instruction::RecordGet {
                        result,
                        record,
                        key,
                    } => {
                        iwl!(text, "    %{} = RecordGet %{}, {}", result, record, key,)
                    }
                    Instruction::RecordSet { record, key, value } => {
                        iwl!(text, "    RecordSet %{}, {}, %{}", record, key, value,)
                    }
                    Instruction::Call(None, c, a) => {
                        iwl!(text, "    Call {:?}({:?})", c, a)
                    }
                    Instruction::Call(Some(r), c, a) => {
                        iwl!(text, "    %{} = Call {:?}({:?})", r, c, a)
                    }
                    Instruction::GetRuntime(rt) => iwl!(text, "    %{} = GetRuntime", rt),
                    Instruction::MakeString(r, s) => {
                        iwl!(
                            text,
                            "    %{} = MakeString {}",
                            r,
                            display_str(&program.constants.get(s).unwrap().payload)
                        );
                    }
                    Instruction::MakeInteger(r, i) => {
                        iwl!(text, "    %{} = MakeNumber {}", r, i);
                    }
                    // Instruction::M(r, v) => iwl!(
                    //     text,
                    //     "    %{} = MakeBoolean {}",
                    //     r,
                    //     display_vt(f.register_types.get(*r), f),
                    //     v
                    // ),
                    Instruction::MakeNull(r) => iwl!(text, "    %{} = MakeNull", r),
                    Instruction::MakeUndefined(r) => iwl!(text, "    %{} = MakeUndefined", r),
                    Instruction::ReferenceOfFunction(r, f) => {
                        iwl!(text, "    %{} = MakeFnPtr @{}", r, f)
                    }
                    Instruction::CompareLessThan(r, l, rr) => {
                        iwl!(text, "    %{} = CompareLessThan %{}, %{}", r, l, rr)
                    }
                    Instruction::CompareEqual(r, l, rr) => {
                        iwl!(text, "    %{} = CompareEqual %{}, %{}", r, l, rr)
                    }
                    Instruction::Negate(r, i) => iwl!(text, "    %{} = Negate %{}", r, i),
                    Instruction::Add(re, l, r) => iwl!(text, "    %{} = Add %{}, %{}", re, l, r),
                }
            }

            match &block.end {
                ControlFlowInstruction::Jmp(BasicBlockJump(id, args)) => {
                    iwl!(text, "    Jump ${}({:?})", id, args)
                }
                ControlFlowInstruction::JmpIf {
                    condition,
                    true_path,
                    false_path,
                } => iwl!(
                    text,
                    "    if %{}:\n        Jump ${}({:?})\n    else\n        Jump ${}({:?})",
                    condition,
                    true_path.0,
                    true_path.1,
                    false_path.0,
                    false_path.1,
                ),
                ControlFlowInstruction::Ret(r) => {
                    iwl!(text, "    Ret {:?}", r)
                }
            };
        }

        iw!(text, "}}\n\n");
    }

    text
}

fn display_norecord(t: &FFIValueType) -> String {
    match t {
        // ValueType::Bool(b) => format!("{}", b),
        // ValueType::ExactInteger(i) => format!("{}", i),
        // ValueType::ExactString(payload) => display_str(payload),
        // ValueType::Record(_) => unreachable!(),
        // ValueType::FnPtr(_) => "todo::FnPtr".into(),
        _ => format!("{:?}", t),
    }
}

fn display_str(payload: &[u8]) -> String {
    if let Ok(s) = String::from_utf8(payload.to_vec()) {
        return format!("{:?}", s);
    };

    let (pre, bytes, post) = unsafe { payload.align_to() };

    if !pre.is_empty() && !post.is_empty() {
        return format!("{:?}", payload);
    }

    if let Ok(s) = String::from_utf16(bytes) {
        return s;
    };

    return format!("{:?}", payload);
}