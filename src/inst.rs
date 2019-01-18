#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
pub struct Inst(u32);

use crate::impl_entity;

impl_entity!(Inst);

use crate::compiler::CondCode;
use crate::types::Type;
use crate::Value;

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum Opcode {
    Iconst,
    F32Const,
    F64Const,
    Iadd,
    Isub,
    Idiv,
    Imul,
    Fadd,
    Fdiv,
    Fmul,
    Fsub,
    Fcmp,
    Icmp,
    Call,
}

#[derive(Clone, Debug)]
pub enum Instruction {
    UnaryImm {
        opcode: Opcode,
        imm: i64,
    },
    UnaryIeee32 {
        opcode: Opcode,
        imm: u32,
    },
    UnaryIeee64 {
        opcode: Opcode,
        imm: u64,
    },

    Binary {
        opcode: Opcode,
        ty: Type,
        x: Value,
        y: Value,
    },
    Cmp {
        opcode: Opcode,
        cond: CondCode,
        x: Value,
        y: Value,
    },

    Jmp(u32),
    JmpZero(Value, u32),
    JmpNzero(Value, u32),
}
