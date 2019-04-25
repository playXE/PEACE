#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Type {
    I8,
    I32,
    I64,

    F32,
    F64,

    Pointer,
    Void,
}

use crate::backend::MachineMode;

impl Type {
    pub fn to_machine(&self) -> MachineMode {
        use MachineMode::*;
        use Type::*;
        match self {
            I8 => Int8,
            I32 => Int32,
            I64 => Int64,
            Pointer => Ptr,
            F32 => Float32,
            F64 => Float64,
            _ => unreachable!(),
        }
    }

    pub fn x64(&self) -> u8 {
        if *self == Type::I64 || *self == Type::F64 {1} else {0}
    }
    pub fn is_float(&self) -> bool {
        if *self == Type::F32 || *self == Type::F64 {true} else {false}
    }
}
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct Value(pub u32);

impl Value {
    pub fn new(v: u32) -> Value {
        Value(v)
    }
}