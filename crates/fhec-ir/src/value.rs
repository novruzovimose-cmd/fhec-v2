use crate::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constant {
    Bool(bool),
    UInt { value: u64, width: u32 },
    Int { value: i64, width: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub id: ValueId,
    pub ty: Type,
}

impl Value {
    pub const fn new(id: ValueId, ty: Type) -> Self {
        Self { id, ty }
    }
}
