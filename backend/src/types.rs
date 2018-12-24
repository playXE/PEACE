#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Type(u32);
use crate::*;

/// Void type, by default this is i32 value for function with return type `void`,e.g:
/// ```c
/// void func() {return;}
/// ```
/// ```llvm
/// function %func():
/// bb1:
///    %null = iload i32 0
///    ret %null
/// ```
pub const Void: Type = Type(0);
/// Integer type with 32 bits
pub const I32: Type = Type(1);
/// Integer type with 64 bits also used as pointer type, e.g:
/// ```llvm
/// %value = %alloc.ptr 8 ; returns I64
/// ```
pub const I64: Type = Type(2);
/// Float type with 32 bits
pub const F32: Type = Type(3);
/// Float type with 64 bits
pub const F64: Type = Type(4);

pub const I8: Type = Type(5);

use std::mem::size_of;

impl Type {
    pub fn size(&self) -> usize {
        match *self {
            Void | I32 => size_of::<int>(),
            I64 => size_of::<long>(),
            F32 => size_of::<float>(),
            F64 => size_of::<double>(),
            I8 => size_of::<ubyte>(),
            _ => panic!("Unknown type: {:02x}", self.0),
        }
    }
}
