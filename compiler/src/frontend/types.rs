use crate::id::*;
use rustc_hash::FxHashMap;

use super::type_annotater::ValueType;

#[derive(Debug, Clone, Default)]
pub struct RecordShape {
    map: FxHashMap<ShapeKey, ValueType>,
}

impl RecordShape {
    pub fn add_prop(&self, key: ShapeKey, value: ValueType) -> RecordShape {
        let mut key_value_map = self.map.clone();
        key_value_map.insert(key, value);
        RecordShape { map: key_value_map }
    }

    pub fn type_at_key<'me>(&'me self, key: &ShapeKey) -> &'me ValueType {
        self.map.get(key).unwrap()
    }

    pub fn union<C: ContextTag>(&self, other: &RecordShape, reg_map: &RegMap<C>) -> RecordShape {
        let mut map = self.map.clone();
        let mut unmerged = vec![];

        // both maps should contain the same properties
        for (k, v) in other.map.iter() {
            match map.get_mut(k) {
                Some(_) => {
                    unmerged.push((k, v));
                }
                None => {
                    map.insert(k.clone(), v.clone());
                }
            };
        }

        // now we're left with conflicts
        for (k, v) in unmerged {
            let v_dest = map.get(k).unwrap();

            if v == v_dest {
                // there is no conflict, we're good
            } else {
                todo!("cannot unify different props yet")
            }
        }

        RecordShape { map }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShapeKey {
    String,
    Str(Vec<u8>),
    InternalSlot(&'static str),
}

impl ShapeKey {
    pub fn is_const(&self) -> bool {
        matches!(self, ShapeKey::Str(_))
    }
}

#[derive(Debug, Clone)]
pub struct RegMap<C> {
    registers: FxHashMap<RegisterId<C>, ValueType>,
    allocation_id_gen: AllocationId<NoContext>,
    allocations: FxHashMap<AllocationId<NoContext>, Vec<ShapeId<C>>>,
    shape_id_gen: ShapeId<C>,
    shapes: FxHashMap<ShapeId<C>, RecordShape>,
}

impl<C: ContextTag> RegMap<C> {
    pub fn insert(&mut self, register: RegisterId<C>, typ: ValueType) {
        if let ValueType::Record(alloc) = &typ {
            debug_assert!(self.allocations.contains_key(&alloc.map_context()));
        }

        self.registers.insert(register, typ);
    }

    pub fn insert_alloc(&mut self) -> AllocationId<NoContext> {
        let shape = self.insert_shape(RecordShape::default());

        let alloc_id = self.allocation_id_gen.next_and_mut();
        self.allocations.insert(alloc_id, vec![shape]);
        alloc_id
    }

    pub fn insert_shape(&mut self, shape: RecordShape) -> ShapeId<C> {
        let shape_id = self.shape_id_gen.next_and_mut();
        self.shapes.insert(shape_id, shape);
        shape_id
    }

    pub fn assign_new_shape(&mut self, allocation: AllocationId<NoContext>, shape: ShapeId<C>) {
        self.allocations.get_mut(&allocation).unwrap().push(shape);
    }

    pub fn extend(&mut self, other: RegMap<C>) {
        self.registers.extend(other.registers);

        for (k, v) in other.allocations {
            self.allocations
                .entry(k)
                .or_insert_with(Default::default)
                .extend(v);
        }

        self.shapes.extend(other.shapes);
    }

    pub fn get(&self, register: RegisterId<C>) -> &ValueType {
        self.registers.get(&register).unwrap()
    }

    pub fn get_shape(&self, allocation: AllocationId<NoContext>) -> &RecordShape {
        let shapes = self.allocations.get(&allocation).unwrap();
        self.get_shape_by_id(shapes.last().unwrap())
    }

    pub fn get_shape_by_id(&self, shape_id: &ShapeId<C>) -> &RecordShape {
        self.shapes.get(shape_id).unwrap()
    }

    pub fn allocations(
        &self,
    ) -> impl Iterator<Item = (&AllocationId<NoContext>, &Vec<ShapeId<C>>)> {
        self.allocations.iter()
    }

    pub fn is_const(&self, register: RegisterId<C>) -> bool {
        self.is_const_typ(self.get(register))
    }

    pub fn is_const_typ(&self, typ: &ValueType) -> bool {
        match typ {
            ValueType::Any
            | ValueType::Runtime
            | ValueType::String
            | ValueType::Number
            | ValueType::Pointer(_)
            | ValueType::Word
            | ValueType::Boolean => false,
            ValueType::ExactInteger(_) | ValueType::ExactString(_) | ValueType::Bool(_) => true,
            &ValueType::Record(allocation) => self.is_const_shape(allocation.map_context()),
        }
    }

    pub fn is_const_shape(&self, allocation: AllocationId<NoContext>) -> bool {
        let shape = self.get_shape(allocation);
        for (k, v) in shape.map.iter() {
            if k.is_const() && self.is_const_typ(v) {
                continue;
            } else {
                return false;
            }
        }
        true
    }

    /// States whether the type is "simple" or not. Simple types are extremely
    /// cheap to rebuild at any moment in the IR, and are considered cheaper to
    /// build than to pass around. Passing them around is considered "expensive"
    /// because then we're using more registers than necessary, which cause
    /// performance deficits because the more registers we use the more likely
    /// we'll need to spill onto the stack to generate a function.
    pub fn is_simple(&self, register: RegisterId<C>) -> bool {
        match self.get(register) {
            ValueType::Runtime
            | ValueType::ExactInteger(_)
            | ValueType::Bool(_)
            | ValueType::ExactString(_) => true,
            ValueType::Record(_) => todo!(),
            _ => false,
        }
    }
}

impl<C: ContextTag> Default for RegMap<C> {
    fn default() -> Self {
        Self {
            registers: Default::default(),
            allocation_id_gen: Default::default(),
            allocations: Default::default(),
            shape_id_gen: Default::default(),
            shapes: Default::default(),
        }
    }
}
