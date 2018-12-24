use crate::registers::*;
use crate::sink::ForwardJump;
use crate::sink::Sink;
use crate::types::*;
use crate::Membase;
use crate::*;

use std::mem::transmute;

pub type Label = usize;

trait Idx {
    fn index(&self) -> usize;
}

impl Idx for usize {
    fn index(&self) -> usize {
        self.clone()
    }
}

pub fn fits_i8(imm: i32) -> bool {
    imm == (imm as i8) as i32
}

impl Sink {
    pub fn emit_sib(&mut self, scale: u8, index: u8, base: u8) {
        assert!(scale < 4);
        assert!(index < 8);
        assert!(base < 8);

        self.put(scale << 6 | index << 3 | base);
    }

    pub fn emit_mov_membaseq_reg(
        &mut self,
        rex_prefix: u8,
        x64: u8,
        opcode: u8,
        src: Reg,
        disp: i32,
        dest: Reg,
    ) {
        let src_msb = if src == RIP { 0 } else { src.msb() };

        if src_msb != 0 || dest.msb() != 0 || x64 != 0 || rex_prefix != 0 {
            self.emit_rex(x64, dest.msb(), 0, src_msb);
        }

        self.put(opcode);
        self.emit_mem_base(src, disp, dest);
    }

    pub fn emit_movl_memq_reg(&mut self, src: Reg, disp: i32, dest: Reg) {
        self.emit_mov_membaseq_reg(0, 0, 0x8b, src, disp, dest);
    }

    pub fn emit_movq_memq_reg(&mut self, src: Reg, disp: i32, dest: Reg) {
        self.emit_mov_membaseq_reg(0, 1, 0x8b, src, disp, dest);
    }

    pub fn emit_movq_reg_memq(&mut self, src: Reg, dest: Reg, disp: i32) {
        self.emit_mov_reg_membaseq(0x89, 1, src, dest, disp);
    }

    pub fn emit_movl_reg_memq(&mut self, src: Reg, dest: Reg, disp: i32) {
        self.emit_mov_reg_membaseq(0x89, 0, src, dest, disp);
    }

    pub fn push_imm(&mut self, v: i64) {
        self.put(0x68);

        self.put_4(v as u32);
    }

    fn emit_mov_reg_membaseq(&mut self, opcode: u8, x64: u8, src: Reg, dest: Reg, disp: i32) {
        let dest_msb = if dest == RIP { 0 } else { dest.msb() };

        if dest_msb != 0 || src.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, src.msb(), 0, dest_msb);
        }

        self.put(opcode);
        self.emit_mem_base(dest, disp, src);
    }

    pub fn emit_movzbl_reg_reg(&mut self, src: Reg, dest: Reg) {
        if src.msb() != 0 || dest.msb() != 0 || !src.is_basic_reg() {
            self.emit_rex(0, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(0xb6);

        self.emit_modrm(0b11, dest.and7(), src.and7());
    }

    pub fn emit_setb_reg_parity(&mut self, reg: Reg, parity: bool) {
        if reg.msb() != 0 || !reg.is_basic_reg() {
            self.emit_rex(0, 0, 0, reg.msb());
        }

        let opcode = if parity { 0x9a } else { 0x9b };

        self.put(0x0f);
        self.put(opcode);
        self.emit_modrm(0b11, 0, reg.and7());
    }
    pub fn emit_movb_reg_reg(&mut self, src: Reg, dest: Reg) {
        if src.msb() != 0 || dest.msb() != 0 || !src.is_basic_reg() {
            self.emit_rex(0, dest.msb(), 0, src.msb());
        }

        self.put(0x88);
        self.emit_modrm(0b11, src.and7(), dest.and7());
    }

    pub fn emit_setb_reg(&mut self, op: CondCode, reg: Reg) {
        if reg.msb() != 0 || !reg.is_basic_reg() {
            self.emit_rex(0, 0, 0, reg.msb());
        }

        let op = match op {
            CondCode::Less => 0x9c,
            CondCode::LessEq => 0x9e,
            CondCode::Greater => 0x9f,
            CondCode::GreaterEq => 0x9d,
            CondCode::UnsignedGreater => 0x97,   // above
            CondCode::UnsignedGreaterEq => 0x93, // above or equal
            CondCode::UnsignedLess => 0x92,      // below
            CondCode::UnsignedLessEq => 0x96,    // below or equal
            CondCode::Zero | CondCode::Equal => 0x94,
            CondCode::NonZero | CondCode::NotEqual => 0x95,
        };

        self.put(0x0f);
        self.put(op);
        self.emit_modrm(0b11, 0, reg.and7());
    }

    fn emit_membase(&mut self, dest: Reg, src: &Membase) {
        match src {
            &Membase::Local(offset) => {
                self.emit_mem_base(RBP, offset, dest);
            }

            &Membase::Base(base, disp) => {
                self.emit_mem_base(base, disp, dest);
            }

            &Membase::Index(base, index, scale, disp) => {
                self.emit_membase_with_index_and_scale(base, index, scale, disp, dest);
            }

            &Membase::Offset(index, scale, disp) => {
                self.emit_membase_without_base(index, scale, disp, dest);
            }
        }
    }

    fn emit_membase_without_base(&mut self, index: Reg, scale: i32, disp: i32, dest: Reg) {
        assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

        let scale = match scale {
            8 => 3,
            4 => 2,
            2 => 1,
            _ => 0,
        };

        self.emit_modrm(0, dest.and7(), 4);
        self.emit_sib(scale, index.and7(), 5);
        self.put_4(disp as u32);
    }

    pub fn lea(&mut self, dest: Reg, src: Membase) {
        self.emit_rex_membase(1, dest, &src);
        self.put(0x8D);
        self.emit_membase(dest, &src);
    }

    pub fn emit_cmp_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        self.emit_alu_reg_reg(x64, 0x39, src, dest);
    }

    pub fn emit_cmp_membase_reg(&mut self, mode: Type, base: Reg, disp: i32, dest: Reg) {
        let base_msb = if base == RIP { 0 } else { base.msb() };

        let (x64, opcode) = match mode {
            I8 => (0, 0x38),
            I32 => (0, 0x39),
            F32 | F64 => unreachable!(),
            I64 => (1, 0x39),
            _ => unreachable!(),
        };

        if x64 != 0 || dest.msb() != 0 || base_msb != 0 {
            self.emit_rex(x64, dest.msb(), 0, base_msb);
        }

        self.put(opcode);
        self.emit_mem_base(base, disp, dest);
    }

    fn emit_mem_base(&mut self, base: Reg, disp: i32, dest: Reg) {
        if base == RSP || base == R12 {
            if disp == 0 {
                self.emit_modrm(0, dest.and7(), RSP.and7());
                self.emit_sib(0, RSP.and7(), RSP.and7());
            } else if fits_i8(disp) {
                self.emit_modrm(1, dest.and7(), RSP.and7());
                self.emit_sib(0, RSP.and7(), RSP.and7());
                self.put(disp as u8);
            } else {
                self.emit_modrm(2, dest.and7(), RSP.and7());
                self.emit_sib(0, RSP.and7(), RSP.and7());
                self.put_4(disp as u32);
            }
        } else if disp == 0 && base != RBP && base != R13 && base != RIP {
            self.emit_modrm(0, dest.and7(), base.and7());
        } else if base == RIP {
            self.emit_modrm(0, dest.and7(), RBP.and7());
            self.put_4(disp as u32);
        } else if fits_i8(disp) {
            self.emit_modrm(1, dest.and7(), base.and7());
            self.put(disp as u8);
        } else {
            self.emit_modrm(2, dest.and7(), base.and7());
            self.put_4(disp as u32);
        }
    }

    pub fn emit_mov_reg_membaseindex(
        &mut self,
        mode: Type,
        src: Reg,
        base: Reg,
        index: Reg,
        scale: i32,
        disp: i32,
    ) {
        assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

        let (x64, opcode) = match mode {
            I8 => (0, 0x88),
            I32 => (0, 0x89),
            I64 => (1, 0x89),
            F32 | F64 => unreachable!(),
            _ => unreachable!(),
        };

        if x64 != 0 || src.msb() != 0 || index.msb() != 0 || base.msb() != 0 {
            self.emit_rex(x64, src.msb(), index.msb(), base.msb());
        }

        self.put(opcode);
        self.emit_membase_with_index_and_scale(base, index, scale, disp, src);
    }

    pub fn emit_mov_membaseindex_reg(
        &mut self,
        mode: Type,
        base: Reg,
        index: Reg,
        scale: i32,
        disp: i32,
        dest: Reg,
    ) {
        assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);
        assert!(mode.size() == scale as usize);

        let (x64, opcode) = match mode {
            I8 => (0, 0x8a),
            I32 => (0, 0x8b),
            I64 => (1, 0x8b),
            F32 | F64 => unreachable!(),
            _ => unreachable!(),
        };

        if x64 != 0 || dest.msb() != 0 || index.msb() != 0 || base.msb() != 0 {
            self.emit_rex(x64, dest.msb(), index.msb(), base.msb());
        }

        self.put(opcode);
        self.emit_membase_with_index_and_scale(base, index, scale, disp, dest);
    }

    fn emit_membase_with_index_and_scale(
        &mut self,
        base: Reg,
        index: Reg,
        scale: i32,
        disp: i32,
        dest: Reg,
    ) {
        assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

        let scale = match scale {
            8 => 3,
            4 => 2,
            2 => 1,
            _ => 0,
        };

        if disp == 0 {
            self.emit_modrm(0, dest.and7(), 4);
            self.emit_sib(scale, index.and7(), base.and7());
        } else if fits_i8(disp) {
            self.emit_modrm(1, dest.and7(), 4);
            self.emit_sib(scale, index.and7(), base.and7());
            self.put(disp as u8);
        } else {
            self.emit_modrm(2, dest.and7(), 4);
            self.emit_sib(scale, index.and7(), base.and7());
            self.put_4(disp as u32);
        }
    }

    pub fn emit_rex_membase(&mut self, x64: u8, dest: Reg, src: &Membase) {
        assert!(x64 == 0 || x64 == 1);

        let (base_msb, index_msb) = match src {
            &Membase::Local(_) => (RSP.msb(), 0),
            &Membase::Base(base, _) => {
                let base_msb = if base == RIP { 0 } else { base.msb() };
                (base_msb, 0)
            }

            &Membase::Index(base, index, _, _) => (base.msb(), index.msb()),
            &Membase::Offset(index, _, _) => (0, index.msb()),
        };

        if dest.msb() != 0 || index_msb != 0 || base_msb != 0 || x64 != 0 {
            self.emit_rex(x64, dest.msb(), index_msb, base_msb);
        }
    }
    pub fn emit_rex(&mut self, w: u8, r: u8, x: u8, b: u8) {
        assert!(w == 0 || w == 1);
        assert!(r == 0 || r == 1);
        assert!(x == 0 || x == 1);
        assert!(b == 0 || b == 1);

        self.put(0x4 << 4 | w << 3 | r << 2 | x << 1 | b);
    }

    pub fn emit_prolog(&mut self) {
        self.emit_push_reg(RBP);
        self.emit_mov_reg_reg(1, RSP, RBP);
    }

    pub fn emit_epilog(&mut self) {
        self.emit_pop_reg(RBP);
    }

    pub fn load_float(&mut self, mode: Type, dest: FReg, imm: f64) {
        let pos = self.data().len() as i32;
        match mode {
            F32 => {
                let off = self.dseg_mut().add_float(imm as f32);
                self.movss_load(dest, Membase::Base(RIP, -(off + pos + 8)));
            }
            F64 => {
                let off = self.dseg_mut().add_double(imm);
                self.movsd_load(dest, Membase::Base(RIP, -(off + pos + 8)));
            }
            _ => unimplemented!(),
        }
    }

    pub fn load_int(&mut self, mode: Type, dest: Reg, imm: i64) {
        match mode {
            I8 | I32 => {
                self.emit_movl_imm_reg(imm as i32, dest);
            }
            I64 => {
                self.emit_movq_imm64_reg(imm, dest);
            }
            _ => unreachable!(),
        }
    }

    pub fn emit_mov_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        if x64 != 0 || src.msb() != 0 || dest.msb() != 0 {
            self.emit_rex(x64, src.msb(), 0, dest.msb());
        }
        self.put(0x89);
        self.emit_modrm(0b11, src.and7(), dest.and7());
    }

    pub fn emit_push_reg(&mut self, reg: Reg) {
        if reg.msb() != 0 {
            self.emit_rex(0, 0, 0, 1);
        }

        self.put(0x50 + reg.and7())
    }

    pub fn emit_pop_reg(&mut self, reg: Reg) {
        if reg.msb() != 0 {
            self.emit_rex(0, 0, 0, 1);
        }

        self.put(0x58 + reg.and7())
    }

    pub fn ret(&mut self) {
        self.put(0xc3);
    }

    pub fn emit_relative_call(&mut self, offset: int) {
        self.put(0xe8);

        self.put_4(offset as uint);
    }

    pub fn emit_call_ptr(&mut self, ptr: i64) {
        self.put(0xe8);
        self.put_8(ptr as ulong);
    }

    pub fn emit_call_reg(&mut self, dest: Reg) {
        if dest.msb() != 0 {
            self.emit_rex(0, 0, 0, dest.msb());
        }

        self.put(0xff);
        self.emit_modrm(0b11, 0b10, dest.and7());
    }

    pub fn emit_movl_imm_reg(&mut self, imm: i32, reg: Reg) {
        if reg.msb() != 0 {
            self.emit_rex(0, 0, 0, 1);
        }

        self.put((0xB8 as u8) + reg.and7());
        let bytes: [u8; 4] = unsafe { transmute(imm) };
        self.put_slice(&bytes);
    }

    pub fn emit_movq_imm64_reg(&mut self, imm: i64, reg: Reg) {
        self.emit_rex(1, 0, 0, reg.msb());
        self.put(0xb8 + reg.and7());
        let bytes: [u8; 8] = unsafe { transmute(imm) };
        self.put_slice(&bytes);
    }

    pub fn emit_imul_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(0xaf);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }

    pub fn emit_idiv_reg_reg(&mut self, x64: u8, reg: Reg) {
        if reg.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, 0, 0, reg.msb());
        }

        self.put(0xf7);
        self.emit_modrm(0b11, 0b111, reg.and7());
    }

    pub fn emit_cdq(&mut self) {
        self.put(0x99);
    }

    pub fn emit_cqo(&mut self) {
        self.emit_rex(1, 0, 0, 0);
        self.put(0x99);
    }

    pub fn emit_sub_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, src.msb(), 0, dest.msb());
        }

        self.put(0x29);
        self.emit_modrm(0b11, src.and7(), dest.and7());
    }

    pub fn emit_xorb_imm_reg(&mut self, imm: u8, dest: Reg) {
        self.emit_alub_imm_reg(0x80, 0x34, 0b110, imm, dest);
    }

    pub fn emit_andb_imm_reg(&mut self, imm: u8, dest: Reg) {
        self.emit_alub_imm_reg(0x80, 0x24, 0b100, imm, dest);
    }
    fn emit_alub_imm_reg(&mut self, opcode: u8, rax_opcode: u8, modrm_reg: u8, imm: u8, dest: Reg) {
        if dest == RAX {
            self.put(rax_opcode);
            self.put(imm);
        } else {
            if dest.msb() != 0 || !dest.is_basic_reg() {
                self.emit_rex(0, 0, 0, dest.msb());
            }

            self.put(opcode);
            self.emit_modrm(0b11, modrm_reg, dest.and7());
            self.put(imm);
        }
    }

    pub fn emit_sub_imm_membase(&mut self, mode: Type, base: Reg, imm: u8) {
        let (x64, opcode) = match mode {
            I64 => (1, 0x83),
            I32 => (0, 0x83),
            F32 | F64 => unreachable!(),
            I8 => (0, 0x80),
            _ => unreachable!(),
        };

        if x64 != 0 || base.msb() != 0 {
            self.emit_rex(x64, 0, 0, base.msb());
        }

        self.put(opcode);
        self.emit_modrm(0b00, 0b101, base.and7());
        self.put(imm);
    }

    pub fn emit_add_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, src.msb(), 0, dest.msb());
        }

        self.put(0x01);
        self.emit_modrm(0b11, src.and7(), dest.and7());
    }

    pub fn emit_movq_imm_reg(&mut self, imm: i32, reg: Reg) {
        self.emit_rex(1, 0, 0, reg.msb());
        self.put(0xc7);
        self.emit_modrm(0b11, 0, reg.and7());
        let bytes: [u8; 4] = unsafe { transmute(imm) };
        self.put_slice(&bytes);
    }

    pub fn emit_cmp_imm_reg(&mut self, mode: Type, imm: i32, reg: Reg) {
        let x64 = match mode {
            I8 | I32 => 0,
            I64 => 1,
            F32 | F64 => unreachable!(),
            _ => unreachable!(),
        };

        self.emit_aluq_imm_reg(x64, imm, reg, 0x3d, 0b111);
    }

    pub fn emit_subq_imm_reg(&mut self, imm: i32, reg: Reg) {
        self.emit_aluq_imm_reg(1, imm, reg, 0x2d, 0b101);
    }

    pub fn emit_addq_imm_reg(&mut self, imm: i32, reg: Reg) {
        self.emit_aluq_imm_reg(1, imm, reg, 0x05, 0);
    }

    pub fn emit_andq_imm_reg(&mut self, imm: i32, reg: Reg) {
        self.emit_aluq_imm_reg(1, imm, reg, 0x25, 4);
    }
    pub fn emit_or_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        self.emit_alu_reg_reg(x64, 0x09, src, dest);
    }

    pub fn emit_neg_reg(&mut self, x64: u8, reg: Reg) {
        self.emit_alul_reg(0xf7, 0b11, x64, reg);
    }

    pub fn emit_not_reg(&mut self, x64: u8, reg: Reg) {
        self.emit_alul_reg(0xf7, 0b10, x64, reg);
    }

    pub fn emit_not_reg_byte(&mut self, reg: Reg) {
        self.emit_alul_reg(0xf6, 0b10, 0, reg);
    }

    fn emit_alul_reg(&mut self, opcode: u8, modrm_reg: u8, x64: u8, reg: Reg) {
        if reg.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, 0, 0, reg.msb());
        }

        self.put(opcode);
        self.emit_modrm(0b11, modrm_reg, reg.and7());
    }

    pub fn emit_and_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        self.emit_alu_reg_reg(x64, 0x21, src, dest);
    }

    pub fn emit_xor_reg_reg(&mut self, x64: u8, src: Reg, dest: Reg) {
        self.emit_alu_reg_reg(x64, 0x31, src, dest);
    }

    pub fn emit_alu_reg_reg(&mut self, x64: u8, opcode: u8, src: Reg, dest: Reg) {
        if x64 != 0 || src.msb() != 0 || dest.msb() != 0 {
            self.emit_rex(x64, src.msb(), 0, dest.msb());
        }

        self.put(opcode);
        self.emit_modrm(0b11, src.and7(), dest.and7());
    }

    pub fn emit_aluq_imm_reg(
        &mut self,
        x64: u8,
        imm: i32,
        reg: Reg,
        rax_opcode: u8,
        modrm_reg: u8,
    ) {
        assert!(x64 == 0 || x64 == 1);
        if x64 != 0 || reg.msb() != 0 {
            self.emit_rex(x64, 0, 0, reg.msb());
        }

        if fits_i8(imm) {
            self.put(0x83);
            self.emit_modrm(0b11, modrm_reg, reg.and7());
            self.put(imm as u8);
        } else if reg == RAX {
            self.put(rax_opcode);
            self.put_4(imm as u32);
        } else {
            self.put(0x81);
            self.emit_modrm(0b11, modrm_reg, reg.and7());
            self.put_4(imm as u32);
        }
    }

    pub fn emit_modrm(&mut self, mode: u8, reg: u8, rm: u8) {
        assert!(mode < 4);
        assert!(reg < 8);
        assert!(rm < 8);

        self.put(mode << 6 | reg << 3 | rm);
    }

    pub fn movss(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(false, 0x10, dest, src);
    }

    pub fn sqrtsd(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(true, 0x51, dest, src);
    }

    pub fn sqrtss(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(false, 0x51, dest, src);
    }

    pub fn movsd(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(true, 0x10, dest, src);
    }

    pub fn xorps(&mut self, dest: FReg, src: Membase) {
        self.sse_float_freg_membase_66(false, 0x57, dest, src);
    }

    pub fn xorpd(&mut self, dest: FReg, src: Membase) {
        self.sse_float_freg_membase_66(true, 0x57, dest, src);
    }

    pub fn movsd_load(&mut self, dest: FReg, membase: Membase) {
        self.sse_float_freg_membase(true, 0x10, dest, membase);
    }

    pub fn movsd_store(&mut self, membase: Membase, src: FReg) {
        self.sse_float_freg_membase(true, 0x11, src, membase);
    }

    pub fn movss_store(&mut self, membase: Membase, src: FReg) {
        self.sse_float_freg_membase(false, 0x11, src, membase);
    }

    pub fn addss(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(false, 0x58, dest, src);
    }

    pub fn subss(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(false, 0x5c, dest, src);
    }
    pub fn subsd(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(true, 0x5c, dest, src);
    }
    pub fn mulsd(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(true, 0x59, dest, src);
    }
    pub fn mulss(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(false, 0x59, dest, src);
    }
    pub fn divss(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(false, 0x5e, dest, src);
    }
    pub fn divsd(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(true, 0x5e, dest, src);
    }

    pub fn addsd(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(true, 0x58, dest, src);
    }
    pub fn cvtsd2ss(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(true, 0x5a, dest, src);
    }

    pub fn cvtss2sd(&mut self, dest: FReg, src: FReg) {
        self.sse_float_freg_freg(false, 0x5a, dest, src);
    }

    pub fn cvtsi2ss(&mut self, dest: FReg, x64: u8, src: Reg) {
        self.sse_float_freg_reg(false, 0x2a, dest, x64, src);
    }

    pub fn cvtsi2sd(&mut self, dest: FReg, x64: u8, src: Reg) {
        self.sse_float_freg_reg(true, 0x2a, dest, x64, src);
    }

    pub fn cvttss2si(&mut self, x64: u8, dest: Reg, src: FReg) {
        self.sse_float_reg_freg(false, 0x2c, x64, dest, src);
    }

    pub fn cvttsd2si(&mut self, x64: u8, dest: Reg, src: FReg) {
        self.sse_float_reg_freg(true, 0x2c, x64, dest, src);
    }
    pub fn movss_load(&mut self, dest: FReg, membase: Membase) {
        self.sse_float_freg_membase(false, 0x10, dest, membase);
    }

    pub fn emit_shlq_reg(&mut self, imm: u8, dest: Reg) {
        self.emit_rex(1, 0, 0, dest.msb());
        self.put(0xC1);
        self.emit_modrm(0b11, 0b100, dest.and7());
        self.put(imm);
    }

    pub fn cmov(&mut self, x64: u8, dest: Reg, src: Reg, cond: CondCode) {
        let opcode = match cond {
            CondCode::Zero | CondCode::Equal => 0x44,
            CondCode::NonZero | CondCode::NotEqual => 0x45,
            CondCode::Greater => 0x4F,
            CondCode::GreaterEq => 0x4D,
            CondCode::Less => 0x4C,
            CondCode::LessEq => 0x4E,
            CondCode::UnsignedGreater => 0x47,   // above
            CondCode::UnsignedGreaterEq => 0x43, // above or equal
            CondCode::UnsignedLess => 0x42,      // below
            CondCode::UnsignedLessEq => 0x46,    // below or equal
        };

        if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(opcode);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }

    pub fn emit_shll_reg(&mut self, imm: u8, dest: Reg) {
        if dest.msb() != 0 {
            self.emit_rex(0, 0, 0, dest.msb());
        }

        self.put(0xC1);
        self.emit_modrm(0b11, 0b100, dest.and7());
        self.put(imm);
    }

    pub fn emit_shl_reg_cl(&mut self, x64: u8, dest: Reg) {
        if dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, 0, 0, dest.msb());
        }

        self.put(0xD3);
        self.emit_modrm(0b11, 0b100, dest.and7());
    }

    pub fn emit_shr_reg_cl(&mut self, x64: u8, dest: Reg) {
        if dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, 0, 0, dest.msb());
        }

        self.put(0xD3);
        self.emit_modrm(0b11, 0b101, dest.and7());
    }

    pub fn emit_shr_reg_imm(&mut self, x64: u8, dest: Reg, imm: u8) {
        if dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, 0, 0, dest.msb());
        }

        self.put(if imm == 1 { 0xD1 } else { 0xC1 });
        self.emit_modrm(0b11, 0b101, dest.and7());

        if imm != 1 {
            self.put(imm);
        }
    }

    pub fn emit_sar_reg_cl(&mut self, x64: u8, dest: Reg) {
        if dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, 0, 0, dest.msb());
        }

        self.put(0xD3);
        self.emit_modrm(0b11, 0b111, dest.and7());
    }

    pub fn emit_movzx_byte(&mut self, x64: u8, src: Reg, dest: Reg) {
        if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
            self.emit_rex(x64, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(0xb6);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }

    fn sse_float_freg_membase(&mut self, dbl: bool, op: u8, dest: FReg, src: Membase) {
        let prefix = if dbl { 0xf2 } else { 0xf3 };

        self.put(prefix);
        self.emit_rex_membase(0, Reg(dest.0), &src);
        self.put(0x0f);
        self.put(op);
        self.emit_membase(Reg(dest.0), &src);
    }

    fn sse_float_freg_freg(&mut self, dbl: bool, op: u8, dest: FReg, src: FReg) {
        let prefix = if dbl { 0xf2 } else { 0xf3 };

        self.put(prefix);

        if dest.msb() != 0 || src.msb() != 0 {
            self.emit_rex(0, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(op);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }

    fn sse_float_freg_reg(&mut self, dbl: bool, op: u8, dest: FReg, x64: u8, src: Reg) {
        let prefix = if dbl { 0xf2 } else { 0xf3 };

        self.put(prefix);

        if x64 != 0 || dest.msb() != 0 || src.msb() != 0 {
            self.emit_rex(x64, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(op);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }

    fn sse_float_freg_membase_66(&mut self, dbl: bool, op: u8, dest: FReg, src: Membase) {
        if dbl {
            self.put(0x66);
        }

        self.emit_rex_membase(0, Reg(dest.0), &src);
        self.put(0x0f);
        self.put(op);
        self.emit_membase(Reg(dest.0), &src);
    }

    fn sse_float_reg_freg(&mut self, dbl: bool, op: u8, x64: u8, dest: Reg, src: FReg) {
        let prefix = if dbl { 0xf2 } else { 0xf3 };

        self.put(prefix);

        if x64 != 0 || dest.msb() != 0 || src.msb() != 0 {
            self.emit_rex(x64, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(op);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }
    pub fn emit_jcc(&mut self, cond: CondCode, lbl: Label) {
        let opcode = match cond {
            CondCode::Zero | CondCode::Equal => 0x84,
            CondCode::NonZero | CondCode::NotEqual => 0x85,
            CondCode::Greater => 0x8F,
            CondCode::GreaterEq => 0x8D,
            CondCode::Less => 0x8C,
            CondCode::LessEq => 0x8E,
            CondCode::UnsignedGreater => 0x87,   // above
            CondCode::UnsignedGreaterEq => 0x83, // above or equal
            CondCode::UnsignedLess => 0x82,      // below
            CondCode::UnsignedLessEq => 0x86,    // below or equal
        };

        self.put(0x0f);
        self.put(opcode);
        self.emit_label(lbl);
    }

    pub fn emit_label(&mut self, lbl: Label) {
        let value = self.labels[lbl.index()];

        match value {
            // backward jumps already know their target
            Some(idx) => {
                let current = self.data().len() + 4;
                let target = idx;

                let diff = -((current - target) as i32);
                self.put_4(diff as u32);
            }

            // forward jumps do not know their target yet
            // we need to do this later...
            None => {
                let pos = self.data().len();
                self.put_4(0);
                self.jumps.push(ForwardJump { at: pos, to: lbl });
            }
        }
    }

    pub fn emit_testl_reg_reg(&mut self, op1: Reg, op2: Reg) {
        if op1.msb() != 0 || op2.msb() != 0 {
            self.emit_rex(0, op1.msb(), 0, op2.msb());
        }

        self.put(0x85);
        self.emit_modrm(0b11, op1.and7(), op2.and7());
    }

    pub fn emit_movsx(&mut self, src: Reg, dest: Reg) {
        self.emit_rex(1, dest.msb(), 0, src.msb());

        self.put(0x63);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }

    pub fn emit_jmp(&mut self, lbl: Label) {
        self.put(0xe9);
        self.emit_label(lbl);
    }

    pub fn emit_jmp_reg(&mut self, reg: Reg) {
        if reg.msb() != 0 {
            self.emit_rex(0, 0, 0, reg.msb());
        }
        self.put(0xFF);
        self.emit_modrm(0b11, 0b100, reg.and7());
    }
    pub fn testl_reg_membase(&mut self, dest: Reg, src: Membase) {
        self.emit_rex_membase(0, dest, &src);
        self.put(0x85);
        self.emit_membase(dest, &src);
    }

    pub fn pxor(&mut self, dest: FReg, src: FReg) {
        self.put(0x66);

        if dest.msb() != 0 || src.msb() != 0 {
            self.emit_rex(0, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(0xef);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }
    pub fn ucomiss(&mut self, dest: FReg, src: FReg) {
        self.sse_cmp(false, dest, src);
    }

    pub fn ucomisd(&mut self, dest: FReg, src: FReg) {
        self.sse_cmp(true, dest, src);
    }
    fn sse_cmp(&mut self, dbl: bool, dest: FReg, src: FReg) {
        if dbl {
            self.put(0x66);
        }

        if dest.msb() != 0 || src.msb() != 0 {
            self.emit_rex(0, dest.msb(), 0, src.msb());
        }

        self.put(0x0f);
        self.put(0x2e);
        self.emit_modrm(0b11, dest.and7(), src.and7());
    }
}
