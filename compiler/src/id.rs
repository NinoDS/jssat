use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::atomic::AtomicUsize};

use rustc_hash::FxHashMap;

macro_rules! gen_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name<Ctx>(::std::num::NonZeroUsize, ::std::marker::PhantomData<Ctx>);

        impl<C: ContextTag> $name<C> {
            pub fn new() -> Self {
                Self::new_const()
            }

            pub const fn new_const() -> Self {
                Self::new_with_value_const(0)
            }

            pub const fn new_with_value_const(value: usize) -> Self {
                debug_assert!(value != usize::MAX);
                Self(
                    ::std::num::NonZeroUsize::new(value + 1).unwrap(),
                    ::std::marker::PhantomData,
                )
            }

            pub fn next(&self) -> Self {
                <Self as crate::id::IdCompat>::new_with_value(
                    <Self as crate::id::IdCompat>::value(&self) + 1,
                )
            }

            pub fn next_and_mut(&mut self) -> Self {
                let result = *self;
                *self = self.next();
                result
            }

            pub fn convert<I: IdCompat>(&self) -> I {
                convert::<Self, I>(*self)
            }

            pub fn map_context<C2: ContextTag>(&self) -> $name<C2> {
                $name::<C2>::new_with_value_const(self.value())
            }
        }

        impl<C: ContextTag> crate::id::IdCompat for $name<C> {
            fn new_with_value(value: usize) -> Self {
                $name::<C>::new_with_value_const(value)
            }

            fn value(&self) -> usize {
                self.0.get() - 1
            }
        }

        impl<C: ContextTag> Default for $name<C> {
            fn default() -> Self {
                Self::new_with_value(0)
            }
        }

        impl<C> ::std::fmt::Display for $name<C> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

pub trait IdCompat: PartialEq + Eq + Hash + Sized + Default + Debug + Copy + Clone {
    fn new() -> Self {
        Self::default()
    }

    fn new_with_value(value: usize) -> Self;

    fn value(&self) -> usize;
}

pub trait ContextTag: PartialEq + Eq + Hash + Sized + Copy + Clone + Debug {}

pub fn convert<A, B>(a: A) -> B
where
    A: IdCompat,
    B: IdCompat,
{
    B::new_with_value(a.value())
}

macro_rules! gen_id_ctx {
    (
        // https://amanjeev.com/blog/rust-document-macro-invocations
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {}

        impl ContextTag for $name {}
    };
}

gen_id_ctx!(
    /// Represents an ID that doesn't have an inherent context. Contexts are used
    /// on IDs to differentiate them from different passes, as some passes may
    /// generate their own IDs, and differentiating the two by the type is useful.
    NoContext
);

// TODO: this should be in `assembler.rs` but meh
gen_id_ctx!(AssemblerCtx);
gen_id_ctx!(
    /// Context used by [`crate::frontend::ir`]
    IrCtx
);
gen_id_ctx!(
    /// Context used by [`crate::frontend::type_annotator`]
    AnnotatedCtx
);
gen_id_ctx!(
    /// Context used by [`crate::frontend::conv_only_bb`]
    PureBbCtx
);
gen_id_ctx!(
    /// Context used by [`crate::backend::llvm`]
    LlvmCtx
);

gen_id!(TypeId);
gen_id!(TopLevelId);
gen_id!(ConstantId);
gen_id!(GlobalId);
gen_id!(FunctionId);
gen_id!(ExternalFunctionId);
gen_id!(BlockId);
gen_id!(RegisterId);
gen_id!(InternalSlotId);
gen_id!(StructId);
gen_id!(OpaqueStructId);
gen_id!(AllocationId);
gen_id!(ShapeId);

#[derive(Debug)]
pub struct Counter<I> {
    current: AtomicUsize,
    phantom: PhantomData<I>,
}

impl<I: IdCompat> Counter<I> {
    pub fn new() -> Self {
        Self::new_with_value(0)
    }

    pub fn new_with_value(current: usize) -> Self {
        Counter {
            current: AtomicUsize::new(current),
            phantom: PhantomData::default(),
        }
    }

    pub fn after(current: I) -> Self {
        Self::new_with_value(current.value() + 1)
    }

    pub fn current(&self) -> I {
        I::new_with_value(self.current.load(std::sync::atomic::Ordering::Relaxed))
    }

    pub fn next(&self) -> I {
        I::new_with_value(
            self.current
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        )
    }

    pub fn dup(&self) -> Self {
        Self::new_with_value(self.current.load(std::sync::atomic::Ordering::Relaxed))
    }
}

impl<I: IdCompat> Default for Counter<I> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct RegIdMap<T, U> {
    registers: FxHashMap<RegisterId<T>, RegisterId<U>>,
    reg_counter: Counter<RegisterId<U>>,
}

impl<T, U> RegIdMap<T, U>
where
    T: ContextTag,
    U: ContextTag,
{
    pub fn new() -> Self {
        Self {
            registers: Default::default(),
            reg_counter: Default::default(),
        }
    }

    pub fn map(&mut self, source: RegisterId<T>) -> RegisterId<U> {
        let reg_counter = &self.reg_counter;
        *(self.registers)
            .entry(source)
            .or_insert_with(|| reg_counter.next())
    }

    pub fn gen(&mut self) -> RegisterId<U> {
        self.reg_counter.next()
    }
}

impl<T, U> Default for RegIdMap<T, U>
where
    T: ContextTag,
    U: ContextTag,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct BlockIdMap<T, U> {
    registers: FxHashMap<BlockId<T>, BlockId<U>>,
    reg_counter: Counter<BlockId<U>>,
}

impl<T, U> BlockIdMap<T, U>
where
    T: ContextTag,
    U: ContextTag,
{
    pub fn new() -> Self {
        Self {
            registers: Default::default(),
            reg_counter: Default::default(),
        }
    }

    pub fn map(&mut self, source: BlockId<T>) -> BlockId<U> {
        let reg_counter = &self.reg_counter;
        *(self.registers)
            .entry(source)
            .or_insert_with(|| reg_counter.next())
    }
}

impl<T, U> Default for BlockIdMap<T, U>
where
    T: ContextTag,
    U: ContextTag,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct AllocIdMap<T, U> {
    registers: FxHashMap<AllocationId<T>, AllocationId<U>>,
    reg_counter: Counter<AllocationId<U>>,
}

impl<T, U> AllocIdMap<T, U>
where
    T: ContextTag,
    U: ContextTag,
{
    pub fn new() -> Self {
        Self {
            registers: Default::default(),
            reg_counter: Default::default(),
        }
    }

    pub fn map(&mut self, source: AllocationId<T>) -> AllocationId<U> {
        let (id, _) = self.map_is_new(source);
        id
    }

    pub fn map_is_new(&mut self, source: AllocationId<T>) -> (AllocationId<U>, bool) {
        let reg_counter = &self.reg_counter;
        let mut is_new = false;
        let alloc = *(self.registers).entry(source).or_insert_with(|| {
            is_new = true;
            reg_counter.next()
        });
        (alloc, is_new)
    }

    pub fn gen(&mut self) -> AllocationId<U> {
        self.reg_counter.next()
    }
}

impl<T, U> Default for AllocIdMap<T, U>
where
    T: ContextTag,
    U: ContextTag,
{
    fn default() -> Self {
        Self::new()
    }
}
