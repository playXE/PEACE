use crate::codegen::*;
use derive_more::Display;
use hashlink::{LinkedHashMap, LinkedHashSet};
use string_interner::*;
#[derive(Clone, Debug, PartialEq, Eq, Display)]
pub enum Instruction {
    #[display(fmt = "{} = Int.{:?} {} {}", _1, _0, _2, _3)]
    IntBinary(IntBinaryOperation, Box<Node>, Box<Node>, Box<Node>),
    #[display(fmt = "{} = Float.{:?} {} {}", _1, _0, _2, _3)]
    FloatBinary(FloatBinaryOperation, Box<Node>, Box<Node>, Box<Node>),
    /// select %value,$if_non_zero,$if_zero
    #[display(fmt = "select {} {} {}", _0, _1, _2)]
    Select(Box<Node>, Box<Node>, Box<Node>),
    /// Jump to pointer or basic block.
    /// jump $block or label.
    #[display(fmt = "jump {}", _0)]
    Jump(Box<Node>),
    /// Jump if condition is true.
    /// jumpci intcc,%value1,%value2,$if_true,$if_false
    #[display(fmt = "jumpi_if_{:?} {} {} {} {}", _0, _1, _2, _3, _4)]
    JumpCondInt(IntCC, Box<Node>, Box<Node>, Box<Node>, Box<Node>),
    /// jumpcf floatcc, %value1,%value2,$if_true,$if_false
    #[display(fmt = "jumpf_if_{:?} {} {} {} {}", _0, _1, _2, _3, _4)]
    JumpCondFloat(FloatCC, Box<Node>, Box<Node>, Box<Node>, Box<Node>),
    /// %value = call %function <arguments>
    #[display(fmt = "call_indirect {} {} <args not displayed>", _0, _1)]
    CallIndirect(Box<Node>, Box<Node>, Vec<Box<Node>>, FunctionSignature),
    /// %value = call $function <arguments>
    #[display(fmt = "{} = call {} <args not displayed>", _0, _1)]
    Call(Box<Node>, Box<Node>, Vec<Box<Node>>, FunctionSignature),
    #[display(fmt = "tcall {} <args not displayed>", _0)]
    TailCall(Box<Node>, Vec<Box<Node>>, FunctionSignature),
    /// Load value at offset.
    ///
    /// %value = %ptr + %offset * sizeof(type)
    #[display(fmt = "{} = load.{} {} + {}", _0, _3, _1, _2)]
    Load(Box<Node>, Box<Node>, Box<Node>, Type),
    /// Store value at offset.
    /// %ptr + %offset * sizeof(type)
    #[display(fmt = "store.{} {} + {}, {}", _3, _0, _1, _2)]
    Store(Box<Node>, Box<Node>, Box<Node>, Type),
    /// Move from one register to another.
    ///
    /// It's possible to move virtual register to real and vice versa.
    #[display(fmt = "move {} {}", _0, _1)]
    Move(Box<Node>, Box<Node>),
    /// %value = alloca %size
    #[display(fmt = "{} = alloca {}", _0, _1)]
    Alloca(Box<Node>, Box<Node>),
    #[display(fmt = "{} = cast.{} {}", _0, _2, _1)]
    Cast(Box<Node>, Box<Node>, Type),
    #[display(fmt = "{} = reinterpret_as.{} {}", _0, _2, _1)]
    Reinterpret(Box<Node>, Box<Node>, Type),
    #[display(fmt = "{} = load_imm.{} {}", _0, _2, _1)]
    LoadImm(Box<Node>, Box<Node>, Type),
    #[display(fmt = "return {}", _0)]
    Return(Box<Node>),
    #[display(fmt = "{} = load_param.{} {} ", _0, _2, _1)]
    LoadParam(Box<Node>, usize, Type),
    #[display(fmt = "{} = call {}", _0, _1)]
    RawCall(Box<Node>, Box<Node>),
    #[display(fmt = "tcall {}", _0)]
    RawTCall(Box<Node>),
    #[display(fmt = "Int.{:?} {} {}", _0, _1, _2)]
    RawIntBinary(IntBinaryOperation, Box<Node>, Box<Node>),
    #[display(fmt = "Float.{:?} {} {}", _0, _1, _2)]
    RawFloatBinary(FloatBinaryOperation, Box<Node>, Box<Node>),
}
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FloatBinaryOperation {
    Add,
    Sub,
    Div,
    Mul,
    Mod,
}
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FloatCC {
    Greater,
    Less,
    LessEqual,
    GreaterEqual,
    Equal,
    NotEqual,
}
#[derive(Clone, Debug, PartialEq, Eq, Display)]

pub enum Node {
    #[display(fmt = "none")]
    None,
    #[display(fmt = "{}", _0)]
    Instruction(Instruction),
    #[display(fmt = "{}", _0)]
    Operand(Operand),
}

impl Node {
    pub fn try_replace_reg(&mut self, from: usize, to: usize) -> bool {
        match self {
            Self::Operand(x) => match x {
                Operand::Register(r, _) => {
                    if *r == from {
                        *r = to;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn any_reg_id(&self) -> usize {
        match self {
            Node::Operand(Operand::VirtualRegister(r, _))
            | Node::Operand(Operand::Register(r, _)) => *r,
            _ => unreachable!(),
        }
    }
    pub fn maybe_any_reg_id(&self) -> usize {
        match self {
            Node::Operand(Operand::VirtualRegister(r, _))
            | Node::Operand(Operand::Register(r, _)) => *r,
            _ => unreachable!(),
        }
    }
    pub fn block_id(&self) -> Option<usize> {
        match self {
            Node::Operand(Operand::Block(x)) => Some(*x),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Display)]
pub enum Operand {
    #[display(fmt = "{}", _0)]
    Immediate32(i32),
    #[display(fmt = "{}", _0)]
    Immediate8(i8),
    #[display(fmt = "{}", _0)]
    Immediate16(i16),
    #[display(fmt = "{}", _0)]
    Immediate64(i64),
    #[display(fmt = "{}", _0)]
    Float64(u64),
    #[display(fmt = "{}", _0)]
    Float32(u32),
    #[display(fmt = "{:?}", _0)]
    Symbol(String),
    #[display(fmt = "{:?}", _0)]
    LIRFunction(String),
    #[display(fmt = "$bb{:?}", _0)]
    Block(usize),
    #[display(fmt = "$lbl{:?}", _0)]
    Label(Sym),
    #[display(fmt = "%v{}", _0)]
    /// Virtual register.
    VirtualRegister(usize, Type),
    #[display(fmt = "%r{}", _0)]
    /// Real machine register.
    Register(usize, Type),
    #[display(fmt = "{}", _0)]
    Memory(MemoryLocation),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, Display)]
pub enum Type {
    #[display(fmt = "f64")]
    Float64 = 1,
    #[display(fmt = "f32")]
    Float32,
    #[display(fmt = "i8")]
    Int8,
    #[display(fmt = "i16")]
    Int16,
    #[display(fmt = "i32")]
    Int32,
    #[display(fmt = "i64")]
    Int64,
    #[display(fmt = "u8")]
    UInt8,
    #[display(fmt = "u16")]
    UInt16,
    #[display(fmt = "u32")]
    UInt32,
    #[display(fmt = "u64")]
    UInt64,
}

#[cfg(target_pointer_width = "64")]
pub const PTR_TYPE: Type = Type::UInt64;
#[cfg(target_pointer_width = "32")]
pub const PTR_TYPE: Type = Type::UInt64;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub id: usize,
    pub livein: Vec<usize>,
    pub liveout: Vec<usize>,
}

impl BasicBlock {
    pub fn new(idx: usize, code: Vec<Instruction>) -> Self {
        Self {
            id: idx,
            instructions: code,
            livein: vec![],
            liveout: vec![],
        }
    }
    pub fn join(&mut self, other: BasicBlock) {
        self.instructions.pop();
        for ins in other.instructions {
            self.instructions.push(ins);
        }
    }
    pub fn branch_targets(&self) -> [Option<&Box<Node>>; 2] {
        let last_ins = &self.instructions[self.instructions.len() - 1];
        match &*last_ins {
            Instruction::Select(_, if_nzero, if_zero) => [Some(if_nzero), Some(if_zero)],
            Instruction::JumpCondInt(_, _, _, if_nzero, if_zero)
            | Instruction::JumpCondFloat(_, _, _, if_nzero, if_zero) => {
                [Some(if_nzero), Some(if_zero)]
            }
            Instruction::Jump(x) => [Some(x), None],
            Instruction::Return(_) => [None, None],
            Instruction::TailCall(_, _, _) => [None, None],
            _ => panic!("Terminator not found in {:#?}", self),
        }
    }

    pub fn try_replace_branch_targets(&mut self, from: Box<Node>, to: Box<Node>) -> bool {
        let i = self.instructions.len() - 1;
        let last_ins = &mut self.instructions[i];
        match &mut *last_ins {
            Instruction::Return(_) => false,
            Instruction::TailCall(_, _, _) => false,
            Instruction::Jump(x) => {
                if *x == from {
                    *x = to;
                    true
                } else {
                    false
                }
            }
            Instruction::Select(_, x, y)
            | Instruction::JumpCondFloat(_, _, _, x, y)
            | Instruction::JumpCondInt(_, _, _, x, y) => {
                if *x == from {
                    *x = to;
                    true
                } else if *y == from {
                    *y = to;
                    true
                } else {
                    false
                }
            }
            _ => panic!("Terminator not found in {:#?}", self),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CFGNode {
    pub block: usize,
    pub succs: Vec<usize>,
    pub preds: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CFG {
    pub inner: LinkedHashMap<usize, CFGNode>,
}

impl CFG {
    pub fn empty() -> Self {
        Self {
            inner: LinkedHashMap::new(),
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
    pub fn has_path_with_node_excluded(
        &self,
        from: &usize,
        to: &usize,
        exclude_node: &usize,
    ) -> bool {
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

use core::hash::{Hash, Hasher};

impl Hash for BasicBlock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for BasicBlock {}
impl PartialEq for BasicBlock {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct LIRFunction {
    pub signature: FunctionSignature,
    pub code: Vec<BasicBlock>,
    pub cfg: Option<CFG>,
    pub frame: crate::codegen::frame::Frame,
    pub loop_analysis: Option<crate::pass::loopanalysis::LoopAnalysisResult>,
    pub values: LinkedHashMap<usize, Box<Node>>,
    pub functions: LinkedHashMap<String, FunctionSignature>,
    pub data: LinkedHashSet<Sym>,
}

impl LIRFunction {
    pub fn new(sig: FunctionSignature) -> Self {
        Self {
            signature: sig,
            code: vec![],
            cfg: None,
            frame: crate::codegen::frame::Frame::new(),
            loop_analysis: None,
            functions: LinkedHashMap::new(),
            data: LinkedHashSet::new(),
            values: LinkedHashMap::new(),
        }
    }
    fn init_machine_regs_for_func(&mut self) {
        use crate::codegen::*;;
        for reg in ALL_MACHINE_REGS.values() {
            let id = reg.any_reg_id();
            self.values.insert(id, Box::new((**reg).clone()));
        }
    }
    pub fn print_to_stdout(&self) {
        print!("{} func {} (", self.signature.ret, self.signature.name);
        for (i, p) in self.signature.params.iter().enumerate() {
            print!("{}", p);
            if i != self.signature.params.len() - 1 {
                print!(",");
            }
        }
        print!("): \n");
        for bb in self.code.iter() {
            print!("  $bb{}: \n", bb.id);
            for ins in bb.instructions.iter() {
                print!("   {}\n", ins);
            }
        }
    }

    pub fn get_value(&self, id: usize) -> &Node {
        self.values.get(&id).unwrap()
    }

    pub fn build_cfg(&mut self) {
        let mut ret = CFG::empty();
        let code = &mut self.code;
        let mut predecessors_: LinkedHashMap<usize, LinkedHashSet<usize>> = LinkedHashMap::new();
        for (id, block) in code.iter().enumerate() {
            if block.instructions.is_empty() {
                continue;
            }
            for target in block
                .branch_targets()
                .iter()
                .map(|x| x.map(|x| x.block_id()))
                .flatten()
            {
                if target.is_none() {
                    continue;
                }
                let target = target.unwrap();
                match predecessors_.get_mut(&target) {
                    Some(set) => {
                        set.insert(id);
                    }
                    None => {
                        let mut set = LinkedHashSet::new();
                        set.insert(id);
                        predecessors_.insert(target, set);
                    }
                }
            }
        }

        let mut successors_: LinkedHashMap<usize, LinkedHashSet<usize>> = LinkedHashMap::new();
        for (id, block) in code.iter().enumerate() {
            if block.instructions.is_empty() {
                continue;
            }

            for target in block
                .branch_targets()
                .iter()
                .map(|x| x.map(|x| x.block_id()))
                .flatten()
            {
                if target.is_none() {
                    continue;
                }
                let target = target.unwrap();
                match successors_.get_mut(&id) {
                    Some(set) => {
                        set.insert(target);
                    }
                    None => {
                        let mut set = LinkedHashSet::new();
                        set.insert(target);
                        successors_.insert(id, set);
                    }
                }
            }
        }

        for (id, _block) in code.iter().enumerate() {
            let mut node = CFGNode {
                block: id as _,
                preds: vec![],
                succs: vec![],
            };
            if predecessors_.contains_key(&id) {
                for pred in predecessors_.get(&id).unwrap() {
                    node.preds.push(*pred as usize)
                }
            }

            if successors_.contains_key(&id) {
                for succ in successors_.get(&id).unwrap() {
                    node.succs.push(*succ as usize);
                }
            }
            ret.inner.insert(id as usize, node);
        }
        self.cfg = Some(ret);
    }
}
use std::collections::HashSet;
impl Instruction {
    pub fn branch_targets(&self) -> [Option<&Box<Node>>; 2] {
        let last_ins = self;
        match &*last_ins {
            Instruction::Select(_, if_nzero, if_zero) => [Some(if_nzero), Some(if_zero)],
            Instruction::JumpCondInt(_, _, _, if_nzero, if_zero)
            | Instruction::JumpCondFloat(_, _, _, if_nzero, if_zero) => {
                [Some(if_nzero), Some(if_zero)]
            }
            Instruction::Jump(x) => [Some(x), None],
            Instruction::Return(_) => [None, None],
            Instruction::TailCall(_, _, _) => [None, None],
            _ => unreachable!(),
        }
    }
    pub fn replace_reg(&mut self, from: usize, to: usize) {
        macro_rules! r {
            ($d: expr) => {
                {$d.try_replace_reg(from, to);}
            };
            ($($d: expr) *) => {
                {$(r!($d);)*}
            }
        }
        match self {
            Instruction::LoadImm(dst, _, _) => {
                r!(dst);
            }
            Instruction::RawIntBinary(_, x, y) => r!(x y),
            Instruction::RawFloatBinary(_, x, y) => r!(x y),
            Instruction::Move(x, y) => r!(x y),
            Instruction::Select(x, y, z) => r!(x y z),
            Instruction::FloatBinary(_, x, y, z) => r!(x y z),
            Instruction::IntBinary(_, x, y, z) => r!(x y z),
            Instruction::Jump(x) => r!(x),
            Instruction::JumpCondInt(_, x, y, z, w) => r!(x y z w),
            Instruction::JumpCondFloat(_, x, y, z, w) => r!(x y z w),
            Instruction::CallIndirect(x, y, args, _) => {
                r!(x y);
                for r in args.iter_mut() {
                    r!(r);
                }
            }
            Instruction::RawCall(dst, f) => {
                r!(dst);
                r!(f);
            }
            Instruction::RawTCall(f) => {
                r!(f);
            }
            Instruction::Call(r, _, args, _) => {
                r!(r);
                for r in args.iter_mut() {
                    r!(r);
                }
            }
            Instruction::TailCall(r, args, _) => {
                r!(r);
                for r in args.iter_mut() {
                    r!(r);
                }
            }
            Instruction::Load(x, y, z, _) => {
                r!(x y z);
            }
            Instruction::Store(x, y, z, _) => {
                r!(x y z);
            }
            Instruction::Alloca(x, _) => {
                r!(x);
            }
            Instruction::Cast(x, y, _) => r!(x y),
            Instruction::Reinterpret(x, y, _) => r!(x y),
            Instruction::Return(x) => r!(x),
            Instruction::LoadParam(r, _, _) => r!(r),
        }
    }
    pub fn get_defs(&self) -> Vec<usize> {
        let mut set = HashSet::new();
        match self {
            Instruction::LoadImm(dst, _, _) | Instruction::LoadParam(dst, _, _) => {
                set.insert(dst.any_reg_id());
            }
            Instruction::Move(dst, _) => {
                if let Node::Operand(Operand::Register(_, _)) = &**dst {
                    set.insert(dst.any_reg_id());
                }
            }
            Instruction::RawCall(def, _) => {
                set.insert(def.any_reg_id());
            }
            Instruction::Call(val, _, _, _) | Instruction::CallIndirect(val, _, _, _) => {
                set.insert(val.any_reg_id());
            }
            Instruction::Load(dst, _, _, _) => {
                set.insert(dst.any_reg_id());
            }
            Instruction::Alloca(dst, _) => {
                set.insert(dst.any_reg_id());
            }
            Instruction::FloatBinary(_, dst, _, _) | Instruction::IntBinary(_, dst, _, _) => {
                set.insert(dst.any_reg_id());
            }
            Instruction::Cast(dst, _, _) | Instruction::Reinterpret(dst, _, _) => {
                set.insert(dst.any_reg_id());
            }
            Instruction::RawFloatBinary(_, x, y) | Instruction::RawIntBinary(_, x, y) => {}
            _ => (),
        }
        set.iter().copied().collect()
    }

    pub fn get_uses(&self) -> Vec<usize> {
        let mut set = HashSet::new();
        match self {
            Instruction::RawFloatBinary(_, x, y) | Instruction::RawIntBinary(_, x, y) => {
                set.insert(x.any_reg_id());
                if let Node::Operand(Operand::Register(r, _)) = &**y {
                    set.insert(*r);
                }
            }
            Instruction::Call(_, c, args, _) | Instruction::CallIndirect(_, c, args, _) => {
                if let Instruction::CallIndirect(_, _, _, _) = self {
                    set.insert(c.any_reg_id());
                }
                for arg in args.iter() {
                    set.insert(arg.any_reg_id());
                }
            }
            Instruction::RawCall(_, f) | Instruction::RawTCall(f) => {
                if let Node::Operand(Operand::Register(r, _)) = &**f {
                    set.insert(*r);
                }
            }
            Instruction::Jump(node) => match &**node {
                Node::Operand(Operand::VirtualRegister(r, _))
                | Node::Operand(Operand::Register(r, _)) => {
                    set.insert(*r);
                }
                _ => (),
            },
            Instruction::JumpCondFloat(_, x, y, n1, n2)
            | Instruction::JumpCondInt(_, x, y, n1, n2) => {
                set.insert(x.any_reg_id());
                set.insert(y.any_reg_id());
                match &**n1 {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                }
                match &**n2 {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                }
            }
            Instruction::TailCall(x, args, _) => {
                match &**x {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                };
                for arg in args.iter() {
                    set.insert(arg.any_reg_id());
                }
            }
            Instruction::Select(x, n1, n2) => {
                set.insert(x.any_reg_id());
                match &**n1 {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                }
                match &**n2 {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                }
            }
            Instruction::Load(_, ptr, offset, _) => {
                match &**offset {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                };
                set.insert(ptr.any_reg_id());
            }
            Instruction::Store(ptr, offset, value, _) => {
                match &**offset {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                };
                match &**ptr {
                    Node::Operand(Operand::VirtualRegister(r, _))
                    | Node::Operand(Operand::Register(r, _)) => {
                        set.insert(*r);
                    }
                    _ => (),
                };
                set.insert(value.any_reg_id());
            }
            Instruction::Move(_, x) => {
                if let Node::Operand(Operand::Register(_, _)) = &**x {
                    set.insert(x.any_reg_id());
                }
                //set.insert(x.any_reg_id());
            }
            Instruction::Reinterpret(_, x, _) | Instruction::Cast(_, x, _) => {
                set.insert(x.any_reg_id());
            }
            Instruction::Return(x) => {
                set.insert(x.any_reg_id());
            }
            Instruction::IntBinary(_, x, y, z) | Instruction::FloatBinary(_, x, y, z) => {
                if let Node::Operand(Operand::Register { .. }) = &**y {
                    set.insert(y.any_reg_id());
                }
                if let Node::Operand(Operand::Register { .. }) = &**z {
                    set.insert(z.any_reg_id());
                }
                //set.insert(x.any_reg_id());
            }
            _ => (),
        }
        set.iter().copied().collect()
    }
}

pub const UINT8_TYPE: Type = Type::UInt8;
pub const UINT16_TYPE: Type = Type::UInt16;
pub const UINT32_TYPE: Type = Type::UInt32;
pub const UINT64_TYPE: Type = Type::UInt64;
// IDs reserved for machine registers
pub const MACHINE_ID_START: usize = 0;
pub const MACHINE_ID_END: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSignature {
    pub ret: Type,
    pub params: Vec<Type>,
    pub name: String,
}

impl FunctionSignature {
    pub fn new(name: &str, t: &[Type], r: Type) -> Self {
        Self {
            ret: r,
            params: t.to_owned(),
            name: name.to_owned(),
        }
    }
}

pub struct LIRFunctionBuilder {
    func: LIRFunction,
    vregs: usize,
    params: Vec<Type>,
    ret: Type,
    variables: LinkedHashMap<String, Box<Node>>,
    current_block: usize,
}

impl LIRFunctionBuilder {
    fn reg(&mut self) -> usize {
        let r = self.vregs;
        self.vregs += 1;
        r
    }

    pub fn real_gpr_reg(&self, r: usize) -> Box<Node> {
        Box::new((*crate::codegen::ALL_GPRS[r]).clone())
    }

    pub fn new(name: &str, params: &[Type], ret: Type) -> Self {
        let mut this = Self {
            variables: LinkedHashMap::new(),
            func: LIRFunction::new(FunctionSignature {
                params: params.clone().to_vec(),
                ret,
                name: name.to_owned(),
            }),
            vregs: MACHINE_ID_END,
            ret,
            params: params.to_vec(),
            current_block: 0,
        };

        this.func.code.push(BasicBlock::new(0, vec![]));
        this.func.init_machine_regs_for_func();
        this
    }
    fn new_virtual(&mut self, ty: Type) -> Box<Node> {
        let val = Box::new(Node::Operand(Operand::Register(self.reg(), ty)));
        self.func.values.insert(val.any_reg_id(), val.clone());
        val
    }

    fn param_ty(&self, t: usize) -> Type {
        self.params[t]
    }
    fn emit(&mut self, ins: Instruction) {
        self.func.code[self.current_block].instructions.push(ins);
    }

    pub fn call(&mut self, name: &str, args: &[Box<Node>]) -> Box<Node> {
        let sig = self.func.functions.get(name).unwrap().clone();
        let r = self.new_virtual(sig.ret);
        let value = Box::new(Node::Operand(Operand::Symbol(name.to_owned())));
        self.emit(Instruction::Call(
            r.clone(),
            value,
            args.to_owned(),
            sig.clone(),
        ));
        r
    }

    pub fn add_function(&mut self, sig: FunctionSignature) {
        self.func.functions.insert(sig.name.clone(), sig);
    }
    pub fn ty_info(r: &Node) -> Type {
        match r {
            Node::Operand(Operand::Register(_, t)) => *t,
            _ => unreachable!(),
        }
    }
    pub fn load_param(&mut self, x: usize) -> Box<Node> {
        if x >= ARGUMENT_GPRS.len() {
            panic!("unsuported argument count");
        }
        let r = self.new_virtual(self.param_ty(x));
        self.emit(Instruction::LoadParam(r.clone(), x, self.param_ty(x)));
        r
    }
    pub fn int_binary(&mut self, op: IntBinaryOperation, x: Box<Node>, y: Box<Node>) -> Box<Node> {
        let r = self.new_virtual(Self::ty_info(&*x));
        self.emit(Instruction::IntBinary(op, r.clone(), x, y));
        r
    }

    pub fn load_imm64(&mut self, x: i64) -> Box<Node> {
        let r = self.new_virtual(Type::Int64);
        self.emit(Instruction::LoadImm(
            r.clone(),
            Box::new(Node::Operand(Operand::Immediate64(x))),
            Type::Int64,
        ));
        r
    }
    pub fn load_imm32(&mut self, x: i32) -> Box<Node> {
        let r = self.new_virtual(Type::Int32);
        self.emit(Instruction::LoadImm(
            r.clone(),
            Box::new(Node::Operand(Operand::Immediate32(x))),
            Type::Int32,
        ));
        r
    }
    pub fn return_(&mut self, val: Box<Node>) {
        self.emit(Instruction::Return(val));
    }

    pub fn finish(mut self) -> LIRFunction {
        self.func
    }
}

impl Type {
    pub fn int_length(&self) -> usize {
        self.size() * 8
    }
    pub fn size(&self) -> usize {
        match self {
            Type::Float32 | Type::Int32 | Type::UInt32 => std::mem::size_of::<u32>(),
            Type::Int64 | Type::Float64 | Type::UInt64 => std::mem::size_of::<u64>(),
            Type::Int8 | Type::UInt8 => std::mem::size_of::<u8>(),
            Type::Int16 | Type::UInt16 => std::mem::size_of::<u16>(),
        }
    }

    pub fn align(&self) -> usize {
        match self {
            Type::Float32 | Type::Int32 | Type::UInt32 => std::mem::align_of::<u32>(),
            Type::Int64 | Type::Float64 | Type::UInt64 => std::mem::align_of::<u64>(),
            Type::Int8 | Type::UInt8 => std::mem::align_of::<u8>(),
            Type::Int16 | Type::UInt16 => std::mem::align_of::<u16>(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Display)]
pub enum MemoryLocation {
    /// addr = base + offset + index * scale
    #[display(fmt = "[{} + offset + index * scale]", base)]
    Address {
        base: Box<Node>, // +8
        offset: Option<Box<Node>>,
        index: Option<Box<Node>>,
        scale: Option<u8>,
    },
    /// addr = base + label(offset)
    #[display(fmt = "[base + label]")]
    Symbolic {
        base: Option<Box<Node>>,
        label: String,
        is_global: bool,
        is_native: bool,
    },
}
