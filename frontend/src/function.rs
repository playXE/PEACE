use crate::abi::*;
use crate::kind::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ebb(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Variable(u32);

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct CallFixup {
    pub name: String,
    pub pos: usize,
}

impl Variable {
    pub fn new(i: usize) -> Variable {
        Variable(i as u32)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Value(u32);

impl Value {
    pub fn new(i: usize) -> Value {
        Value(i as u32)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl Ebb {
    pub fn new(i: usize) -> Ebb {
        Ebb(i as u32)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

use peace_backend::registers::*;
use peace_backend::sink::{get_executable_memory, Memory, Sink};
use peace_backend::CondCode;
use peace_backend::Membase;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Function {
    pub name: String,
    linkage: Linkage,
    /// External functions like `printf` may use VA_ARGS so we just don't use parameters
    parameters: Option<Vec<Param>>,
    sink: Sink,
    used_regs: Vec<Register>,
    free_regs: Vec<Register>,
    values: HashMap<Value, Kind>,
    value_kind: HashMap<Value, ValueKind>,
    localsize: i32,
    idx: usize,
    pub fixups: Vec<CallFixup>,
    is_alive: HashMap<Value, bool>,
    //values_location: HashMap<Value,Ebb>,
    variables: HashMap<Variable, Kind>,
    variable_loc: HashMap<Variable, i32>,
}

#[derive(Clone, Debug, Copy)]
pub enum ValueKind {
    Reg(Register),
    Stack(i32),
}

impl Function {
    pub fn new(name: &str, linkage: Linkage) -> Function {
        let mut sink = Sink::new();
        sink.emit_prolog();
        Function {
            name: name.to_owned(),
            linkage: linkage.clone(),
            parameters: if !linkage.is_import() || !linkage.is_extern() || !linkage.is_dynamic() {
                Some(vec![])
            } else {
                None
            },
            sink: sink,
            idx: 0,
            free_regs: vec![
                Register::General(RAX),
                Register::General(R13),
                Register::General(R14),
                Register::General(R15),
                Register::General(R12),
                Register::Float(XMM10),
                Register::Float(XMM11),
                Register::Float(XMM12),
                Register::Float(XMM12),
                Register::Float(XMM13),
            ],
            fixups: Vec::new(),
            used_regs: vec![],
            is_alive: HashMap::new(),
            localsize: 0,
            value_kind: HashMap::new(),
            values: HashMap::new(),
            //values_location: HashMap::new(),
            variable_loc: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    pub fn get_value_data<'r>(&'r self, v: &Value) -> &'r Kind {
        self.values.get(v).expect("value not defined")
    }

    pub fn add_params(&mut self, params_: Vec<Kind>) {
        let params = REG_PARAMS;
        let fparams = FREG_PARAMS;
        let mut fpc = 0;
        let mut pc = 0;
        let mut used_params = vec![];
        let mut name = 0;

        for (idx, kind) in params_.iter().enumerate() {
            if pc < params.len() && kind.is_int() {
                self.sink.emit_mov_reg_reg(kind.x64(), params[pc], R10);

                let arg = self.declare_variable(name, *kind);

                let off = self.variable_loc.get(&arg).unwrap();

                self.sink.store_mem(
                    kind.to_machine(),
                    Membase::Local(*off),
                    Register::General(R10),
                );
                name += 1;
                if pc != params.len() {
                    pc += 1;
                }
                used_params.push(idx);
            } else {
                if kind.x64() == 0 {
                    self.sink.movss(XMM9, fparams[fpc]);
                } else {
                    self.sink.movsd(XMM9, fparams[fpc]);
                }

                let arg = self.declare_variable(name, *kind);

                let off = self.variable_loc.get(&arg).unwrap();

                self.sink.store_mem(
                    kind.to_machine(),
                    Membase::Local(*off),
                    Register::Float(XMM9),
                );
                name += 1;
                if fpc != fparams.len() {
                    fpc += 1;
                }
                used_params.push(idx);
            }
        }

        let mut off = 0;

        for (idx, kind) in params_.iter().enumerate() {
            if used_params.contains(&idx) {
                continue;
            }

            let ty: Kind = *kind;
            if ty.is_int() {
                self.sink
                    .load_mem(ty.to_machine(), Register::General(R10), Membase::Local(off));
                let size = ty.to_machine().size() as i32;

                let var = self.declare_variable(name, ty);
                name += 1;

                let varoff = self.variable_loc.get(&var).unwrap();

                self.sink.store_mem(
                    ty.to_machine(),
                    Membase::Local(*varoff),
                    Register::General(R10),
                );

                off = peace_backend::align(off as i32 + size, size);
            } else {
                self.sink
                    .load_mem(ty.to_machine(), Register::Float(XMM9), Membase::Local(off));
                let size = ty.to_machine().size() as i32;

                let var = self.declare_variable(name, ty);

                name += 1;
                let varoff = self.variable_loc.get(&var).unwrap();

                self.sink.store_mem(
                    ty.to_machine(),
                    Membase::Local(*varoff),
                    Register::Float(XMM9),
                );
                off = peace_backend::align(off as i32 + size, size);
            }
        }
    }

    pub fn sink<'r>(&'r mut self) -> &'r mut Sink {
        &mut self.sink
    }

    pub fn linkage(&self) -> Linkage {
        self.linkage.clone()
    }

    pub fn params<'r>(&'r mut self) -> &'r mut Vec<Param> {
        if self.parameters.is_some() {
            self.parameters.as_mut().unwrap()
        } else {
            self.parameters = Some(vec![]);
            self.parameters.as_mut().unwrap()
        }
    }

    pub fn new_label(&mut self) -> u32 {
        self.sink.create_label() as u32
    }

    pub fn label_here(&mut self, l: u32) {
        self.sink.bind_label(l as usize);
    }

    pub fn jump(&mut self, l: u32) {
        self.sink.emit_jmp(l as usize);
    }

    pub fn jump_zero(&mut self, v: Value, l: u32) {
        let kind = self.values.get(&v).unwrap();
        match kind {
            Int32 | Int64 | Bool32 | Bool64 => {
                self.load_value(v, Register::General(R10));
                self.sink.test_and_jump_if(CondCode::Zero, R10, l as usize);
            }
            _ => unreachable!(),
        }
    }

    pub fn jump_nonzero(&mut self, v: Value, l: u32) {
        let kind = self.values.get(&v).unwrap();
        match kind {
            Int32 | Int64 | Bool32 | Bool64 => {
                self.load_value(v, Register::General(R10));
                self.sink
                    .test_and_jump_if(CondCode::NonZero, R10, l as usize);
            }
            _ => unreachable!(),
        }
    }

    pub fn allocate(&mut self, t: Kind) -> i32 {
        let size = t.to_machine().size() as i32;

        let new_offset = peace_backend::align(self.localsize as i32 + size, size);
        self.localsize = new_offset as i32;
        new_offset
    }

    fn allocate_value(&mut self, v: Value) -> ValueKind {
        let data = self.values.get(&v).unwrap();

        let general = match data {
            Int64 | Int32 | Bool32 | Bool64 | Pointer => true,
            Float32 | Float64 => false,
        };

        if general {
            let available = [
                Register::General(R12),
                Register::General(R13),
                Register::General(R14),
                Register::General(R15),
            ];
            let mut free = None;

            for register in available.iter() {
                if !self.used_regs.contains(&register) {
                    free = Some(register);
                    self.used_regs.push(*register);
                    break;
                }
            }

            if free.is_some() {
                return ValueKind::Reg(free.unwrap().clone());
            } else {
                let size = self.allocate(*data);

                return ValueKind::Stack(-size);
            }
        } else {
            let available = [
                Register::Float(XMM10),
                Register::Float(XMM11),
                Register::Float(XMM12),
                Register::Float(XMM12),
                Register::Float(XMM13),
            ];
            let mut free = None;
            for register in available.iter() {
                if !self.used_regs.contains(&register) {
                    free = Some(register);
                    self.used_regs.push(*register);
                    break;
                }
            }

            if free.is_some() {
                return ValueKind::Reg(free.unwrap().clone());
            } else {
                let size = self.allocate(*data);

                return ValueKind::Stack(-size);
            }
        }
    }

    fn kill(&mut self, v: Value) -> bool {
        let kinds = self.value_kind.clone();
        let vkind = kinds.get(&v).unwrap();
        self.values.remove(&v);
        //self.values_location.remove(&v);
        self.value_kind.remove(&v);
        match vkind {
            ValueKind::Reg(r) => {
                for (i, reg) in self.used_regs.iter().enumerate() {
                    if reg == r {
                        self.used_regs.remove(i);
                        return true;
                    }
                }
                return false;
            }
            ValueKind::Stack(off) => {
                self.localsize -= off;
                return true;
            }
        }
    }

    pub fn declare_variable(&mut self, index: u32, ty: Kind) -> Variable {
        let off = self.allocate(ty);

        let var = Variable::new(index as usize);

        self.variables.insert(var, ty);
        self.variable_loc.insert(var, -off);

        var
    }

    pub fn def_var(&mut self, var: Variable, val: Value) {
        let data = self.get_value_data(&val);
        let data_var = self
            .variables
            .get(&var)
            .expect(&format!("variable {:?} not defined", var));;

        if data != data_var {
            panic!(
                "Value and variable got different types. Expected {:?},got {:?}",
                data_var, data
            );
        }

        let offset = self
            .variable_loc
            .get(&var)
            .expect(&format!("variable {:?} not defined", var));

        let vkind = self.value_kind.get(&val).expect("Value not defined");
        match vkind.clone() {
            ValueKind::Reg(register) => {
                self.sink
                    .store_mem(data_var.to_machine(), Membase::Local(*offset), register);
            }
            ValueKind::Stack(off) => {
                if data_var.is_int() {
                    self.sink.load_mem(
                        data_var.to_machine(),
                        Register::General(R10),
                        Membase::Local(off),
                    );
                    self.sink.store_mem(
                        data_var.to_machine(),
                        Membase::Local(*offset),
                        Register::General(R10),
                    );
                } else {
                    self.sink.load_mem(
                        data_var.to_machine(),
                        Register::Float(XMM8),
                        Membase::Local(off),
                    );
                    self.sink.store_mem(
                        data_var.to_machine(),
                        Membase::Local(*offset),
                        Register::Float(XMM8),
                    );
                }
            }
        }

        self.kill(val);
    }

    pub fn use_var(&mut self, var: Variable) -> Value {
        let data = self
            .variables
            .get(&var)
            .expect("Variable not defined")
            .clone();
        let offset = self
            .variable_loc
            .get(&var)
            .expect("Value not defined")
            .clone();

        let value = Value::new(self.idx);
        self.idx += 1;
        self.values.insert(value, data);

        let vkind = self.allocate_value(value);
        self.value_kind.insert(value, vkind);
        match vkind {
            ValueKind::Reg(reg) => {
                if data.is_int() {
                    self.sink.load_mem(
                        data.to_machine(),
                        Register::General(R10),
                        Membase::Local(offset),
                    );
                    self.sink.emit_mov_reg_reg(data.x64(), R10, reg.reg());
                }
            }
            ValueKind::Stack(off) => {
                self.sink.load_mem(
                    data.to_machine(),
                    Register::General(R10),
                    Membase::Local(offset),
                );
                self.sink.store_mem(
                    data.to_machine(),
                    Membase::Local(off),
                    Register::General(R10),
                );
            }
        }

        value
    }

    pub fn store(&mut self, ty: Kind, base: Value, offset: i32, src: Value) {
        self.load_value(base, Register::General(R10));
        let vdata = self.get_value_data(&src);
        if vdata.is_int() {
            self.load_value(src, Register::General(R11));
            self.sink.store_mem(
                ty.to_machine(),
                Membase::Base(R10, offset),
                Register::General(R11),
            );
        } else {
            self.load_value(src, Register::Float(XMM9));
        }
    }

    pub fn iconst(&mut self, imm: impl Into<i64>, ty: Kind) -> Value {
        let value = Value::new(self.idx);
        self.idx += 1;

        self.values.insert(value, ty);
        //self.values_location.insert(value,self.current_block);

        let vkind = self.allocate_value(value);
        self.value_kind.insert(value, vkind.clone());
        if let ValueKind::Reg(reg) = vkind {
            self.sink.load_int(ty.to_machine(), reg.reg(), imm.into());
            return value;
        }

        if let ValueKind::Stack(off) = vkind {
            self.sink.load_int(ty.to_machine(), R10, imm.into());

            self.sink
                .store_mem(ty.to_machine(), Membase::Local(off), Register::General(R10));
            return value;
        }

        unreachable!()
    }

    fn load_value(&mut self, v: Value, reg: Register) {
        let data = self.values.get(&v).unwrap();
        let vkind = self.value_kind.get(&v).expect("Kind not found");
        let x64 = *data == Int64 || *data == Pointer || *data == Bool64;
        match vkind {
            ValueKind::Reg(r) => match r {
                Register::General(g) => {
                    self.sink.emit_mov_reg_reg(x64 as u8, *g, reg.reg());
                }
                Register::Float(f) => {
                    if data == &Float32 {
                        self.sink.movss(reg.freg(), *f);
                    } else {
                        self.sink.movsd(reg.freg(), *f);
                    }
                }
            },
            ValueKind::Stack(off) => {
                self.sink
                    .load_mem(data.to_machine(), reg, Membase::Local(*off));
            }
        }
    }

    pub fn isub(&mut self, x: Value, y: Value) -> Value {
        self.load_value(x, Register::General(R10));
        self.load_value(y, Register::General(R11));
        let data = self.values.get(&x).unwrap();
        let data = data.clone();
        self.sink.emit_sub_reg_reg(data.x64(), R10, R11);
        let value = Value::new(self.idx);
        self.values.insert(value, data.clone());
        //self.values_location.insert(value, self.current_block);
        self.kill(x);
        self.kill(y);
        let kind = self.allocate_value(value);
        match kind {
            ValueKind::Reg(r) => {
                self.sink.emit_mov_reg_reg(data.x64(), R11, r.reg());
            }
            ValueKind::Stack(off) => {
                self.sink.store_mem(
                    data.to_machine(),
                    Membase::Local(off),
                    Register::General(R11),
                );
            }
        }
        self.value_kind.insert(value, kind);
        self.idx += 1;
        value
    }

    pub fn iadd(&mut self, x: Value, y: Value) -> Value {
        self.load_value(x, Register::General(R10));
        self.load_value(y, Register::General(R11));
        let data = self.values.get(&x).unwrap();
        let data = data.clone();
        self.sink.emit_add_reg_reg(data.x64(), R10, R11);
        let value = Value::new(self.idx);
        self.values.insert(value, data.clone());
        //self.values_location.insert(value, self.current_block);
        self.kill(x);
        self.kill(y);
        let kind = self.allocate_value(value);
        match kind {
            ValueKind::Reg(r) => {
                self.sink.emit_mov_reg_reg(data.x64(), R11, r.reg());
            }
            ValueKind::Stack(off) => {
                self.sink.store_mem(
                    data.to_machine(),
                    Membase::Local(off),
                    Register::General(R11),
                );
            }
        }
        self.value_kind.insert(value, kind);
        self.idx += 1;
        value
    }

    pub fn imul(&mut self, x: Value, y: Value) -> Value {
        self.load_value(x, Register::General(R10));
        self.load_value(y, Register::General(R11));
        let data = self.values.get(&x).unwrap();
        let data = data.clone();
        self.sink.emit_imul_reg_reg(data.x64(), R10, R11);
        let value = Value::new(self.idx);
        self.values.insert(value, data.clone());
        //self.values_location.insert(value, self.current_block);
        self.kill(x);
        self.kill(y);
        let kind = self.allocate_value(value);
        match kind {
            ValueKind::Reg(r) => {
                self.sink.emit_mov_reg_reg(data.x64(), R11, r.reg());
            }
            ValueKind::Stack(off) => {
                self.sink.store_mem(
                    data.to_machine(),
                    Membase::Local(off),
                    Register::General(R11),
                );
            }
        }
        self.value_kind.insert(value, kind);
        self.idx += 1;
        value
    }

    pub fn idiv(&mut self, x: Value, y: Value) -> Value {
        self.load_value(x, Register::General(R10));
        self.load_value(y, Register::General(R11));
        let data = self.values.get(&x).unwrap();
        let data = data.clone();
        self.sink.emit_mov_reg_reg(data.x64(), R10, RAX);
        if data.x64() != 0 {
            self.sink.emit_cqo();
        } else {
            self.sink.emit_cdq();
        }
        self.sink.emit_idiv_reg_reg(data.x64(), R11);
        let value = Value::new(self.idx);
        self.values.insert(value, data.clone());
        //self.values_location.insert(value, self.current_block);
        self.kill(x);
        self.kill(y);
        let kind = self.allocate_value(value);
        match kind {
            ValueKind::Reg(r) => {
                self.sink.emit_mov_reg_reg(data.x64(), RAX, r.reg());
            }
            ValueKind::Stack(off) => {
                self.sink.store_mem(
                    data.to_machine(),
                    Membase::Local(off),
                    Register::General(R11),
                );
            }
        }
        self.value_kind.insert(value, kind);
        self.idx += 1;
        value
    }

    pub fn ret(&mut self, x: Value) {
        let data = self.values.get(&x).unwrap();
        match data {
            Int32 | Int64 | Pointer | Bool32 | Bool64 => {
                self.load_value(x, Register::General(RAX));

                self.sink.emit_epilog();
                self.sink.ret();
            }
            _ => {
                self.load_value(x, Register::Float(XMM0));
                self.sink.emit_epilog();
                self.sink.ret();
            }
        }
    }

    pub fn memory(&self) -> Memory {
        get_executable_memory(&self.sink)
    }

    pub fn call_indirect(&mut self, fun: &str, args: &[Value], ret: Kind) -> Value {
        let params = REG_PARAMS;
        let fparams = FREG_PARAMS;

        let mut fpc = 0;
        let mut pc = 0;
        let mut used_params = vec![];
        self.sink.emit_push_reg(R12);
        self.sink.emit_push_reg(R13);
        self.sink.emit_push_reg(R14);
        self.sink.emit_push_reg(R15);

        for (idx, value) in args.iter().enumerate() {
            let kind = self.values.get(value).unwrap();

            if pc < params.len() && kind.is_int() {
                self.load_value(*value, Register::General(params[pc]));
                if pc != params.len() {
                    pc += 1;
                }
                used_params.push(idx);
            } else if fpc < params.len() && !kind.is_int() {
                self.load_value(*value, Register::Float(fparams[fpc]));
                if fpc != fparams.len() {
                    fpc += 1;
                }
                used_params.push(idx);
            }
        }
        let mut _size = 0i32;
        for (idx, value) in args.iter().enumerate() {
            if used_params.contains(&idx) {
                continue;
            }

            let kind: &Kind = self.values.get(&value).unwrap();
            let kind = kind.clone();
            _size += kind.to_machine().size() as i32;
            if !kind.is_int() {
                self.load_value(*value, Register::Float(XMM8));
                self.sink.store_mem(
                    kind.to_machine(),
                    Membase::Local(_size),
                    Register::Float(XMM8),
                );
            } else {
                self.load_value(*value, Register::General(R10));
                self.sink.store_mem(
                    kind.to_machine(),
                    Membase::Local(_size),
                    Register::General(R10),
                );
            }
        }

        self.sink.load_int(Pointer.to_machine(), RAX, 0);
        self.fixups.push(CallFixup {
            name: fun.to_owned(),
            pos: self.sink.size() - 8,
        });

        self.sink.emit_call_reg(RAX);

        //self.sink.emit_pop_reg(RAX);
        self.sink.emit_pop_reg(R15);
        self.sink.emit_pop_reg(R14);
        self.sink.emit_pop_reg(R13);
        self.sink.emit_pop_reg(R12);
        let value = Value::new(self.idx);
        self.idx += 1;
        self.values.insert(value, ret);

        let vkind = self.allocate_value(value);
        self.value_kind.insert(value, vkind.clone());

        match vkind {
            ValueKind::Reg(reg) => match reg {
                Register::General(g) => self.sink.emit_mov_reg_reg(ret.x64(), RAX, g),
                Register::Float(f) => {
                    if ret == Float32 {
                        self.sink.movss(f, XMM0);
                    } else {
                        self.sink.movsd(f, XMM0);
                    }
                }
            },
            ValueKind::Stack(off) => {
                if !ret.is_int() {
                    self.sink.store_mem(
                        ret.to_machine(),
                        Membase::Local(off),
                        Register::Float(XMM0),
                    );
                } else {
                    self.sink.store_mem(
                        ret.to_machine(),
                        Membase::Local(off),
                        Register::General(RAX),
                    );
                }
            }
        }
        value
    }
}
