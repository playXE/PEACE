use super::*;
use crate::ir::*;
use std::collections::HashMap;
pub struct ConstantFoldingPass;

#[derive(Copy, Clone)]
pub enum Const {
    Int(i64),
    Float(f32),
    Double(f64),
}

impl<'a> FunctionPass<'a> for ConstantFoldingPass {
    type Output = ();
    type Err = ();
    fn run<'b: 'a>(&mut self, f: &'b mut LIRFunction) -> Result<Self::Output, Self::Err> {
        let mut constants = HashMap::new();

        for bb in f.code.iter_mut() {
            for ins in bb.instructions.iter_mut() {
                if let Instruction::LoadImm(dst, imm, t) = ins {
                    let c = match &**imm {
                        Node::Operand(op) => match op {
                            Operand::Immediate64(x) if *t > Type::Float32 => Const::Int(*x),
                            Operand::Immediate32(x) if *t > Type::Float32 => Const::Int(*x as _),
                            Operand::Immediate8(x) => Const::Int(*x as _),
                            Operand::Immediate16(x) => Const::Int(*x as _),
                            Operand::Immediate32(x) => Const::Float(f32::from_bits(*x as u32)),
                            Operand::Immediate64(x) => Const::Double(f64::from_bits(*x as u64)),
                            _ => continue,
                        },
                        _ => unreachable!(),
                    };
                    constants.insert(dst.any_reg_id(), c);
                } else if let Instruction::IntBinary(op, dst, x, y) = ins {
                    match (
                        try_get_const(&constants, x, Type::Int64),
                        try_get_const(&constants, y, Type::Int64),
                    ) {
                        (Some(x), Some(y)) => match (x, y) {
                            (Const::Int(x), Const::Int(y)) => {
                                *ins = Instruction::LoadImm(
                                    dst.clone(),
                                    Box::new(Node::Operand(match *op {
                                        IntBinaryOperation::Add => {
                                            Operand::Immediate64(x.wrapping_add(y))
                                        }
                                        IntBinaryOperation::Sub => {
                                            Operand::Immediate64(x.wrapping_sub(y))
                                        }
                                        IntBinaryOperation::Div => {
                                            Operand::Immediate64(x.wrapping_div(y))
                                        }
                                        IntBinaryOperation::Mul => {
                                            Operand::Immediate64(x.wrapping_mul(y))
                                        }
                                        IntBinaryOperation::Shl => {
                                            Operand::Immediate64(x.wrapping_shl(y as u32))
                                        }
                                        IntBinaryOperation::Shr => {
                                            Operand::Immediate64(x.wrapping_shr(y as u32))
                                        }
                                        IntBinaryOperation::BitwiseAnd => {
                                            Operand::Immediate64(x & y)
                                        }
                                        IntBinaryOperation::BitwiseOr => {
                                            Operand::Immediate64(x | y)
                                        }
                                        IntBinaryOperation::BitwiseXor => {
                                            Operand::Immediate64(x ^ y)
                                        }
                                        _ => continue,
                                    })),
                                    Type::Int64,
                                )
                            }
                            _ => continue,
                        },
                        (None, Some(x)) => match x {
                            Const::Double(x) => {
                                *y = Box::new(Node::Operand(Operand::Immediate64(
                                    x.to_bits() as i64
                                )))
                            }
                            Const::Float(x) => {
                                *y = Box::new(Node::Operand(Operand::Immediate32(
                                    x.to_bits() as i32
                                )))
                            }
                            Const::Int(x) => *y = Box::new(Node::Operand(Operand::Immediate64(x))),
                            _ => continue,
                        },
                        _ => continue,
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn try_get_const(c: &HashMap<usize, Const>, node: &Node, t: Type) -> Option<Const> {
    if let Node::Operand(Operand::Register(r, _)) = node {
        if let Some(c) = c.get(r) {
            return Some(*c);
        } else {
            None
        }
    } else {
        let c = match node {
            Node::Operand(x) => match x {
                Operand::Immediate64(x) if t > Type::Float32 => Const::Int(*x),
                Operand::Immediate32(x) if t > Type::Float32 => Const::Int(*x as _),
                Operand::Immediate8(x) => Const::Int(*x as _),
                Operand::Immediate16(x) => Const::Int(*x as _),
                Operand::Immediate32(x) => Const::Float(f32::from_bits(*x as u32)),
                Operand::Immediate64(x) => Const::Double(f64::from_bits(*x as u64)),
                _ => return None,
            },
            _ => return None,
        };
        Some(c)
    }
}
