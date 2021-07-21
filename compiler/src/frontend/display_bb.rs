use crate::frontend::ir::{ControlFlowInstruction, Instruction};

use super::conv_only_bb::PureBlocks;
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

pub fn display(program: &PureBlocks) -> String {
    let mut text = String::new();

    for (id, block) in program.blocks.iter() {
        iw!(text, "fn @{}(", id);

        for p in block.parameters.iter() {
            iw!(text, "%{}, ", p);
        }

        iw!(text, ") {{\n");

        for inst in block.instructions.iter() {
            match inst {
                Instruction::Comment(s, l) => iwl!(text, "    -- {}, {}", s, l),
                Instruction::NewRecord(r) => {
                    iwl!(text, "    %{} = RecordNew", r.result,)
                }
                Instruction::RecordGet(t) => iwl!(text, "    {:?}", t),
                Instruction::RecordSet(t) => iwl!(text, "    {:?}", t),
                Instruction::CallVirt(t) => iwl!(text, "    {:?}", t),
                Instruction::CallExtern(t) => iwl!(text, "    {:?}", t),
                Instruction::CallStatic(t) => iwl!(text, "    {:?}", t),
                Instruction::MakeTrivial(t) => iwl!(text, "    {:?}", t),
                Instruction::MakeBytes(inst) => iwl!(text, "    {:?}", inst),
                Instruction::GetFnPtr(inst) => iwl!(text, "todo"),
                Instruction::OpLessThan(inst) => {
                    let r = inst.result;
                    let l = inst.lhs;
                    let rr = inst.rhs;
                    iwl!(text, "    %{} = CompareLessThan %{}, %{}", r, l, rr)
                }
                Instruction::MakeInteger(inst) => iwl!(text, "    {:?}", inst),
                Instruction::OpEquals(inst) => iwl!(text, "    {:?}", inst),
                Instruction::OpNegate(inst) => iwl!(text, "    {:?}", inst),
                Instruction::OpAdd(inst) => iwl!(text, "    {:?}", inst),
            }
        }

        match &block.end {
            ControlFlowInstruction::Jmp(inst) => {
                iwl!(text, "    Jump ${}({:?})", inst.0 .0, inst)
            }
            ControlFlowInstruction::JmpIf(inst) => iwl!(text, "    {:?}", inst),
            ControlFlowInstruction::Ret(r) => {
                iwl!(text, "    Ret {:?}", r)
            }
        };

        iw!(text, "}}\n\n");
    }

    text
}
