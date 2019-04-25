use crate::backend::align;
use crate::backend::assembler::*;
use crate::backend::assemblerx64::*;
use crate::backend::constants_x64::*;
use crate::backend::*;
use crate::module::*;
use crate::types::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Copy, PartialEq)]
enum ValueData {
    Gpr(Register),
    Fpr(XMMRegister),
    Stack(i32),
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Reloc {
    pub global_name: String,
    pub at: usize,
    pub to: usize,
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

#[cfg(target_family = "windows")]
const ARG_GPR: [Register; 4] = [RCX, RDX, R8, R9];
#[cfg(target_family = "windows")]
const ARG_FPR: [Register; 4] = [XMM0, XMM1, XMM2, XMM3];

#[cfg(target_family = "unix")]
const ARG_GPR: [Register; 6] = [RDI, RSI, RDX, RCX, R8, R9];
#[cfg(target_family = "unix")]
const ARG_FPR: [XMMRegister; 8] = [XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7];
#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub asm: Assembler,
    pub free: HashSet<Reg>,
    pub stack_offset: i32,
    pub used: HashSet<Reg>,
    pub(crate) relocs: Vec<Reloc>,
    variables: HashMap<u32, (Type, i32)>,
    values: HashMap<Value, (ValueData, Type)>,
    value_id: u32,
    labels: HashMap<String, usize>,
    pub linkage: crate::module::Linkage,
}

impl Function {
    pub fn new(name: &str, linkage: Linkage) -> Function {
        let mut f = Function {
            values: HashMap::new(),
            name: name.to_owned(),
            linkage: linkage,
            asm: Assembler::new(),
            free: HashSet::new(),
            stack_offset: 0,
            used: HashSet::new(),
            relocs: vec![],
            variables: HashMap::new(),
            value_id: 0,
            labels: HashMap::new(),
        };
        f.new_label("<__epilog__>");
        f.prolog();
        f
    }
    pub fn new_label(&mut self, name: &str) {
        let label = self.asm.create_label();
        self.labels.insert(name.to_owned(), label);
    }

    pub fn bind_label(&mut self, name: &str) {
        let label = self.labels.get(name).expect("Label not found");

        self.asm.bind_label(*label);
    }

    pub fn asm_mut<'a>(&'a mut self) -> &'a mut Assembler {
        &mut self.asm
    }

    pub fn get_value_type(&self, value: Value) -> Type {
        self.values.get(&value).expect("Value not found").1
    }

    fn get_value_loc(&self, value: Value) -> ValueData {
        self.values.get(&value).expect("Value not found").0
    }

    fn free(&mut self, v: Value) {
        let loc = self.get_value_loc(v);
        match loc {
            ValueData::Gpr(reg) => {
                self.used.remove(&Reg::Gpr(reg));
            }
            ValueData::Fpr(reg) => {
                self.used.remove(&Reg::Float(reg));
            }
            _ => {}
        };
        self.values.remove(&v);
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
                if !self.used.contains(&Reg::Gpr(*reg)) {
                    self.used.insert(Reg::Gpr(*reg));

                    return ValueData::Gpr(*reg);
                }
            }
            let off = self.allocate_in_stack(ty);
            ValueData::Stack(-off)
        } else {
            for reg in AVAIL_FPR.iter() {
                if !self.used.contains(&Reg::Float(*reg)) {
                    self.used.insert(Reg::Float(*reg));
                    return ValueData::Fpr(*reg);
                }
            }
            let off = self.allocate_in_stack(ty);
            ValueData::Stack(-off)
        }
    }

    pub fn allocate_in_stack(&mut self, ty: Type) -> i32 {
        let size = ty.to_machine().size();
        let offset = align(self.stack_offset + size as i32, size as i32);
        self.stack_offset = offset;
        offset
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
        self.values.insert(value, (loc, ty));
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

        self.values.insert(value, (loc, ty));

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
    pub fn prolog(&mut self) {
        emit_pushq_reg(&mut self.asm, RBP);
        emit_mov_reg_reg(&mut self.asm, 1, RSP, RBP);
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


    pub fn jump(&mut self, label: &str) {
        let l = self.labels.get(label).expect("Label not found");
        emit_jmp(&mut self.asm, *l);
    }

    pub fn int_cmp(&mut self, x: Value, y: Value, cc: CondCode) -> Value {
        let value = Value::new(self.value_id);
        self.value_id += 1;
        let (x_loc, x_ty) = (self.get_value_loc(x), self.get_value_type(x));
        let (y_loc, y_ty) = (self.get_value_loc(y), self.get_value_type(y));

        self.free(x);
        self.free(y);

        let loc = self.allocate_reg(Type::I8);

        if x_loc.is_off() && y_loc.is_off() {
            self.asm
                .load_mem(x_ty.to_machine(), Reg::Gpr(RAX), Mem::Local(x_loc.off()));
            self.asm
                .load_mem(y_ty.to_machine(), Reg::Gpr(RBX), Mem::Local(y_loc.off()));
            self.asm.cmp_reg(x_ty.to_machine(), RAX, RBX);
            self.asm.set(RAX, cc);
        } else if x_loc.is_off() {
            self.asm
                .load_mem(x_ty.to_machine(), Reg::Gpr(RAX), Mem::Local(x_loc.off()));
            self.asm.cmp_reg(x_ty.to_machine(), RAX, y_loc.gpr());
            self.asm.set(RAX, cc);
        } else if y_loc.is_off() {
            self.asm
                .load_mem(y_ty.to_machine(), Reg::Gpr(RAX), Mem::Local(y_loc.off()));
            self.asm.cmp_reg(x_ty.to_machine(), x_loc.gpr(), RAX);
            self.asm.set(RAX, cc);
        } else {
            self.asm
                .cmp_reg(x_ty.to_machine(), x_loc.gpr(), y_loc.gpr());
            self.asm.set(RAX, cc);
        }

        if loc.is_off() {
            self.asm
                .store_mem(MachineMode::Int8, Mem::Local(loc.off()), Reg::Gpr(RAX));
        } else {
            emit_movb_reg_reg(&mut self.asm, RAX, loc.gpr());
        }

        self.values.insert(value, (loc, x_ty));

        value
    }

    pub fn float_cmp(&mut self, x: Value, y: Value, cc: CondCode) -> Value {
        let value = Value::new(self.value_id);
        self.value_id += 1;

        let (x_loc, x_ty) = (self.get_value_loc(x), self.get_value_type(x));
        let (y_loc, y_ty) = (self.get_value_loc(y), self.get_value_type(y));

        assert!(x_ty.is_float() && y_ty.is_float(), "Float values expected");

        self.free(x);
        self.free(y);
        let loc = self.allocate_reg(Type::I8);
        if x_loc.is_off() && y_loc.is_off() {
            self.asm
                .load_mem(x_ty.to_machine(), Reg::Float(XMM0), Mem::Local(x_loc.off()));
            self.asm
                .load_mem(y_ty.to_machine(), Reg::Float(XMM1), Mem::Local(y_loc.off()));

            self.asm.float_cmp(x_ty.to_machine(), RAX, XMM0, XMM1, cc);

        } else if x_loc.is_off() {
            self.asm
                .load_mem(x_ty.to_machine(), Reg::Float(XMM0), Mem::Local(x_loc.off()));
            self.asm
                .float_cmp(x_ty.to_machine(), RAX, XMM0, y_loc.fpr(), cc);
        } else if y_loc.is_off() {
            self.asm
                .load_mem(y_ty.to_machine(), Reg::Float(XMM0), Mem::Local(y_loc.off()));
            self.asm
                .float_cmp(x_ty.to_machine(), RAX, x_loc.fpr(), XMM0, cc);
        } else {
            self.asm
                .float_cmp(x_ty.to_machine(), RAX, x_loc.fpr(), y_loc.fpr(), cc);
        }
        if loc.is_off() {
            self.asm
                .store_mem(MachineMode::Int8, Mem::Local(loc.off()), Reg::Gpr(RAX));
        } else {
            emit_movb_reg_reg(&mut self.asm, RAX, loc.gpr());
        }

        self.values.insert(value, (loc, x_ty));
        value
    }

    pub fn load(&mut self, base: Value, offset: i32, ty: Type) -> Value {
        assert!(!self.get_value_type(base).is_float());
        let value = Value::new(self.value_id);
        self.value_id += 1;
        let loc = self.get_value_loc(base);
        self.free(base);
        if loc.is_off() {
            self.asm
                .load_mem(ty.to_machine(), Reg::Gpr(RAX), Mem::Local(loc.off()));
        }
        let new_loc = self.allocate_reg(ty);
        if ty.is_float() {
            self.asm.load_mem(
                ty.to_machine(),
                Reg::Float(XMM0),
                Mem::Base(if loc.is_off() { RAX } else { loc.gpr() }, offset),
            );
            if new_loc.is_off() {
                self.asm
                    .store_mem(ty.to_machine(), Mem::Local(new_loc.off()), Reg::Float(XMM0));
            } else {
                if ty.x64() != 0 {
                    movsd(&mut self.asm, loc.fpr(), XMM0);
                } else {
                    movss(&mut self.asm, loc.fpr(), XMM0);
                }
            }
        } else {
            self.asm.load_mem(
                ty.to_machine(),
                Reg::Gpr(RAX),
                Mem::Base(if loc.is_off() { RAX } else { loc.gpr() }, offset),
            );

            if new_loc.is_off() {
                self.asm
                    .store_mem(ty.to_machine(), Mem::Local(new_loc.off()), Reg::Gpr(RAX));
            } else {
                emit_mov_reg_reg(&mut self.asm, ty.x64(), RAX, loc.gpr());
            }
        }

        self.values.insert(value, (new_loc, ty));
        value
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

        self.jump("<__epilog__>");

    }

    pub fn finalize(&mut self) {
        let l = self.labels.get("<__epilog__>").unwrap();
        self.asm.bind_label(*l);
        emit_popq_reg(&mut self.asm, RBP);
        self.asm.emit(0xc3);
    }

    pub fn call(&mut self, fname: &str, args: &[Value], ret: Type) -> Value {
        let value = Value::new(self.value_id);

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
                unimplemented!()

            }
        }

        for (loc, to, ty) in register_args.iter() {
            if !ty.is_float() {
                if loc.is_gpr() {
                    emit_mov_reg_reg(&mut self.asm, ty.x64(), loc.gpr(), to.reg());
                } else {
                    self.asm
                        .load_mem(ty.to_machine(), *to, Mem::Local(loc.off()));
                }
            } else {
                if loc.is_fpr() {
                    if ty.x64() == 0 {
                        movss(&mut self.asm, to.freg(), loc.fpr());
                    } else {
                        movsd(&mut self.asm, to.freg(), loc.fpr());
                    }
                } else {
                    self.asm
                        .load_mem(ty.to_machine(), *to, Mem::Local(loc.off()));
                }
            }
        }

        self.asm.load_int_const(MachineMode::Ptr, RAX, 0);
        self.relocs.push(Reloc {
            global_name: fname.to_owned(),
            at: self.asm.pos() - 8,
            to: self.asm.pos(),
        });
        emit_callq_reg(&mut self.asm, RAX);
        if ret != Type::Void {
            let loc = self.allocate_reg(ret);

            if loc.is_fpr() {
                if ret.x64() == 0 {
                    movss(&mut self.asm, loc.fpr(), XMM0);
                } else {
                    movsd(&mut self.asm, loc.fpr(), XMM0);
                }
            } else if loc.is_gpr() {
                emit_mov_reg_reg(&mut self.asm, ret.x64(), RAX, loc.gpr());
            } else {
                if ret.is_float() {
                    self.asm.store_mem(
                        ret.to_machine(),
                        Mem::Local(loc.off()),
                        Reg::Float(loc.fpr()),
                    );
                } else {
                    self.asm.store_mem(
                        ret.to_machine(),
                        Mem::Local(loc.off()),
                        Reg::Gpr(loc.gpr()),
                    );
                }
            }


            self.values.insert(value, (loc, ret));
        }
        value
    }
}
