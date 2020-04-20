
use hashlink::{LinkedHashMap,LinkedHashSet};
use string_interner::*;
#[derive(Clone,Debug)]
pub enum Instruction {
    IntBinary(IntBinaryOperation,Box<Node>,Box<Node>,Box<Node>),
    FloatBinary(FloatBinaryOperation,Box<Node>,Box<Node>,Box<Node>),
    /// select %value,$if_non_zero,$if_zero
    Select(Box<Node>,Box<Node>,Box<Node>),
    /// Jump to pointer or basic block.
    /// jump $block or label.
    Jump(Box<Node>),
    /// Jump if condition is true.
    /// jumpci intcc,%value1,%value2,$if_true,$if_false
    JumpCondInt(IntCC,Box<Node>,Box<Node>,Box<Node>,Box<Node>),
    /// jumpcf floatcc, %value1,%value2,$if_true,$if_false
    JumpCondFloat(FloatCC,Box<Node>,Box<Node>,Box<Node>,Box<Node>),
    /// %value = call %function <arguments>
    CallIndirect(Box<Node>,Box<Node>,Vec<Box<Node>>),
    /// %value = call $function <arguments>
    Call(Box<Node>,Box<Node>,Vec<Box<Node>>),
    TailCall(Box<Node>,Vec<Box<Node>>),
    /// Load value at offset. 
    /// 
    /// %value = %ptr + %offset * sizeof(type)
    Load(Box<Node>,Box<Node>,Box<Node>,Type),
    /// Store value at offset.
    /// %ptr + %offset * sizeof(type)
    Store(Box<Node>,Box<Node>,Box<Node>,Type),
    /// Move from one register to another. 
    /// 
    /// It's possible to move virtual register to real and vice versa.
    Move(Box<Node>,Box<Node>),
    /// %value = alloca %size
    Alloca(Box<Node>,Box<Node>),

}
#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum IntBinaryOperation {
    Add,
    Sub,
    Div,
    Mul,
    Mod,
    Shr,
    Shl,
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
}
#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum FloatBinaryOperation {
    Add,
    Sub,
    Div,
    Mul,
    Mod,
}
#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum IntCC {
    Greater,
    UnsignedGreater,
    Less,
    UnsignedLess,
    UnsignedGreaterEqual,
    UnsignedLessEqual,
    Equal,
    UnsignedEqual,
    NotEqual,
}
#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum FloatCC {
    Greater,
    Less,
    LessEqual,
    GreaterEqual,
    Equal,
    NotEqual
}
#[derive(Clone,Debug)]
pub enum Node {
    Instruction(Instruction),
    Operand(Operand),

}
#[derive(Clone,Debug)]
pub enum Operand {
    Immediate32(i32),
    Immediate8(i8),
    Immediate16(i16),
    Immediate64(i64),
    Float64(u64),
    Float32(u32),
    Symbol(Sym),
    Function(Sym),
    Block(Sym),
    Label(Sym),
    /// Virtual register.
    VirtualRegister(usize,Type),
    /// Real machine register. 
    Register(usize,Type),
}

#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum Type {
    Float64,
    Float32,
    Int8,
    Int16,
    Int32,
    Int64,
    
    UInt8,
    UInt16,
    UInt32,
    UInt64,

}

#[cfg(target_pointer_width="64")]
pub const PTR_TYPE: Type = Type::UInt64;
#[cfg(target_pointer_width="32")]
pub const PTR_TYPE: Type = Type::UInt64;

pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub id: usize,
}

pub struct Function {
    pub basic_blocks: Vec<BasicBlock>,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct CFGNode {
    pub block: usize,
    pub succs: Vec<usize>,
    pub preds :Vec<usize>,
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct CFG {
    pub inner: LinkedHashMap<usize,CFGNode>,
}

impl CFG {
    pub fn empty() -> Self {
        Self {
            inner: LinkedHashMap::new()
        }
    }
    pub fn get_blocks(&self) -> Vec<usize> {
        self.inner.keys().map(|x| x.clone()).collect()
    }

    pub fn get_preds(&self, block: &usize) -> &Vec<usize> {
        &self.inner.get(block).unwrap().preds
    }

    pub fn get_succs(&self, block: &usize) -> &Vec<usize> {
        &self.inner.get(block).unwrap().succs
    }

    pub fn has_edge(&self, from: &usize, to: &usize) -> bool {
        if self.inner.contains_key(from) {
            let ref node = self.inner.get(from).unwrap();
            for succ in node.succs.iter() {
                if succ == to {
                    return true;
                }
            }
        }
        false
    }

    /// checks if there exists a path between from and to, without excluded node
    pub fn has_path_with_node_excluded(&self, from: &usize, to: &usize, exclude_node: &usize) -> bool {
        // we cannot exclude start and end of the path
        assert!(exclude_node != from && exclude_node != to);

        if from == to {
            true
        } else {
            // we are doing BFS

            // visited nodes
            let mut visited: LinkedHashSet<&usize> = LinkedHashSet::new();
            // work queue
            let mut work_list: Vec<&usize> = vec![];
            // initialize visited nodes, and work queue
            visited.insert(from);
            work_list.push(from);

            while !work_list.is_empty() {
                let n = work_list.pop().unwrap();
                for succ in self.get_succs(n) {
                    if succ == exclude_node {
                        // we are not going to follow a path with the excluded
                        // node
                        continue;
                    } else {
                        // if we are reaching destination, return true
                        if succ == to {
                            return true;
                        }

                        // push succ to work list so we will traverse them later
                        if !visited.contains(succ) {
                            visited.insert(succ);
                            work_list.push(succ);
                        }
                    }
                }
            }

            false
        }
    }
}