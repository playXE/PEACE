use crate::inst::*;
use crate::Value;
use fnv::{FnvHashMap, FnvHashSet};

pub struct DataFlowGraph {
    pub results: FnvHashMap<Inst, Vec<Value>>,
    pub insts: FnvHashMap<Inst, Instruction>,
    pub variables: FnvHashSet<Value>,
    pub value_locations: FnvHashMap<Value, ValueData>,
}

impl DataFlowGraph {
    pub fn new() -> DataFlowGraph {
        DataFlowGraph {
            results: FnvHashMap::default(),
            insts: FnvHashMap::default(),
            value_locations: FnvHashMap::default(),
            variables: FnvHashSet::default(),
        }
    }
}

use crate::compiler::registers::{Register, XMMRegister};
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum ValueData {
    Gpr(Register),
    Fpr(XMMRegister),
    Stack(i32),
}

impl ValueData {
    pub fn gpr(&self) -> Register {
        match self {
            ValueData::Gpr(reg) => *reg,
            _ => panic!(""),
        }
    }
    pub fn fpr(&self) -> XMMRegister {
        match self {
            ValueData::Fpr(reg) => *reg,
            _ => panic!(""),
        }
    }
    pub fn off(&self) -> i32 {
        match self {
            ValueData::Stack(off) => *off,
            _ => panic!(""),
        }
    }

    pub fn is_gpr(&self) -> bool {
        match self {
            ValueData::Gpr(_) => true,
            _ => false,
        }
    }
    pub fn is_off(&self) -> bool {
        match self {
            ValueData::Stack(_) => true,
            _ => false,
        }
    }

    pub fn is_fpr(&self) -> bool {
        !self.is_gpr() && !self.is_off()
    }
}
