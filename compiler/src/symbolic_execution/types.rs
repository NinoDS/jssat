use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::id::{Counter, LiftedCtx, SymbolicCtx};
use crate::isa::{InternalSlot, RecordKey, TrivialItem};
use crate::UnwrapNone;

type AllocationId = crate::id::AllocationId<LiftedCtx>;
type ConstantId = crate::id::ConstantId<SymbolicCtx>;
type RegisterId = crate::id::RegisterId<LiftedCtx>;
type ShapeId = crate::id::ShapeId<SymbolicCtx>;
/// The ID of a function whose argument types are not yet known.
type DynFnId = crate::id::FunctionId<LiftedCtx>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReturnType {
    Void,
    Value(RegisterType),
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegisterType {
    Any,
    Trivial(TrivialItem),
    Bytes,
    Byts(ConstantId),
    Number,
    Int(i64),
    Boolean,
    Bool(bool),
    FnPtr(DynFnId),
    Record(AllocationId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKey {
    Str(ConstantId),
    Slot(InternalSlot),
}

pub type ShapeValueType = RegisterType;

// pub enum ShapeValueType {
//     Any,
//     Trivial(TrivialItem),
//     Bytes,
//     Number,
//     Boolean,
//     FnPtr(DynFnId),
//     // NOTE: this might seem to make more sense to be `Record(ShapeId)`, but in
//     // practice this doesnt help.
//     Record(AllocationId),
// }

#[derive(Clone, Default)]
pub struct Shape {
    fields: FxHashMap<ShapeKey, ShapeValueType>,
}

impl Shape {
    pub fn new_with(&self, key: ShapeKey, value: ShapeValueType) -> Shape {
        let mut new = Shape {
            fields: self.fields.clone(),
        };
        new.fields.insert(key, value);
        new
    }

    pub fn get_typ(&self, key: ShapeKey) -> ShapeValueType {
        *self.fields.get(&key).unwrap()
    }
}

#[derive(Clone)]
pub struct TypeBag {
    registers: FxHashMap<RegisterId, RegisterType>,
    alloc_counter: Counter<AllocationId>,
    alloc_shapes: FxHashMap<AllocationId, Vec<ShapeId>>,
    shape_counter: Counter<ShapeId>,
    shapes: FxHashMap<ShapeId, Shape>,
    const_counter: Counter<ConstantId>,
    consts: FxHashMap<ConstantId, Vec<u8>>,
}

impl TypeBag {
    pub fn new_record(&mut self, register: RegisterId) {
        let alloc = self.alloc_counter.next();
        let shape = Shape::default();
        let shape_id = self.new_shape(shape);
        self.push_shape(alloc, shape_id);
        self.assign_type(register, RegisterType::Record(alloc));
    }

    pub fn append_shape(&mut self, register: RegisterId, shape: Shape) {
        let id = self.new_shape(shape);
        if let RegisterType::Record(a) = self.get(register) {
            self.push_shape(a, id);
        } else {
            panic!("attempted to get record shape on non-record");
        }
    }

    pub fn push_shape(&mut self, alloc: AllocationId, shape: ShapeId) {
        self.alloc_shapes
            .entry(alloc)
            .or_insert_with(Default::default)
            .push(shape);
    }

    fn alloc_allocation(&mut self) -> AllocationId {
        self.alloc_counter.next()
    }

    pub fn new_shape(&mut self, shape: Shape) -> ShapeId {
        // TODO: maybe intern the shape to prevent duplicates?
        // profiling would need to be done if that's worth it at all
        let id = self.shape_counter.next();
        self.shapes
            .insert(id, shape)
            .expect_none("should not have duplicate shapes");
        id
    }

    pub fn assign_type(&mut self, register: RegisterId, typ: RegisterType) {
        self.registers
            .insert(register, typ)
            .expect_none("should not have duplicate type for register");
    }

    pub fn intern_constant(&mut self, payload: Vec<u8>) -> ConstantId {
        let id = self.const_counter.next();
        self.consts.insert(id, payload);
        id
    }

    pub fn get(&self, register: RegisterId) -> RegisterType {
        *self.registers.get(&register).unwrap()
    }

    pub fn record_shape(&self, register: RegisterId) -> &Shape {
        if let RegisterType::Record(a) = self.get(register) {
            let shape_id = self.get_shape_id(a);
            self.get_shape(shape_id)
        } else {
            panic!("attempted to get record shape on non-record");
        }
    }

    pub fn get_fnptr(&self, register: RegisterId) -> DynFnId {
        if let RegisterType::FnPtr(f) = self.get(register) {
            f
        } else {
            panic!("attempted to get function pointer of non-fnptr");
        }
    }

    pub fn get_shape_id(&self, alloc: AllocationId) -> ShapeId {
        let shape_history = self.alloc_shapes.get(&alloc).unwrap();
        debug_assert!(!shape_history.is_empty());

        *shape_history.last().unwrap()
    }

    pub fn get_shape(&self, shape: ShapeId) -> &Shape {
        self.shapes.get(&shape).unwrap()
    }

    pub fn conv_key(&self, key: RecordKey<LiftedCtx>) -> ShapeKey {
        match key {
            RecordKey::Prop(r) => match self.get(r) {
                RegisterType::Byts(s) => ShapeKey::Str(s),
                RegisterType::Any
                | RegisterType::Trivial(_)
                | RegisterType::Bytes
                | RegisterType::Number
                | RegisterType::Int(_)
                | RegisterType::Boolean
                | RegisterType::Bool(_)
                | RegisterType::FnPtr(_)
                | RegisterType::Record(_) => panic!("unsupported key at this time"),
            },
            RecordKey::Slot(s) => ShapeKey::Slot(s),
        }
    }

    pub fn unintern_const(&self, id: ConstantId) -> &Vec<u8> {
        self.consts.get(&id).unwrap()
    }

    /// Given a set of registers, will pull out the types of those registers
    /// and create a new `TypeBag` containing only the types of the registers
    /// specified.
    pub fn extract(&self, regs: &[RegisterId]) -> Self {
        // TODO: coudl clean this up but EH!
        let mut new = TypeBag::default();

        let mut alloc_map = FxHashMap::default();
        let mut need_to_shape = VecDeque::new();

        for reg in regs {
            let typ = self.get(*reg);

            if let RegisterType::Record(a) = typ {
                need_to_shape.push_back(a);

                alloc_map.insert(a, new.alloc_allocation());
            } else {
                new.assign_type(*reg, typ);
            }
        }

        let mut shape_map = FxHashMap::default();

        while let Some(alloc_id) = need_to_shape.pop_front() {
            let shape_id = self.get_shape_id(alloc_id);

            if let Some(id) = shape_map.get(&shape_id) {
                let mapped_alloc_id = alloc_map.get(&alloc_id).unwrap();
                new.push_shape(*mapped_alloc_id, *id);
                continue;
            }

            let mut new_shape = Shape::default();
            let shape = self.get_shape(shape_id);

            for (&k, &v) in shape.fields.iter() {
                if let RegisterType::Record(a) = v {
                    if let Some(a) = alloc_map.get(&a) {
                        new_shape.fields.insert(k, RegisterType::Record(*a));
                    } else {
                        let new_alloc_id = new.alloc_allocation();
                        need_to_shape.push_back(a);
                        alloc_map.insert(a, new_alloc_id);
                        new_shape
                            .fields
                            .insert(k, RegisterType::Record(new_alloc_id));
                    }
                } else {
                    new_shape.fields.insert(k, v);
                }
            }

            let new_shape_id = new.new_shape(new_shape);
            shape_map.insert(shape_id, new_shape_id);
            new.push_shape(alloc_id, new_shape_id);
        }

        new
    }
}

impl Default for TypeBag {
    fn default() -> Self {
        TypeBag {
            registers: Default::default(),
            alloc_counter: Default::default(),
            alloc_shapes: Default::default(),
            shape_counter: Default::default(),
            shapes: Default::default(),
            const_counter: Default::default(),
            consts: Default::default(),
        }
    }
}

impl PartialEq for TypeBag {
    /// Makes sure that every register pairing of two type bags are the same.
    fn eq(&self, other: &Self) -> bool {
        if self.registers.len() != other.registers.len() {
            return false;
        }

        fn try_helper(me: &TypeBag, you: &TypeBag) -> Option<()> {
            let mut shapes_must_match = VecDeque::new();

            for (reg, typ) in me.registers.iter() {
                let oth_typ = you.registers.get(reg)?;

                match (typ, oth_typ) {
                    (RegisterType::Record(a), RegisterType::Record(b)) => {
                        let a = me.get_shape_id(*a);
                        let b = you.get_shape_id(*b);
                        shapes_must_match.push_back((a, b));
                    }
                    (a, b) if a == b => {
                        continue;
                    }
                    _ => return None,
                }
            }

            let mut equal_shapes = FxHashSet::default();

            while let Some((a, b)) = shapes_must_match.pop_front() {
                // we may have made sure these two shapes have existed before
                if !equal_shapes.insert((a, b)) {
                    continue;
                }

                // now make sure that those shapes were actually equal
                let a = me.get_shape(a);
                let b = you.get_shape(b);
                if a.fields.len() != b.fields.len() {
                    return None;
                }

                for (idx, typ) in a.fields.iter() {
                    let oth_typ = b.fields.get(idx)?;

                    // TODO: dedup this?
                    match (typ, oth_typ) {
                        (RegisterType::Record(a), RegisterType::Record(b)) => {
                            let a = me.get_shape_id(*a);
                            let b = you.get_shape_id(*b);
                            shapes_must_match.push_back((a, b));
                        }
                        (a, b) if a == b => {
                            continue;
                        }
                        _ => return None,
                    }
                }
            }

            Some(())
        }

        try_helper(self, other).map(|_| true).unwrap_or(false)
    }
}