pub mod regalloc;
#[cfg(target_arch = "x86_64")]
pub mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
pub mod frame;

use crate::ir::*;

/// RegGroup describes register class
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegGroup {
    /// general purpose register
    GPR,
    /// requires two general purpose register
    GPREX,
    /// floating point register
    FPR,
}

impl RegGroup {
    pub fn from_node(node: &Node) -> Option<Self> {
        match node {
            Node::Operand(op) => match op {
                Operand::Immediate8(_)
                | Operand::Immediate32(_)
                | Operand::Immediate16(_)
                | Operand::Immediate64(_)
                | Operand::Symbol(_)
                | Operand::Label(_)
                | Operand::LIRFunction(_)
                | Operand::VirtualRegister(_, Type::UInt8)
                | Operand::VirtualRegister(_, Type::UInt16)
                | Operand::VirtualRegister(_, Type::UInt32)
                | Operand::VirtualRegister(_, Type::UInt64)
                | Operand::VirtualRegister(_, Type::Int8)
                | Operand::VirtualRegister(_, Type::Int16)
                | Operand::VirtualRegister(_, Type::Int32)
                | Operand::VirtualRegister(_, Type::Int64)
                | Operand::Register(_, Type::UInt8)
                | Operand::Register(_, Type::UInt16)
                | Operand::Register(_, Type::UInt32)
                | Operand::Register(_, Type::UInt64)
                | Operand::Register(_, Type::Int8)
                | Operand::Register(_, Type::Int16)
                | Operand::Register(_, Type::Int32)
                | Operand::Register(_, Type::Int64) => Some(Self::GPR),
                Operand::Register(_, Type::Float32)
                | Operand::Register(_, Type::Float64)
                | Operand::VirtualRegister(_, Type::Float32)
                | Operand::VirtualRegister(_, Type::Float64) => Some(Self::FPR),
                Operand::Float32(_) | Operand::Float64(_) => Some(Self::FPR),
                _ => None,
            },
            _ => None,
        }
    }
}
