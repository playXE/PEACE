#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Int64,
    Int32,
    Float64,
    Float32,
    Bool32,
    Bool64,

    Pointer,
}

use peace_backend::types::*;

pub use self::Kind::*;

impl Kind {
    pub fn to_machine(&self) -> Type {
        match self {
            Int64 | Bool64 | Pointer => I64,
            Int32 | Bool32 => I32,
            Float32 => F32,
            Float64 => F64,
        }
    }

    pub fn x64(&self) -> u8 {
        match self {
            Int32 | Bool32 => 0,
            Int64 | Pointer | Bool64 => 1,
            _ => unreachable!(),
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Int32 | Int64 | Bool32 | Bool64 | Pointer => true,
            _ => false,
        }
    }
}
