pub mod callconv;
pub mod lower_function;
use super::*;
use crate::ir::*;
use hashlink::LinkedHashMap;
use std::collections::HashMap;
use std::sync::Arc as Rc;
// number of normal callee saved registers (excluding RSP and RBP)
pub const CALLEE_SAVED_COUNT: usize = 5;

/// a macro to declare a general purpose register
macro_rules! GPR {
    ($id:expr,  $ty: ident) => {{
        Rc::new(Node::Operand(Operand::Register($id, $ty)))
    }};
}

/// a macro to declare a floating point register
macro_rules! FPR {
    ($id:expr,$name: literal) => {{
        Rc::new(Node::Operand(Operand::Register($id, Type::Float64)))
    }};
}

macro_rules! GPR_ALIAS {
    ($alias: ident: ($id64: expr, $r64: ident) ->
     $r32: ident, $r16: ident, $r8l: ident, $r8h: ident) => {
        lazy_static::lazy_static! {
            pub static ref $r64: Rc<Node> = GPR!($id64, UINT64_TYPE);
            pub static ref $r32: Rc<Node> = GPR!($id64 + 1, UINT32_TYPE);
            pub static ref $r16: Rc<Node> = GPR!($id64 + 2, UINT16_TYPE);
            pub static ref $r8l: Rc<Node> = GPR!($id64 + 3, UINT8_TYPE);
            pub static ref $r8h: Rc<Node> = GPR!($id64 + 4, UINT8_TYPE);
            pub static ref $alias: [Rc<Node>; 5] = [
                $r64.clone(),
                $r32.clone(),
                $r16.clone(),
                $r8l.clone(),
                $r8h.clone()
            ];
        }
    };

    ($alias: ident: ($id64: expr, $r64: ident) -> $r32: ident, $r16: ident, $r8: ident) => {
        lazy_static::lazy_static! {
            pub static ref $r64: Rc<Node> = GPR!($id64, UINT64_TYPE);
            pub static ref $r32: Rc<Node> = GPR!($id64 + 1, UINT32_TYPE);
            pub static ref $r16: Rc<Node> = GPR!($id64 + 2, UINT16_TYPE);
            pub static ref $r8: Rc<Node> = GPR!($id64 + 3, UINT8_TYPE);
            pub static ref $alias: [Rc<Node>; 4] =
                [$r64.clone(), $r32.clone(), $r16.clone(), $r8.clone()];
        }
    };

    ($alias: ident: ($id64: expr, $r64: ident)) => {
        lazy_static::lazy_static! {
            pub static ref $r64: Rc<Node> = GPR!($id64, UINT64_TYPE);
            pub static ref $alias: [Rc<Node>; 4] =
                [$r64.clone(), $r64.clone(), $r64.clone(), $r64.clone()];
        }
    };
}

// declare all general purpose registers for x86_64
// non 64-bit registers are alias of its 64-bit one

GPR_ALIAS!(RAX_ALIAS: (0, RAX)  -> EAX, AX , AL, AH);
GPR_ALIAS!(RCX_ALIAS: (5, RCX)  -> ECX, CX , CL, CH);
GPR_ALIAS!(RDX_ALIAS: (10,RDX)  -> EDX, DX , DL, DH);
GPR_ALIAS!(RBX_ALIAS: (15,RBX)  -> EBX, BX , BL, BH);
GPR_ALIAS!(RSP_ALIAS: (20,RSP)  -> ESP, SP , SPL);
GPR_ALIAS!(RBP_ALIAS: (24,RBP)  -> EBP, BP , BPL);
GPR_ALIAS!(RSI_ALIAS: (28,RSI)  -> ESI, SI , SIL);
GPR_ALIAS!(RDI_ALIAS: (32,RDI)  -> EDI, DI , DIL);
GPR_ALIAS!(R8_ALIAS : (36,R8 )  -> R8D, R8W, R8B);
GPR_ALIAS!(R9_ALIAS : (40,R9 )  -> R9D, R9W, R9B);
GPR_ALIAS!(R10_ALIAS: (44,R10) -> R10D,R10W,R10B);
GPR_ALIAS!(R11_ALIAS: (48,R11) -> R11D,R11W,R11B);
GPR_ALIAS!(R12_ALIAS: (52,R12) -> R12D,R12W,R12B);
GPR_ALIAS!(R13_ALIAS: (56,R13) -> R13D,R13W,R13B);
GPR_ALIAS!(R14_ALIAS: (60,R14) -> R14D,R14W,R14B);
GPR_ALIAS!(R15_ALIAS: (64,R15) -> R15D,R15W,R15B);
GPR_ALIAS!(RIP_ALIAS: (68,RIP));

lazy_static::lazy_static! {
    /// a map from 64-bit register IDs to a vector of its aliased register (Values),
    /// including the 64-bit register
    pub static ref GPR_ALIAS_TABLE : LinkedHashMap<usize, Vec<Rc<Node>>> = {
        let mut ret = LinkedHashMap::new();

        ret.insert(RAX.any_reg_id(), RAX_ALIAS.to_vec());
        ret.insert(RCX.any_reg_id(), RCX_ALIAS.to_vec());
        ret.insert(RDX.any_reg_id(), RDX_ALIAS.to_vec());
        ret.insert(RBX.any_reg_id(), RBX_ALIAS.to_vec());
        ret.insert(RSP.any_reg_id(), RSP_ALIAS.to_vec());
        ret.insert(RBP.any_reg_id(), RBP_ALIAS.to_vec());
        ret.insert(RSI.any_reg_id(), RSI_ALIAS.to_vec());
        ret.insert(RDI.any_reg_id(), RDI_ALIAS.to_vec());
        ret.insert(R8.any_reg_id() , R8_ALIAS.to_vec() );
        ret.insert(R9.any_reg_id() , R9_ALIAS.to_vec() );
        ret.insert(R10.any_reg_id(), R10_ALIAS.to_vec());
        ret.insert(R11.any_reg_id(), R11_ALIAS.to_vec());
        ret.insert(R12.any_reg_id(), R12_ALIAS.to_vec());
        ret.insert(R13.any_reg_id(), R13_ALIAS.to_vec());
        ret.insert(R14.any_reg_id(), R14_ALIAS.to_vec());
        ret.insert(R15.any_reg_id(), R15_ALIAS.to_vec());
        ret.insert(RIP.any_reg_id(), RIP_ALIAS.to_vec());

        ret
    };

    /// a map from any register to its 64-bit alias
    pub static ref GPR_ALIAS_LOOKUP : HashMap<usize, Rc<Node>> = {
        let mut ret = HashMap::new();

        for vec in GPR_ALIAS_TABLE.values() {
            let colorable = vec[0].clone();

            for gpr in vec {
                ret.insert(gpr.any_reg_id(), colorable.clone());
            }
        }

        ret
    };
}
use lazy_static::lazy_static;

lazy_static! {
    /// GPRs for returning values
    //  order matters
    pub static ref RETURN_GPRS : [Rc<Node>; 2] = [
        RAX.clone(),
        RDX.clone(),
    ];

    /// GPRs for passing arguments
    //  order matters
    pub static ref ARGUMENT_GPRS : [Rc<Node>; 6] = [
        RDI.clone(),
        RSI.clone(),
        RDX.clone(),
        RCX.clone(),
        R8.clone(),
        R9.clone()
    ];

    /// callee saved GPRs
    pub static ref CALLEE_SAVED_GPRS : [Rc<Node>; 6] = [
        RBX.clone(),
        RBP.clone(),
        R12.clone(),
        R13.clone(),
        R14.clone(),
        R15.clone()
    ];

    /// caller saved GPRs
    pub static ref CALLER_SAVED_GPRS : [Rc<Node>; 9] = [
        RAX.clone(),
        RCX.clone(),
        RDX.clone(),
        RSI.clone(),
        RDI.clone(),
        R8.clone(),
        R9.clone(),
        R10.clone(),
        R11.clone()
    ];

    /// all the genral purpose registers
    //  FIXME: why RBP is commented out?
    pub static ref ALL_GPRS : [Rc<Node>; 15] = [
        RAX.clone(),
        RCX.clone(),
        RDX.clone(),
        RBX.clone(),
        RSP.clone(),
//        RBP.clone(),
        RSI.clone(),
        RDI.clone(),
        R8.clone(),
        R9.clone(),
        R10.clone(),
        R11.clone(),
        R12.clone(),
        R13.clone(),
        R14.clone(),
        R15.clone()
    ];
}

pub const FPR_ID_START: usize = 100;

lazy_static! {
    // SSE registers
    pub static ref XMM0  : Rc<Node> = FPR!(FPR_ID_START,    "xmm0");
    pub static ref XMM1  : Rc<Node> = FPR!(FPR_ID_START + 1,"xmm1");
    pub static ref XMM2  : Rc<Node> = FPR!(FPR_ID_START + 2,"xmm2");
    pub static ref XMM3  : Rc<Node> = FPR!(FPR_ID_START + 3,"xmm3");
    pub static ref XMM4  : Rc<Node> = FPR!(FPR_ID_START + 4,"xmm4");
    pub static ref XMM5  : Rc<Node> = FPR!(FPR_ID_START + 5,"xmm5");
    pub static ref XMM6  : Rc<Node> = FPR!(FPR_ID_START + 6,"xmm6");
    pub static ref XMM7  : Rc<Node> = FPR!(FPR_ID_START + 7,"xmm7");
    pub static ref XMM8  : Rc<Node> = FPR!(FPR_ID_START + 8,"xmm8");
    pub static ref XMM9  : Rc<Node> = FPR!(FPR_ID_START + 9,"xmm9");
    pub static ref XMM10 : Rc<Node> = FPR!(FPR_ID_START + 10,"xmm10");
    pub static ref XMM11 : Rc<Node> = FPR!(FPR_ID_START + 11,"xmm11");
    pub static ref XMM12 : Rc<Node> = FPR!(FPR_ID_START + 12,"xmm12");
    pub static ref XMM13 : Rc<Node> = FPR!(FPR_ID_START + 13,"xmm13");
    pub static ref XMM14 : Rc<Node> = FPR!(FPR_ID_START + 14,"xmm14");
    pub static ref XMM15 : Rc<Node> = FPR!(FPR_ID_START + 15,"xmm15");

    /// FPRs to return values
    pub static ref RETURN_FPRS : [Rc<Node>; 2] = [
        XMM0.clone(),
        XMM1.clone()
    ];

    /// FPRs to pass arguments
    //  order matters
    pub static ref ARGUMENT_FPRS : [Rc<Node>; 8] = [
        XMM0.clone(),
        XMM1.clone(),
        XMM2.clone(),
        XMM3.clone(),
        XMM4.clone(),
        XMM5.clone(),
        XMM6.clone(),
        XMM7.clone()
    ];

    /// callee saved FPRs (none for x86_64)
    pub static ref CALLEE_SAVED_FPRS : [Rc<Node>; 0] = [];

    /// caller saved FPRs
    pub static ref CALLER_SAVED_FPRS : [Rc<Node>; 16] = [
        XMM0.clone(),
        XMM1.clone(),
        XMM2.clone(),
        XMM3.clone(),
        XMM4.clone(),
        XMM5.clone(),
        XMM6.clone(),
        XMM7.clone(),
        XMM8.clone(),
        XMM9.clone(),
        XMM10.clone(),
        XMM11.clone(),
        XMM12.clone(),
        XMM13.clone(),
        XMM14.clone(),
        XMM15.clone(),
    ];

    /// all the floating point registers
    static ref ALL_FPRS : [Rc<Node>; 16] = [
        XMM0.clone(),
        XMM1.clone(),
        XMM2.clone(),
        XMM3.clone(),
        XMM4.clone(),
        XMM5.clone(),
        XMM6.clone(),
        XMM7.clone(),
        XMM8.clone(),
        XMM9.clone(),
        XMM10.clone(),
        XMM11.clone(),
        XMM12.clone(),
        XMM13.clone(),
        XMM14.clone(),
        XMM15.clone(),
    ];
}

lazy_static! {
    /// a map for all the machine registers, from ID to Rc<Node>
    pub static ref ALL_MACHINE_REGS : LinkedHashMap<usize, Rc<Node>> = {
        let mut map = LinkedHashMap::new();

        for vec in GPR_ALIAS_TABLE.values() {
            for reg in vec {
                map.insert(reg.any_reg_id(), reg.clone());
            }
        }

        map.insert(XMM0.any_reg_id(), XMM0.clone());
        map.insert(XMM1.any_reg_id(), XMM1.clone());
        map.insert(XMM2.any_reg_id(), XMM2.clone());
        map.insert(XMM3.any_reg_id(), XMM3.clone());
        map.insert(XMM4.any_reg_id(), XMM4.clone());
        map.insert(XMM5.any_reg_id(), XMM5.clone());
        map.insert(XMM6.any_reg_id(), XMM6.clone());
        map.insert(XMM7.any_reg_id(), XMM7.clone());
        map.insert(XMM8.any_reg_id(), XMM8.clone());
        map.insert(XMM9.any_reg_id(), XMM9.clone());
        map.insert(XMM10.any_reg_id(), XMM10.clone());
        map.insert(XMM11.any_reg_id(), XMM11.clone());
        map.insert(XMM12.any_reg_id(), XMM12.clone());
        map.insert(XMM13.any_reg_id(), XMM13.clone());
        map.insert(XMM14.any_reg_id(), XMM14.clone());
        map.insert(XMM15.any_reg_id(), XMM15.clone());

        map
    };

    /// all the usable general purpose registers for reg allocator to assign
    //  order matters here (since register allocator will prioritize assigning temporaries
    //  to a register that appears early)
    //  we put caller saved regs first (they imposes no overhead if there is no call instruction)
    pub static ref ALL_USABLE_GPRS : Vec<Rc<Node>> = vec![
        // caller saved registers
        RAX.clone(),
        RCX.clone(),
        RDX.clone(),
        RSI.clone(),
        RDI.clone(),
        R8.clone(),
        R9.clone(),
        R10.clone(),
        R11.clone(),
        // callee saved registers
        RBX.clone(),
        R12.clone(),
        R13.clone(),
        R14.clone(),
        R15.clone(),
    ];

    /// all the usable floating point registers for reg allocator to assign
    //  order matters here (since register allocator will prioritize assigning temporaries
    //  to a register that appears early)
    //  we put caller saved regs first (they imposes no overhead if there is no call instruction)
    pub static ref ALL_USABLE_FPRS : Vec<Rc<Node>> = vec![
        // floating point registers
        XMM0.clone(),
        XMM1.clone(),
        XMM2.clone(),
        XMM3.clone(),
        XMM4.clone(),
        XMM5.clone(),
        XMM6.clone(),
        XMM7.clone(),
        XMM8.clone(),
        XMM9.clone(),
        XMM10.clone(),
        XMM11.clone(),
        XMM12.clone(),
        XMM13.clone(),
        XMM14.clone(),
        XMM15.clone()
    ];

    /// all the usable registers for register allocators to assign
    //  order matters here (since register allocator will prioritize assigning temporaries
    //  to a register that appears early)
    //  we put caller saved regs first (they imposes no overhead if there is no call instruction)
    pub static ref ALL_USABLE_MACHINE_REGS : Vec<Rc<Node>> = {
        let mut ret = vec![];
        ret.extend_from_slice(&ALL_USABLE_GPRS);
        ret.extend_from_slice(&ALL_USABLE_FPRS);
        ret
    };

    /// all the caller saved registers
    pub static ref ALL_CALLER_SAVED_REGS : Vec<Rc<Node>> = {
        let mut ret = vec![];
        for r in CALLER_SAVED_GPRS.iter() {
            ret.push(r.clone());
        }
        for r in CALLER_SAVED_FPRS.iter() {
            ret.push(r.clone());
        }
        ret
    };
}

/// returns the number of all registers on this platform
pub fn number_of_all_regs() -> usize {
    ALL_MACHINE_REGS.len()
}

/// returns a reference to a map for all the registers
pub fn all_regs() -> &'static LinkedHashMap<usize, Rc<Node>> {
    &ALL_MACHINE_REGS
}
/// returns a reference to a vector of all usable registers
pub fn all_usable_regs() -> &'static Vec<Rc<Node>> {
    &ALL_USABLE_MACHINE_REGS
}

/// gets the number of registers in a certain register group
pub fn number_of_usable_regs_in_group(group: RegGroup) -> usize {
    match group {
        RegGroup::GPR => ALL_USABLE_GPRS.len(),
        RegGroup::GPREX => ALL_USABLE_GPRS.len(),
        RegGroup::FPR => ALL_USABLE_FPRS.len(),
    }
}
/// returns RegGroup for a given machine register (by ID)
/// panics if the ID is not a machine register
pub fn pick_group_for_reg(reg_id: usize) -> RegGroup {
    let reg = all_regs().get(&reg_id).unwrap();
    RegGroup::from_node(reg).unwrap()
}
/// returns offset of callee saved register
/// Reg should be a 64-bit callee saved GPR or FPR
pub fn get_callee_saved_offset(reg: usize) -> isize {
    debug_assert!(is_callee_saved(reg) && reg != RBP.any_reg_id());

    let id = if reg == RBX.any_reg_id() {
        0
    } else {
        (reg - R12.any_reg_id()) / 4 + 1
    };
    (id as isize + 1) * (-8)
}

/// is a machine register (by ID) callee saved?
/// returns false if the ID is not a machine register
pub fn is_callee_saved(reg_id: usize) -> bool {
    for reg in CALLEE_SAVED_GPRS.iter() {
        if reg_id == reg.any_reg_id() {
            return true;
        }
    }

    false
}

/// gets the color for a machine register (returns 64-bit alias for it)
pub fn get_color_for_precolored(id: usize) -> usize {
    debug_assert!(id < MACHINE_ID_END);

    if id < FPR_ID_START {
        match GPR_ALIAS_LOOKUP.get(&id) {
            Some(val) => val.any_reg_id(),
            None => panic!("cannot find GPR {}", id),
        }
    } else {
        // we do not have alias for FPRs
        id
    }
}

/// returns P<Value> for a register ID of its alias of the given length
/// panics if the ID is not a machine register ID
pub fn get_alias_for_length(id: usize, length: usize) -> Rc<Node> {
    if id < FPR_ID_START {
        let vec = match GPR_ALIAS_TABLE.get(&id) {
            Some(vec) => vec,
            None => panic!("didnt find {} as GPR", id),
        };

        match length {
            64 => vec[0].clone(),
            32 => vec[1].clone(),
            16 => vec[2].clone(),
            8 => vec[3].clone(),
            1 => vec[3].clone(),
            _ => panic!("unexpected length {} for {}", length, vec[0]),
        }
    } else {
        for r in ALL_FPRS.iter() {
            if r.any_reg_id() == id {
                return r.clone();
            }
        }

        panic!("didnt find {} as FPR", id)
    }
}
