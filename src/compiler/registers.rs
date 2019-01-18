#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Register {
    RAX = 0,
    RCX = 1,
    RDX = 2,
    RBX = 3,
    RSP = 4,
    RBP = 5,
    RSI = 6,
    RDI = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
    RIP = 16,
}

impl Register {
    pub fn is_basic_reg(self) -> bool {
        self == RAX || self == RBX || self == RCX || self == RDX
    }
    pub fn msb(self) -> u8 {
        assert!(self != RIP);

        (self as u8 >> 3) & 0x01
    }

    pub fn and7(self) -> u8 {
        assert!(self != RIP);

        self as u8 & 0x07
    }
}

pub use self::Register::*;

#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum XMMRegister {
    XMM0 = 0,
    XMM1 = 1,
    XMM2 = 2,
    XMM3 = 3,
    XMM4 = 4,
    XMM5 = 5,
    XMM6 = 6,
    XMM7 = 7,
    XMM8 = 8,
    XMM9 = 9,
    XMM10 = 10,
    XMM11 = 11,
    XMM12 = 12,
    XMM13 = 13,
    XMM14 = 14,
    XMM15 = 15,
}

impl XMMRegister {
    pub fn msb(self) -> u8 {
        //assert!(self != RIP);

        (self as u8 >> 3) & 0x01
    }

    pub fn and7(self) -> u8 {
        //assert!(self != RIP);

        self as u8 & 0x07
    }
}

pub use self::XMMRegister::*;

pub type FpuRegister = XMMRegister;
pub const FpuTMP: FpuRegister = XMM0;

pub const TMP: Register = R11;
pub const TMP2: Register = R10;
pub const PP: Register = R15;
/// Stack pointer register
pub const SPREG: Register = RSP;
/// Frame pointer register
pub const FPREG: Register = RBP;

pub const kExceptionObjectReg: Register = RAX;
pub const kStackTraceObjectReg: Register = RDX;

#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
pub enum Reg {
    Gpr(Register),
    Float(XMMRegister),
}

impl Reg {
    pub fn reg(&self) -> Register {
        match self {
            Reg::Gpr(reg) => *reg,
            _ => panic!(""),
        }
    }

    pub fn freg(&self) -> XMMRegister {
        match self {
            Reg::Float(float) => *float,
            _ => panic!(""),
        }
    }
}
