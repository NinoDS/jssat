use derive_more::{Deref, DerefMut};
use jssat_ir::id::{Tag, UnionId};

use super::Type;

#[derive(Deref, DerefMut)]
pub struct Union<'ctx, T: Tag> {
    unique_id: UnionId<T>,
    #[deref]
    #[deref_mut]
    variants: Vec<Type<'ctx, T>>,
}

impl<'ctx, T: Tag> Union<'ctx, T> {
    pub fn new(unique_id: UnionId<T>) -> Self {
        Self {
            unique_id,
            variants: Vec::new(),
        }
    }

    pub fn unique_id(&self) -> UnionId<T> {
        self.unique_id
    }

    // pub fn push(&mut self, typ: Type<'ctx, T>) {
    //     self.variants.push(typ);
    // }

    // pub fn iter<'me>(&'me self) -> impl Iterator<Item = Type<'ctx, T>> + 'me {
    //     self.variants.iter().copied()
    // }
}

impl<'ctx, T: Tag> Eq for Union<'ctx, T> {}
impl<'ctx, T: Tag> PartialEq for Union<'ctx, T> {
    fn eq(&self, other: &Self) -> bool {
        self.unique_id == other.unique_id && self.variants == other.variants
    }
}
