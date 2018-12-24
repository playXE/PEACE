#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

pub mod amd64;
pub mod datasegment;
pub mod registers;
pub mod sink;
pub mod types;

pub type ubyte = u8;
pub type byte = i8;
pub type float = f32;
pub type double = f64;
pub type int = i32;
pub type long = i64;
pub type ptr = *const u8;
pub type mut_ptr = *mut u8;
pub type uint = u32;
pub type ulong = u64;
pub type size_t = usize;

#[macro_export]
macro_rules! as_pointer {
    ($v:expr) => {
        $v as *const u8
    };
    ($t:tt: $v:expr) => {
        $v as *const $t as *const u8
    };
}

pub fn align(value: i32, align: i32) -> i32 {
    if align == 0 {
        return value;
    }

    ((value + align - 1) / align) * align
}

use self::registers::Reg;

pub fn fits_i32(n: i64) -> bool {
    n == (n as i32) as i64
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CondCode {
    Zero,
    NonZero,
    Equal,
    NotEqual,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    UnsignedGreater,
    UnsignedGreaterEq,
    UnsignedLess,
    UnsignedLessEq,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Membase {
    // rbp + val1
    Local(i32),

    // reg1 + val1
    Base(Reg, i32),

    // reg1 + reg2 * val1 + val2
    Index(Reg, Reg, i32, i32),

    // reg1 * val1 + val2
    Offset(Reg, i32, i32),
}
