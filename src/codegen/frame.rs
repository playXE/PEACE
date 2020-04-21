use super::*;
use crate::ir::*;
use std::collections::HashMap;

/// Frame serves one purpose right now:
/// * it manages stack allocation that are known statically (such as callee
///   saved, spilled registers)
/// PEACE frame layout is compatible with C ABI
/// on x64
/// | previous frame ...
/// |---------------
/// | return address
/// | old RBP        <- RBP
/// | callee saved
/// | spilled
/// |---------------
/// | alloca area (not implemented)

#[derive(Clone)]
pub struct Frame {
    pub cur_offset: isize,
    /// arguments passed to this function by registers (used for validating
    /// register allocation)
    pub argument_by_reg: HashMap<usize, Box<Node>>,
    /// arguments passed to this function by stack (used for validating
    /// register allocation)
    pub argument_by_stack: HashMap<usize, Box<Node>>,
    /// allocated frame location for Mu Values
    pub allocated: HashMap<usize, FrameSlot>,
    pub callee_saved: HashMap<isize, isize>,
}

impl Frame {
    /// creates a new Frame
    pub fn new() -> Frame {
        Frame {
            cur_offset: 0,
            argument_by_reg: HashMap::new(),
            argument_by_stack: HashMap::new(),
            callee_saved: HashMap::new(),
            allocated: HashMap::new(),
        }
    }
    /// returns current size,
    /// which is always a multiple of 16 bytes for x64/aarch64 (alignment
    /// requirement)
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn cur_size(&self) -> usize {
        // frame size is a multiple of 16 bytes
        let size = self.cur_offset.abs() as usize;

        // align size to a multiple of 16 bytes
        let size = (size + 16 - 1) & !(16 - 1);

        debug_assert!(size % 16 == 0);

        size
    }
    /// adds a record of a Mu value argument passed in a certain register
    pub fn add_argument_by_reg(&mut self, temp: usize, reg: Box<Node>) {
        self.argument_by_reg.insert(temp, reg);
    }

    /// adds a record of a Mu value argumetn passed on stack
    pub fn add_argument_by_stack(&mut self, temp: usize, stack_slot: Box<Node>) {
        self.argument_by_stack.insert(temp, stack_slot);
    }

    /// allocates next stack slot for a callee saved register, and returns
    /// a memory operand representing the stack slot
    pub fn alloc_slot_for_callee_saved_reg(&mut self, reg: Box<Node>) -> Box<Node> {
        let (mem, off) = {
            let slot = self.alloc_slot(&reg);
            (
                slot.make_memory_op(LIRFunctionBuilder::ty_info(&*reg)),
                slot.offset,
            )
        };
        let o = get_callee_saved_offset(reg.any_reg_id());
        self.callee_saved.insert(o, off);
        mem
    }

    /// removes the record for a callee saved register
    /// We allocate stack slots for all the callee saved regsiter, and later
    /// remove slots for those registers that are not actually used
    pub fn remove_record_for_callee_saved_reg(&mut self, reg: usize) {
        self.allocated.remove(&reg);
        let id = get_callee_saved_offset(reg);
        self.callee_saved.remove(&id);
    }

    /// allocates next stack slot for a spilled register, and returns
    /// a memory operand representing the stack slot
    pub fn alloc_slot_for_spilling(&mut self, reg: Box<Node>) -> Box<Node> {
        let slot = self.alloc_slot(&reg);
        slot.make_memory_op(LIRFunctionBuilder::ty_info(&*reg))
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn alloc_slot(&mut self, val: &Box<Node>) -> &FrameSlot {
        // base pointer is 16 bytes aligned, we are offsetting from base pointer
        // every value should be properly aligned

        let backendty = LIRFunctionBuilder::ty_info(val);
        // asserting that the alignment is no larger than 16 bytes, otherwise
        // we need to adjust offset in a different way
        if backendty.align() > 16 {
            if cfg!(target_arch = "aarch64") || cfg!(target_arch = "x86_64") {
                panic!("A type cannot have alignment greater than 16 on aarch64")
            } else {
                unimplemented!()
            }
        }

        self.cur_offset -= backendty.size() as isize;

        {
            // if alignment doesnt satisfy, make adjustment
            let abs_offset = self.cur_offset.abs() as usize;
            if abs_offset % backendty.align() != 0 {
                use crate::util::math;
                let abs_offset = math::align_up(abs_offset, backendty.align());

                self.cur_offset = -(abs_offset as isize);
            }
        }

        let id = val.any_reg_id();
        let ret = FrameSlot {
            offset: self.cur_offset,
            value: val.clone(),
        };

        self.allocated.insert(id, ret);
        self.allocated.get(&id).unwrap()
    }
}
/// FrameSlot presents a Value stored in a certain frame location
#[derive(Clone)]
pub struct FrameSlot {
    /// location offset from current base pointer
    pub offset: isize,
    /// Mu value that resides in this location
    pub value: Box<Node>,
}

impl FrameSlot {
    /// generates a memory operand for this frame slot
    #[cfg(target_arch = "x86_64")]
    pub fn make_memory_op(&self, ty: Type) -> Box<Node> {
        Box::new(Node::Operand(Operand::Memory(MemoryLocation::Address {
            base: Box::new((**crate::codegen::x86_64::RBP).clone()),
            offset: Some(Box::new(Node::Operand(Operand::Immediate32(
                self.offset as i32,
            )))),
            index: None,
            scale: None,
        })))
    }
    /// generates a memory operand for this frame slot
    #[cfg(target_arch = "aarch64")]
    pub fn make_memory_op(&self, ty: P<MuType>, vm: &VM) -> P<Value> {
        use compiler::backend::aarch64;

        P(Value {
            hdr: MuEntityHeader::unnamed(vm.next_id()),
            ty: ty.clone(),
            v: Value_::Memory(MemoryLocation::VirtualAddress {
                base: aarch64::FP.clone(),
                offset: Some(Value::make_int32_const(vm.next_id(), self.offset as u64)),
                scale: 1,
                signed: true,
            }),
        })
    }
}
