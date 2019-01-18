use self::compiler::prelude::*;
use crate::compiler;
use crate::dfg::ValueData;
use crate::module::Linkage;
use crate::types::*;
use crate::utils::align;
use crate::EntityRef;
use crate::{Value, Variable};

#[cfg(target_family = "windows")]
pub const ARG_GPR: [Register; 4] = [RCX, RDX, R8, R9];
#[cfg(target_family = "windows")]
pub const ARG_FPR: [Register; 4] = [XMM0, XMM1, XMM2, XMM3];

#[cfg(target_family = "unix")]
pub const ARG_GPR: [Register; 6] = [RDI, RSI, RDX, RCX, R8, R9];
#[cfg(target_family = "unix")]
pub const ARG_FPR: [XMMRegister; 8] = [XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reloc {
    pub global_name: String,
    pub at: usize,
    pub to: usize,
}

use fnv::{FnvHashMap, FnvHashSet};

#[derive(Clone)]
pub struct Function {
    pub name: String,
    asm: Assembler,
    pub relocs: Vec<Reloc>,
    epilogs: Vec<Reloc>,
    prolog: Reloc,
    pub linkage: Linkage,
    pub values: FnvHashMap<Value, ValueData>,
    pub value_types: FnvHashMap<Value, Type>,
    pub variables: FnvHashMap<Variable, i32>,
    pub variable_types: FnvHashMap<Variable, Type>,
    used_registers: FnvHashSet<Reg>,
    value_id: usize,
    stack_offset: i32,
}

impl Function {
    pub fn new(name: &str, linkage: Linkage) -> Function {
        Self {
            name: name.to_owned(),
            asm: Assembler::new(),
            linkage: linkage,
            relocs: vec![],
            epilogs: vec![],
            prolog: Reloc {
                global_name: "<prolog>".into(),
                at: 0,
                to: 0,
            },
            values: FnvHashMap::default(),
            value_types: FnvHashMap::default(),
            variables: FnvHashMap::default(),
            variable_types: FnvHashMap::default(),
            used_registers: FnvHashSet::default(),
            stack_offset: 0,
            value_id: 0,
        }
    }

    pub fn get_value_type(&self, x: Value) -> Type {
        *self
            .value_types
            .get(&x)
            .expect(&format!("value {:?} not defined", x))
    }

    pub fn get_value_loc(&self, x: Value) -> ValueData {
        *self
            .values
            .get(&x)
            .expect(&format!("value {:?} not defined", x))
    }

    pub fn get_data(&self) -> &[u8] {
        &self.asm.data()
    }

    pub fn asm_mut<'r>(&'r mut self) -> &'r mut Assembler {
        &mut self.asm
    }

    pub fn prolog(&mut self) {
        emit_pushq_reg(&mut self.asm, RBP);
        emit_mov_reg_reg(&mut self.asm, 1, RSP, RBP);
        /*self.asm.emit(0x48);
        self.asm.emit(0x81);
        self.asm.emit(0xec);
        self.asm.emit32(0);
        self.prolog = Reloc {
            global_name: "<prolog>".into(),
            at: self.asm.pos() - 4,
            to: self.asm.pos(),
        };*/
    }

    pub fn epilog(&mut self) {
        //emit_addq_imm_reg(&mut self.asm, self.stack_offset, RSP);
        //emit_mov_reg_reg(&mut self.asm, 1,RBP,RSP);
        emit_popq_reg(&mut self.asm, RBP);
        emit_retq(&mut self.asm);
    }

    pub fn allocate_in_stack(&mut self, ty: Type) -> i32 {
        let size = ty.to_machine().size();
        let offset = align(self.stack_offset + size as i32, size as i32);
        self.stack_offset = offset;
        offset
    }

    fn free(&mut self, v: Value) {
        let loc = self.get_value_loc(v);
        match loc {
            ValueData::Gpr(reg) => {
                self.used_registers.remove(&Reg::Gpr(reg));
            }
            ValueData::Fpr(reg) => {
                self.used_registers.remove(&Reg::Float(reg));
            }
            _ => {}
        };
        self.values.remove(&v);
        self.value_types.remove(&v);
    }

    fn allocate_reg(&mut self, ty: Type) -> ValueData {
        #[cfg(windows)]
        const AVAIL_GPR: [Register; 7] = [RBX, RSI, RDI, R12, R13, R14, R15];
        #[cfg(not(windows))]
        const AVAIL_GPR: [Register; 5] = [RBX, R12, R13, R14, R15];

        #[cfg(windows)]
        const AVAIL_FPR: [XMMRegister; 10] =
            [XMM8, XMM9, XMM10, XMM11, XMM12, XMM13, XMM14, XMM15, XMM16];
        #[cfg(not(windows))]
        const AVAIL_FPR: [XMMRegister; 6] = [XMM10, XMM11, XMM12, XMM13, XMM14, XMM15];

        if !ty.is_float() {
            for reg in AVAIL_GPR.iter() {
                if !self.used_registers.contains(&Reg::Gpr(*reg)) {
                    self.used_registers.insert(Reg::Gpr(*reg));

                    return ValueData::Gpr(*reg);
                }
            }
            let off = self.allocate_in_stack(ty);
            ValueData::Stack(-off)
        } else {
            for reg in AVAIL_FPR.iter() {
                if !self.used_registers.contains(&Reg::Float(*reg)) {
                    self.used_registers.insert(Reg::Float(*reg));
                    return ValueData::Fpr(*reg);
                }
            }
            let off = self.allocate_in_stack(ty);
            ValueData::Stack(-off)
        }
    }

    pub fn iconst(&mut self, ty: Type, imm: impl Into<i64>) -> Value {
        assert!(!ty.is_float());
        let value = Value::new(self.value_id);
        self.value_id += 1;
        let loc = self.allocate_reg(ty);

        if loc.is_gpr() {
            let imm = imm.into();
            if imm == 0 {
                emit_xor_reg_reg(&mut self.asm, 1, loc.gpr(), loc.gpr());
            } else {
                self.asm.load_int_const(ty.to_machine(), loc.gpr(), imm);
            }
        } else {
            self.asm.load_int_const(ty.to_machine(), RAX, imm.into());
            self.asm
                .store_mem(ty.to_machine(), Mem::Local(loc.off()), Reg::Gpr(RAX));
        }
        self.values.insert(value, loc);
        self.value_types.insert(value, ty);
        value
    }

    fn bin_int(
        &mut self,
        x: Value,
        y: Value,
        f: &Fn(&mut Assembler, MachineMode, Register, Register, Register),
    ) -> Value {
        let value = Value::new(self.value_id);
        self.value_id += 1;
        let ty = self.get_value_type(x);
        let x_loc = self.get_value_loc(x);
        let y_loc = self.get_value_loc(y);
        self.free(x);
        self.free(y);
        let loc = self.allocate_reg(ty);

        self.values.insert(value, loc);
        self.value_types.insert(value, ty);

        if x_loc.is_gpr() && y_loc.is_gpr() {
            if loc.is_gpr() {
                f(
                    &mut self.asm,
                    ty.to_machine(),
                    loc.gpr(),
                    x_loc.gpr(),
                    y_loc.gpr(),
                );
            } else {
                f(
                    &mut self.asm,
                    ty.to_machine(),
                    RAX,
                    x_loc.gpr(),
                    y_loc.gpr(),
                );
                self.asm
                    .store_mem(ty.to_machine(), Mem::Local(loc.off()), Reg::Gpr(RAX));
            }
        } else {
            if x_loc.is_gpr() {
                self.asm
                    .load_mem(ty.to_machine(), Reg::Gpr(RAX), Mem::Local(y_loc.off()));
                f(&mut self.asm, ty.to_machine(), RAX, x_loc.gpr(), RAX);
                if loc.is_gpr() {
                    emit_mov_reg_reg(&mut self.asm, ty.x64(), RAX, loc.gpr());
                } else {
                    self.asm
                        .store_mem(ty.to_machine(), Mem::Local(loc.off()), Reg::Gpr(RAX));
                }
            } else {
                self.asm
                    .load_mem(ty.to_machine(), Reg::Gpr(RAX), Mem::Local(x_loc.off()));
                self.asm
                    .load_mem(ty.to_machine(), Reg::Gpr(RCX), Mem::Local(y_loc.off()));
                if loc.is_gpr() {
                    f(&mut self.asm, ty.to_machine(), loc.gpr(), RAX, RCX);
                } else {
                    f(&mut self.asm, ty.to_machine(), RAX, RAX, RCX);
                    self.asm
                        .store_mem(ty.to_machine(), Mem::Local(loc.off()), Reg::Gpr(RAX));
                }
            }
        }
        value
    }

    /// Integer addition
    pub fn iadd(&mut self, x: Value, y: Value) -> Value {
        self.bin_int(x, y, &Assembler::int_add)
    }
    /// Integer multiplication
    pub fn imul(&mut self, x: Value, y: Value) -> Value {
        self.bin_int(x, y, &Assembler::int_mul)
    }
    /// Integer substraction
    pub fn isub(&mut self, x: Value, y: Value) -> Value {
        self.bin_int(x, y, &Assembler::int_sub)
    }
    /// Integer division
    pub fn idiv(&mut self, x: Value, y: Value) -> Value {
        self.bin_int(x, y, &Assembler::int_div)
    }

    pub fn imod(&mut self, x: Value, y: Value) -> Value {
        self.bin_int(x, y, &Assembler::int_mod)
    }

    pub fn ret(&mut self, x: Value) {
        let loc = self.get_value_loc(x);

        if loc.is_gpr() {
            emit_mov_reg_reg(&mut self.asm, 1, loc.gpr(), RAX);
        } else if loc.is_fpr() {
            movsd(&mut self.asm, XMM0, loc.fpr());
        } else {
            let ty = self.get_value_type(x);
            if ty.is_float() {
                self.asm
                    .load_mem(ty.to_machine(), Reg::Float(XMM0), Mem::Local(loc.off()));
            } else {
                self.asm
                    .load_mem(ty.to_machine(), Reg::Gpr(RAX), Mem::Local(loc.off()));
            }
        }
        self.epilog();
    }

    pub fn fix_prolog(&mut self) {
        /*let reloc = &self.prolog;
        let bits: [u8;4] = unsafe {::std::mem::transmute(self.stack_offset)};

        let mut pc = 0;
        for i in reloc.at..reloc.to {
            self.asm.data[i] = bits[pc];
            pc += 1;
        }*/
    }

    pub fn call(&mut self, fname: &str, args: &[Value],ret: Type) -> Value {
        let value = Value::new(self.value_id);
        self.value_types.insert(value,ret);
        self.value_id += 1;

        let mut used = vec![];

        let register_args = {
            let mut temp: Vec<(ValueData, Reg, Type)> = vec![];
            let mut pc = 0;
            let mut fpc = 0;

            for (idx, value) in args.iter().rev().enumerate() {
                used.push(idx);

                let ty = self.get_value_type(*value);
                let loc = self.get_value_loc(*value);
                if !ty.is_float() {
                    /*if loc.is_gpr() {
                        emit_mov_reg_reg(&mut self.asm, ty.x64(), loc.gpr(), ARG_GPR[pc]);
                    } else {
                        self.asm.store_mem(ty.to_machine(),Mem::Local(loc.off()),Reg::Gpr(ARG_GPR[pc]));
                    }*/
                    
                    temp.push((loc, Reg::Gpr(ARG_GPR[pc]), ty));
                    if pc != ARG_GPR.len() {
                        pc += 1;
                    }
                } else {
                    if fpc != ARG_FPR.len() {
                        fpc += 1;
                    }
                    temp.push((loc, Reg::Float(ARG_FPR[fpc]), ty));
                }
                self.free(*value);
            }

            temp
        };

        for (idx, value) in args.iter().rev().enumerate() {
            if used.contains(&idx) {
                continue;
            }

            let loc = self.get_value_loc(*value);

            if loc.is_gpr() {
                emit_pushq_reg(&mut self.asm, loc.gpr());
            } else {
                self.asm.push_freg(loc.fpr());
            }
        }

        for (loc,to,ty) in register_args.iter() {
            if !ty.is_float() {
                if loc.is_gpr() {
                    emit_mov_reg_reg(&mut self.asm, ty.x64(), loc.gpr(),to.reg());
                } else {
                    self.asm.load_mem(ty.to_machine(),*to,Mem::Local(loc.off()));
                }
            } else {
                if loc.is_fpr() {
                    if ty.x64() == 0 {
                        movss(&mut self.asm, to.freg(), loc.fpr());
                    } else {
                        movsd(&mut self.asm,to.freg(),loc.fpr());
                    }
                } else {
                    self.asm.load_mem(ty.to_machine(),*to,Mem::Local(loc.off()));
                }
            }
        }

        self.asm.load_int_const(MachineMode::Ptr,RAX,0);
        self.relocs.push(Reloc {
            global_name: fname.to_owned(),
            at: self.asm.pos() - 8,
            to: self.asm.pos(),
        });
        emit_callq_reg(&mut self.asm, RAX);
        let loc = self.allocate_reg(ret);

        if loc.is_fpr() {
            if ret.x64() == 0 {
                movss(&mut self.asm, loc.fpr(), XMM0);
            } else {
                movsd(&mut self.asm,loc.fpr(),XMM0);
            }
        } else if loc.is_gpr() {
            emit_mov_reg_reg(&mut self.asm, ret.x64(), RAX, loc.gpr());
        } else {
            if ret.is_float() {
                self.asm.store_mem(ret.to_machine(),Mem::Local(loc.off()),Reg::Float(loc.fpr()));
            } else {
                self.asm.store_mem(ret.to_machine(),Mem::Local(loc.off()),Reg::Gpr(loc.gpr()));
            }
        }

        self.values.insert(value,loc);


        value

    }
}
