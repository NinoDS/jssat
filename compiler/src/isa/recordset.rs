use std::fmt::Write;
use tinyvec::{tiny_vec, TinyVec};

use super::{ISAInstruction, RecordKey};
use crate::{id::*, retag::RegRetagger};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RecordSet<C: Tag, S = ()> {
    pub shape: S,
    pub record: RegisterId<C>,
    pub key: RecordKey<C>,
    pub value: RegisterId<C>,
}

impl<C: Tag, S> ISAInstruction<C> for RecordSet<C, S> {
    fn declared_register(&self) -> Option<RegisterId<C>> {
        None
    }

    fn used_registers(&self) -> TinyVec<[RegisterId<C>; 3]> {
        let mut used_registers = tiny_vec![self.record, self.value];
        if let RecordKey::Prop(register) = self.key {
            used_registers.push(register);
        }
        used_registers
    }

    fn used_registers_mut(&mut self) -> Vec<&mut RegisterId<C>> {
        let mut used_registers = vec![&mut self.record, &mut self.value];
        if let RecordKey::Prop(register) = &mut self.key {
            used_registers.push(register);
        }
        used_registers
    }

    fn display(&self, w: &mut impl Write) -> std::fmt::Result {
        write!(
            w,
            "RecordSet %{}.{} = %{}",
            self.record, self.key, self.value
        )
    }
}

impl<C: Tag, S> RecordSet<C, S> {
    #[track_caller]
    pub fn retag<C2: Tag>(self, retagger: &mut impl RegRetagger<C, C2>) -> RecordSet<C2, S> {
        RecordSet {
            shape: self.shape,
            record: retagger.retag_old(self.record),
            key: self.key.retag(retagger),
            value: retagger.retag_old(self.value),
        }
    }
}
