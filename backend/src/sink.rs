use self::registers::{FReg, Register, R9, RBP};
use self::types::*;
use crate::datasegment::DSeg;
use crate::*;

use byteorder::{LittleEndian, WriteBytesExt};
#[derive(Clone, Debug)]
pub struct Sink {
    data: Vec<u8>,
    sink_size: size_t,
    data_segment: DSeg,
    pub jumps: Vec<ForwardJump>,
    pub labels: Vec<Option<usize>>,
}

#[derive(Debug, Clone)]
pub struct ForwardJump {
    pub at: usize,
    pub to: usize,
}

pub fn copy_vec<T: Copy>(v: &Vec<T>) -> Vec<T> {
    let mut new = vec![];

    for value in v {
        new.push(*value);
    }
    new
}

const PAGE_SIZE: usize = 4096;

use std::mem;

#[cfg(target_family = "unix")]
fn setup(size: usize) -> *mut u8 {
    unsafe {
        let size = size * PAGE_SIZE;
        let mut content: *mut libc::c_void = mem::uninitialized();
        libc::posix_memalign(&mut content, 4096, size);
        let result = libc::mmap(
            content,
            size,
            libc::PROT_EXEC | libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        mem::transmute(result)
    }
}

#[cfg(target_family = "windows")]
fn setup(size: usize) -> *mut u8 {
    unsafe {
        let _size = size * PAGE_SIZE;

        let mem: *mut u8 = mem::transmute(kernel32::VirtualAlloc(
            ::std::ptr::null_mut(),
            _size as u64,
            winapi::um::winnt::MEM_COMMIT,
            winapi::um::winnt::PAGE_EXECUTE_READWRITE,
        ));
        mem
    }
}

#[derive(Copy, Clone)]
pub struct Memory {
    start: *const u8,
    end: *const u8,

    pointer: *const u8,
    size: size_t,
}

impl Memory {
    pub fn new(ptr: *const u8) -> Memory {
        Memory {
            start: unsafe { ptr.offset(0) },
            end: ptr,
            pointer: ptr,
            size: 0xdead,
        }
    }
    pub fn start(&self) -> *const u8 {
        self.start
    }

    pub fn end(&self) -> *const u8 {
        self.end
    }

    pub fn ptr(&self) -> ptr {
        self.pointer
    }

    pub fn size(&self) -> size_t {
        self.size
    }
}

pub fn get_executable_memory(sink: &Sink) -> Memory {
    let data = copy_vec(sink.data());
    let dseg = sink.dseg();
    let total_size = data.len() + dseg.size() as usize;
    let ptr = setup(total_size);

    dseg.finish(ptr);

    let start;
    unsafe {
        start = ptr.offset(dseg.size() as isize);
        ::std::ptr::copy_nonoverlapping(data.as_ptr(), start as *mut ubyte, data.len());
    };

    let memory = Memory {
        start,
        end: unsafe { ptr.offset(total_size as isize) },
        pointer: ptr,
        size: total_size,
    };

    memory
}

impl Sink {
    pub fn new() -> Sink {
        Sink {
            data: Vec::new(),
            sink_size: 0,
            data_segment: DSeg::new(),
            labels: Vec::new(),
            jumps: Vec::new(),
        }
    }

    pub fn finish(&mut self) -> Memory {
        self.fix_forward_jumps();
        self.data_segment.align(16);
        get_executable_memory(&self)
    }

    pub fn create_label(&mut self) -> usize {
        let idx = self.labels.len();

        self.labels.push(None);
        idx
    }

    pub fn cmp(&mut self, r1: Reg, r2: Reg, t: types::Type) {
        let x64 = match t {
            I32 => 0,
            I64 => 1,
            _ => 1,
        };
        self.emit_cmp_reg_reg(x64, r1, r2);
    }

    pub fn float_cmp(&mut self, t: types::Type, dest: Reg, lhs: FReg, rhs: FReg, cond: CondCode) {
        match cond {
            CondCode::Equal | CondCode::NotEqual => {
                let init = if cond == CondCode::Equal { 0 } else { 1 };

                self.load_int(I32, R9, init);
                self.load_int(I32, dest, 0);

                match t {
                    F32 => self.ucomiss(lhs, rhs),
                    F64 => self.ucomisd(lhs, rhs),
                    _ => unreachable!(),
                };

                let parity = if cond == CondCode::Equal { false } else { true };
                self.emit_setb_reg_parity(dest, parity);
                self.cmov(0, dest, R9, CondCode::NotEqual);
            }
            CondCode::Greater | CondCode::GreaterEq => {
                self.load_int(I32, dest, 0);

                match t {
                    F32 => self.ucomiss(lhs, rhs),
                    F64 => self.ucomisd(lhs, rhs),
                    _ => unreachable!(),
                }

                let cond = match cond {
                    CondCode::Greater => CondCode::UnsignedGreater,
                    CondCode::GreaterEq => CondCode::UnsignedGreaterEq,
                    _ => unreachable!(),
                };

                self.emit_setb_reg(cond, dest);
            }

            CondCode::Less | CondCode::LessEq => {
                self.load_int(I32, dest, 0);

                match t {
                    F32 => self.ucomiss(rhs, lhs),
                    F64 => self.ucomisd(rhs, lhs),
                    _ => unreachable!(),
                }

                let cond = match cond {
                    CondCode::Less => CondCode::UnsignedGreater,
                    CondCode::LessEq => CondCode::UnsignedGreaterEq,
                    _ => unreachable!(),
                };

                self.emit_setb_reg(cond, dest);
            }
            _ => unimplemented!(),
        }
    }

    pub fn set(&mut self, dest: Reg, op: CondCode) {
        self.emit_setb_reg(op, dest);
        self.emit_movzbl_reg_reg(dest, dest);
    }

    pub fn test_and_jump_if(&mut self, cond: CondCode, reg: Reg, lbl: usize) {
        assert!(cond == CondCode::Zero || cond == CondCode::NonZero);

        self.emit_testl_reg_reg(reg, reg);
        self.jump_if(cond, lbl);
    }

    pub fn jump_if(&mut self, cond: CondCode, lbl: usize) {
        self.emit_jcc(cond, lbl);
    }

    pub fn bind_label(&mut self, lbl: usize) {
        let lbl_idx = lbl;

        assert!(self.labels[lbl_idx].is_none());
        self.labels[lbl_idx] = Some(self.data.len());
    }

    pub fn fix_forward_jumps(&mut self) {
        for jmp in &self.jumps {
            let target = self.labels[jmp.to].expect("Label not defined");
            let diff = (target - jmp.at - 4) as i32;

            let mut slice = &mut self.data[jmp.at..];
            slice.write_u32::<LittleEndian>(diff as u32).unwrap();
        }
    }

    pub fn data<'r>(&'r self) -> &'r Vec<ubyte> {
        &self.data
    }

    pub fn size(&self) -> size_t {
        self.sink_size
    }

    pub fn dseg<'r>(&'r self) -> &'r DSeg {
        &self.data_segment
    }

    pub fn new_from_buffer(buff: Vec<u8>) -> Sink {
        Sink {
            data: buff,
            sink_size: 0,
            data_segment: DSeg::new(),
            jumps: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn store_mem(&mut self, mode: Type, mem: Membase, src: Register) {
        match mem {
            Membase::Local(offset) => match mode {
                F32 => self.movss_store(mem, src.freg()),
                F64 => self.movsd_store(mem, src.freg()),
                I32 => self.emit_movl_reg_memq(src.reg(), RBP, offset),
                I64 => self.emit_movq_reg_memq(src.reg(), RBP, offset),
                _ => unreachable!(),
            },
            Membase::Base(_base, _disp) => match mode {
                F32 => self.movss_store(mem, src.freg()),
                F64 => self.movsd_store(mem, src.freg()),
                I32 => self.emit_movl_reg_memq(src.reg(), _base, _disp),
                I64 => self.emit_movq_reg_memq(src.reg(), _base, _disp),
                _ => unreachable!(),
            },
            Membase::Index(base, index, scale, disp) => match mode {
                I32 | I64 => {
                    self.emit_mov_reg_membaseindex(mode, src.reg(), base, index, scale, disp);
                }
                F32 => self.movss_store(mem, src.freg()),
                F64 => self.movsd_store(mem, src.freg()),
                _ => unreachable!(),
            },
            _ => unimplemented!(),
        }
    }

    pub fn load_mem(&mut self, t: types::Type, dest: Register, mb: Membase) {
        match mb {
            Membase::Local(offset) => match t {
                I32 => self.emit_movl_memq_reg(RBP, offset, dest.reg()),
                I64 => self.emit_movq_memq_reg(RBP, offset, dest.reg()),
                F32 => self.movss_load(dest.freg(), mb),
                F64 => self.movsd_load(dest.freg(), mb),
                _ => unreachable!(),
            },
            Membase::Base(base, disp) => match t {
                I32 => self.emit_movl_memq_reg(base, disp, dest.reg()),
                I64 => self.emit_movq_memq_reg(base, disp, dest.reg()),
                F32 => self.movss_load(dest.freg(), mb),
                F64 => self.movsd_load(dest.freg(), mb),
                _ => unreachable!(),
            },

            Membase::Index(base, index, scale, disp) => match t {
                I32 | I64 => {
                    self.emit_mov_membaseindex_reg(t, base, index, scale, disp, dest.reg())
                }
                F32 => self.movss_load(dest.freg(), mb),
                F64 => self.movsd_load(dest.freg(), mb),
                _ => unreachable!(),
            },
            Membase::Offset(_, _, _) => unimplemented!(),
        }
    }

    pub fn put(&mut self, b: ubyte) {
        self.data.push(b);
        self.sink_size += 1;
    }

    pub fn put_4(&mut self, v: uint) {
        let slice: [ubyte; 4] = unsafe { mem::transmute(v) };

        for byte in slice.iter() {
            self.put(*byte);
        }
    }

    pub fn put_8(&mut self, v: ulong) {
        let slice: [ubyte; 8] = unsafe { mem::transmute(v) };

        for byte in slice.iter() {
            self.put(*byte);
        }
    }

    pub fn dseg_mut<'r>(&'r mut self) -> &'r mut DSeg {
        &mut self.data_segment
    }

    pub fn data_mut<'r>(&'r mut self) -> &'r mut Vec<u8> {
        &mut self.data
    }

    pub fn put_slice(&mut self, s: &[ubyte]) {
        for byte in s.iter() {
            self.put(*byte);
        }
    }
}
