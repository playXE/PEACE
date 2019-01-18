#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
pub struct Type(u32);

pub const I32: Type = Type(0);
pub const I64: Type = Type(2);
pub const F32: Type = Type(3);
pub const F64: Type = Type(4);
pub const I8: Type = Type(5);

use crate::compiler::MachineMode;

impl Type {
    pub fn to_machine(&self) -> MachineMode {
        match *self {
            I32 => MachineMode::Int32,
            I64 => MachineMode::Int64,
            F32 => MachineMode::Float32,
            F64 => MachineMode::Float64,
            I8 => MachineMode::Int8,
            _ => unreachable!(),
        }
    }

    pub fn is_float(&self) -> bool {
        match *self {
            F32 | F64 => true,
            _ => false,
        }
    }
    pub fn x64(&self) -> u8 {
        match *self {
            F64 | I64 => 1,
            _ => 0,
        }
    }
}
