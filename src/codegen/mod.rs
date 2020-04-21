pub mod regalloc;
#[cfg(target_arch = "x86_64")]
pub mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
pub mod frame;
use crate::ir::*;
use byteorder::WriteBytesExt;
use byteorder::{ByteOrder, LittleEndian};

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
    pub fn from_ty(t: Type) -> Option<Self> {
        match t {
            Type::Float32 | Type::Float64 => Some(Self::FPR),
            _ => Some(Self::GPR),
        }
    }
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

pub fn sequential_layout(tys: &[Type]) -> (usize, usize, Vec<usize>) {
    let mut offsets = vec![];
    let mut cur = 0;
    let mut struct_align = 1;
    for ty in tys.iter() {
        let align = ty.align();
        struct_align = num::integer::lcm(struct_align, align);
        cur = crate::util::math::align_up(cur, align);
        offsets.push(cur);
        cur += ty.size();
    }
    let size = crate::util::math::align_up(cur, struct_align);
    (size, struct_align, offsets)
}

/// returns bit representations for f32
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub fn f32_to_raw(val: f32) -> u32 {
    let mut ret = vec![];
    ret.write_f32::<LittleEndian>(val).unwrap();
    LittleEndian::read_uint(&mut ret, 4) as u32
}

/// returns bit representations for f64
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub fn f64_to_raw(val: f64) -> u64 {
    let mut ret = vec![];
    ret.write_f64::<LittleEndian>(val).unwrap();
    LittleEndian::read_uint(&mut ret, 8)
}
