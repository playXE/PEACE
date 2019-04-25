use super::assembler::*;
use super::assemblerx64 as buf;
use super::constants_x64::*;
use super::*;

pub fn fits_i32(n: i64) -> bool {
    n == (n as i32) as i64
}

impl Assembler {
    pub fn load_int_const(&mut self, mode: MachineMode, dest: Register, imm: i64) {
        match mode {
            MachineMode::Int8 | MachineMode::Int32 => {
                buf::emit_movl_imm_reg(self, imm as i32, dest)
            }
            MachineMode::Int64 | MachineMode::Ptr => {
                buf::emit_movq_imm64_reg(self, imm, dest);
            }
            MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
        }
    }

    pub fn load_float_const(&mut self, mode: MachineMode, dest: XMMRegister, imm: f64) {
        let pos = self.pos() as i32;

        match mode {
            MachineMode::Float32 => {
                let off = self.dseg.add_float(imm as f32);
                buf::movss_load(self, dest, Mem::Base(RIP, -(off + pos + 8)));
            }

            MachineMode::Float64 => {
                let off = self.dseg.add_double(imm);
                buf::movsd_load(self, dest, Mem::Base(RIP, -(off + pos + 8)));
            }

            _ => unreachable!(),
        }
    }

    pub fn load_true(&mut self, dest: Register) {
        buf::emit_movl_imm_reg(self, 1, dest);
    }

    pub fn load_false(&mut self, dest: Register) {
        buf::emit_movl_imm_reg(self, 0, dest);
    }

    pub fn int_neg(&mut self, mode: MachineMode, dest: Register, src: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        buf::emit_neg_reg(self, x64, src);

        if dest != src {
            buf::emit_mov_reg_reg(self, x64, src, dest);
        }
    }

    pub fn int_not(&mut self, mode: MachineMode, dest: Register, src: Register) {
        let x64 = match mode {
            MachineMode::Int8 => {
                buf::emit_not_reg_byte(self, src);
                0
            }

            MachineMode::Int32 => {
                buf::emit_not_reg(self, 0, src);
                0
            }

            MachineMode::Int64 => {
                buf::emit_not_reg(self, 1, src);

                1
            }

            _ => unimplemented!(),
        };

        if dest != src {
            buf::emit_mov_reg_reg(self, x64, src, dest);
        }
    }

    pub fn bool_not(&mut self, dest: Register, src: Register) {
        buf::emit_xorb_imm_reg(self, 1, src);
        buf::emit_andb_imm_reg(self, 1, src);

        if dest != src {
            buf::emit_mov_reg_reg(self, 0, src, dest);
        }
    }

    pub fn float_add(
        &mut self,
        mode: MachineMode,
        dest: XMMRegister,
        lhs: XMMRegister,
        rhs: XMMRegister,
    ) {
        match mode {
            MachineMode::Float32 => buf::addss(self, lhs, rhs),
            MachineMode::Float64 => buf::addsd(self, lhs, rhs),
            _ => unimplemented!(),
        }

        if dest != lhs {
            self.copy_freg(mode, dest, lhs);
        }
    }

    pub fn float_sub(
        &mut self,
        mode: MachineMode,
        dest: XMMRegister,
        lhs: XMMRegister,
        rhs: XMMRegister,
    ) {
        match mode {
            MachineMode::Float32 => buf::subss(self, lhs, rhs),
            MachineMode::Float64 => buf::subsd(self, lhs, rhs),
            _ => unimplemented!(),
        }

        if dest != lhs {
            self.copy_freg(mode, dest, lhs);
        }
    }

    pub fn float_mul(
        &mut self,
        mode: MachineMode,
        dest: XMMRegister,
        lhs: XMMRegister,
        rhs: XMMRegister,
    ) {
        match mode {
            MachineMode::Float32 => buf::mulss(self, lhs, rhs),
            MachineMode::Float64 => buf::mulsd(self, lhs, rhs),
            _ => unimplemented!(),
        }

        if dest != lhs {
            self.copy_freg(mode, dest, lhs);
        }
    }

    pub fn float_div(
        &mut self,
        mode: MachineMode,
        dest: XMMRegister,
        lhs: XMMRegister,
        rhs: XMMRegister,
    ) {
        match mode {
            MachineMode::Float32 => buf::divss(self, lhs, rhs),
            MachineMode::Float64 => buf::divsd(self, lhs, rhs),
            _ => unimplemented!(),
        }

        if dest != lhs {
            self.copy_freg(mode, dest, lhs);
        }
    }

    pub fn float_neg(&mut self, mode: MachineMode, dest: XMMRegister, src: XMMRegister) {
        let (fst, snd) = if mode == MachineMode::Float32 {
            (1i32 << 31, 0)
        } else {
            (0, 1i32 << 31)
        };

        // align MMX data to 16 bytes
        self.dseg.align(16);
        self.dseg.add_int(0);
        self.dseg.add_int(0);
        self.dseg.add_int(snd);
        let disp = self.dseg.add_int(fst);

        let pos = self.pos() as i32;
        let mem = Mem::Base(RIP, 0);

        match mode {
            MachineMode::Float32 => buf::xorps(self, src, mem),
            MachineMode::Float64 => buf::xorpd(self, src, mem),
            _ => unimplemented!(),
        }

        let after = self.pos() as i32;
        let len = after - pos;

        let offset = -(disp + pos + len);
        self.emit_u32_at(after - 4, offset as u32);

        if dest != src {
            self.copy_freg(mode, dest, src);
        }
    }

    pub fn load_mem(&mut self, mode: MachineMode, dest: Reg, mem: Mem) {
        match mem {
            Mem::Local(offset) => match mode {
                MachineMode::Int8 => buf::emit_movzbl_memq_reg(self, RBP, offset, dest.reg()),
                MachineMode::Int32 => buf::emit_movl_memq_reg(self, RBP, offset, dest.reg()),
                MachineMode::Int64 | MachineMode::Ptr => {
                    buf::emit_movq_memq_reg(self, RBP, offset, dest.reg())
                }
                MachineMode::Float32 => buf::movss_load(self, dest.freg(), mem),
                MachineMode::Float64 => buf::movsd_load(self, dest.freg(), mem),
            },

            Mem::Base(base, disp) => match mode {
                MachineMode::Int8 => buf::emit_movzbl_memq_reg(self, base, disp, dest.reg()),
                MachineMode::Int32 => buf::emit_movl_memq_reg(self, base, disp, dest.reg()),
                MachineMode::Int64 | MachineMode::Ptr => {
                    buf::emit_movq_memq_reg(self, base, disp, dest.reg())
                }
                MachineMode::Float32 => buf::movss_load(self, dest.freg(), mem),
                MachineMode::Float64 => buf::movsd_load(self, dest.freg(), mem),
            },

            Mem::Index(base, index, scale, disp) => match mode {
                MachineMode::Int8 => {
                    assert!(scale == 1);
                    buf::emit_movzx_memindex_byte_reg(self, 0, base, index, disp, dest.reg())
                }

                MachineMode::Int32 | MachineMode::Int64 | MachineMode::Ptr => {
                    buf::emit_mov_memindex_reg(self, mode, base, index, scale, disp, dest.reg())
                }

                MachineMode::Float32 => buf::movss_load(self, dest.freg(), mem),
                MachineMode::Float64 => buf::movsd_load(self, dest.freg(), mem),
            },

            Mem::Offset(_, _, _) => unimplemented!(),
        }
    }

    pub fn float_sqrt(&mut self, mode: MachineMode, dest: XMMRegister, src: XMMRegister) {
        match mode {
            MachineMode::Float32 => buf::sqrtss(self, dest, src),
            MachineMode::Float64 => buf::sqrtsd(self, dest, src),
            _ => unreachable!(),
        }
    }

    pub fn copy_reg(&mut self, mode: MachineMode, dest: Register, src: Register) {
        let x64 = match mode {
            MachineMode::Int8 | MachineMode::Int32 => 0,
            MachineMode::Int64 | MachineMode::Ptr => 1,
            MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
        };

        buf::emit_mov_reg_reg(self, x64, src, dest);
    }

    pub fn copy_pc(&mut self, dest: Register) {
        buf::lea(self, dest, Mem::Base(RIP, 0));
    }

    pub fn copy_ra(&mut self, dest: Register) {
        self.load_mem(MachineMode::Ptr, Reg::Gpr(dest), Mem::Base(RSP, 0));
    }

    fn emit_barrier(&mut self, src: Register, card_table_offset: usize) {
        buf::emit_shr_reg_imm(self, 1, src, 9);

        // test if card table offset fits into displacement of memory store
        if card_table_offset <= 0x7FFF_FFFF {
            // emit mov [card_table_offset + base], 0
            buf::emit_movb_imm_memq(self, 0, src, card_table_offset as i32);
        } else {
            let scratch = R11;
            self.load_int_const(MachineMode::Ptr, scratch, card_table_offset as i64);
            buf::emit_movb_imm_memscaleq(self, 0, src, scratch, 0);
        }
    }

    pub fn store_mem(&mut self, mode: MachineMode, mem: Mem, src: Reg) {
        match mem {
            Mem::Local(offset) => match mode {
                MachineMode::Int8 => buf::emit_movb_reg_memq(self, src.reg(), RBP, offset),
                MachineMode::Int32 => buf::emit_movl_reg_memq(self, src.reg(), RBP, offset),
                MachineMode::Int64 | MachineMode::Ptr => {
                    buf::emit_movq_reg_memq(self, src.reg(), RBP, offset)
                }
                MachineMode::Float32 => buf::movss_store(self, mem, src.freg()),
                MachineMode::Float64 => buf::movsd_store(self, mem, src.freg()),
            },

            Mem::Base(base, disp) => match mode {
                MachineMode::Int8 => buf::emit_movb_reg_memq(self, src.reg(), base, disp),
                MachineMode::Int32 => buf::emit_movl_reg_memq(self, src.reg(), base, disp),
                MachineMode::Int64 | MachineMode::Ptr => {
                    buf::emit_movq_reg_memq(self, src.reg(), base, disp)
                }
                MachineMode::Float32 => buf::movss_store(self, mem, src.freg()),
                MachineMode::Float64 => buf::movsd_store(self, mem, src.freg()),
            },

            Mem::Index(base, index, scale, disp) => match mode {
                MachineMode::Int8 | MachineMode::Int32 | MachineMode::Int64 | MachineMode::Ptr => {
                    buf::emit_mov_reg_memindex(self, mode, src.reg(), base, index, scale, disp)
                }

                MachineMode::Float32 => buf::movss_store(self, mem, src.freg()),
                MachineMode::Float64 => buf::movsd_store(self, mem, src.freg()),
            },

            Mem::Offset(_, _, _) => unimplemented!(),
        }
    }

    pub fn copy_sp(&mut self, dest: Register) {
        self.copy_reg(MachineMode::Ptr, dest, RSP);
    }

    pub fn set_sp(&mut self, src: Register) {
        self.copy_reg(MachineMode::Ptr, RSP, src);
    }

    pub fn copy_freg(&mut self, mode: MachineMode, dest: XMMRegister, src: XMMRegister) {
        match mode {
            MachineMode::Float32 => buf::movss(self, dest, src),
            MachineMode::Float64 => buf::movsd(self, dest, src),
            _ => unreachable!(),
        }
    }

    pub fn extend_int_long(&mut self, dest: Register, src: Register) {
        buf::emit_movsx(self, src, dest);
    }

    pub fn extend_byte(&mut self, mode: MachineMode, dest: Register, src: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        buf::emit_movzx_byte(self, x64, src, dest);
    }

    pub fn set(&mut self, dest: Register, op: CondCode) {
        buf::emit_setb_reg(self, op, dest);
        buf::emit_movzbl_reg_reg(self, dest, dest);
    }

    pub fn cmp_mem(&mut self, mode: MachineMode, mem: Mem, rhs: Register) {
        match mem {
            Mem::Local(offset) => buf::emit_cmp_mem_reg(self, mode, RBP, offset, rhs),
            Mem::Base(base, disp) => buf::emit_cmp_mem_reg(self, mode, base, disp, rhs),
            Mem::Index(base, index, scale, disp) => {
                buf::emit_cmp_memindex_reg(self, mode, base, index, scale, disp, rhs)
            }
            Mem::Offset(_, _, _) => unimplemented!(),
        }
    }

    pub fn cmp_mem_imm(&mut self, mode: MachineMode, mem: Mem, imm: i32) {
        match mem {
            Mem::Local(_) => unimplemented!(),
            Mem::Base(base, disp) => buf::emit_cmp_mem_imm(self, mode, base, disp, imm),
            Mem::Index(_, _, _, _) => unimplemented!(),
            Mem::Offset(_, _, _) => unimplemented!(),
        }
    }

    pub fn cmp_reg(&mut self, mode: MachineMode, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int8 | MachineMode::Int32 => 0,
            MachineMode::Int64 | MachineMode::Ptr => 1,
            MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
        };

        buf::emit_cmp_reg_reg(self, x64, rhs, lhs);
    }

    pub fn cmp_reg_imm(&mut self, mode: MachineMode, lhs: Register, imm: i32) {
        buf::emit_cmp_imm_reg(self, mode, imm, lhs);
    }

    pub fn float_cmp(
        &mut self,
        mode: MachineMode,
        dest: Register,
        lhs: XMMRegister,
        rhs: XMMRegister,
        cond: CondCode,
    ) {
        let scratch = &R11;

        match cond {
            CondCode::Equal | CondCode::NotEqual => {
                let init = if cond == CondCode::Equal { 0 } else { 1 };

                self.load_int_const(MachineMode::Int32, *scratch, init);
                self.load_int_const(MachineMode::Int32, dest, 0);

                match mode {
                    MachineMode::Float32 => buf::ucomiss(self, lhs, rhs),
                    MachineMode::Float64 => buf::ucomisd(self, lhs, rhs),
                    _ => unreachable!(),
                }

                let parity = if cond == CondCode::Equal { false } else { true };

                buf::emit_setb_reg_parity(self, dest, parity);
                buf::cmov(self, 0, dest, *scratch, CondCode::NotEqual);
            }

            CondCode::Greater | CondCode::GreaterEq => {
                self.load_int_const(MachineMode::Int32, dest, 0);

                match mode {
                    MachineMode::Float32 => buf::ucomiss(self, lhs, rhs),
                    MachineMode::Float64 => buf::ucomisd(self, lhs, rhs),
                    _ => unreachable!(),
                }

                let cond = match cond {
                    CondCode::Greater => CondCode::UnsignedGreater,
                    CondCode::GreaterEq => CondCode::UnsignedGreaterEq,
                    _ => unreachable!(),
                };

                buf::emit_setb_reg(self, cond, dest);
            }

            CondCode::Less | CondCode::LessEq => {
                self.load_int_const(MachineMode::Int32, dest, 0);

                match mode {
                    MachineMode::Float32 => buf::ucomiss(self, rhs, lhs),
                    MachineMode::Float64 => buf::ucomisd(self, rhs, lhs),
                    _ => unreachable!(),
                }

                let cond = match cond {
                    CondCode::Less => CondCode::UnsignedGreater,
                    CondCode::LessEq => CondCode::UnsignedGreaterEq,
                    _ => unreachable!(),
                };

                buf::emit_setb_reg(self, cond, dest);
            }

            _ => unreachable!(),
        }
    }

    pub fn float_cmp_nan(&mut self, mode: MachineMode, dest: Register, src: XMMRegister) {
        self.load_int_const(MachineMode::Int32, dest, 0);

        match mode {
            MachineMode::Float32 => buf::ucomiss(self, src, src),
            MachineMode::Float64 => buf::ucomisd(self, src, src),
            _ => unreachable!(),
        }

        buf::emit_setb_reg_parity(self, dest, true);
    }

    pub fn cmp_zero(&mut self, mode: MachineMode, lhs: Register) {
        buf::emit_cmp_imm_reg(self, mode, 0, lhs);
    }

    pub fn test_and_jump_if(&mut self, cond: CondCode, reg: Register, lbl: Label) {
        assert!(cond == CondCode::Zero || cond == CondCode::NonZero);

        buf::emit_testl_reg_reg(self, reg, reg);
        self.jump_if(cond, lbl);
    }

    pub fn jump_if(&mut self, cond: CondCode, lbl: Label) {
        buf::emit_jcc(self, cond, lbl);
    }

    pub fn jump(&mut self, lbl: Label) {
        buf::emit_jmp(self, lbl);
    }

    pub fn jump_reg(&mut self, reg: Register) {
        buf::emit_jmp_reg(self, reg);
    }

    pub fn int_div(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        self.div_common(mode, dest, lhs, rhs, RAX);
    }

    pub fn int_mod(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        self.div_common(mode, dest, lhs, rhs, RDX);
    }

    fn div_common(
        &mut self,
        mode: MachineMode,
        dest: Register,
        lhs: Register,
        rhs: Register,
        result: Register,
    ) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        if lhs != RAX {
            assert!(rhs != RAX);
            buf::emit_mov_reg_reg(self, x64, lhs, RAX);
        }

        if x64 != 0 {
            buf::emit_cqo(self);
        } else {
            buf::emit_cdq(self);
        }

        buf::emit_idiv_reg_reg(self, x64, rhs);

        if dest != result {
            buf::emit_mov_reg_reg(self, x64, result, dest);
        }
    }

    pub fn int_mul(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        buf::emit_imul_reg_reg(self, x64, rhs, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_add(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 | MachineMode::Ptr => 1,
            _ => unimplemented!(),
        };

        buf::emit_add_reg_reg(self, x64, rhs, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_add_imm(&mut self, mode: MachineMode, dest: Register, lhs: Register, value: i64) {
        if !fits_i32(value) {
            assert!(mode == MachineMode::Int64 || mode == MachineMode::Ptr);
            let reg_size = R11;
            self.load_int_const(MachineMode::Ptr, reg_size, value);
            self.int_add(mode, dest, lhs, reg_size);
            return;
        }

        let x64 = match mode {
            MachineMode::Int64 | MachineMode::Ptr => 1,
            _ => unimplemented!(),
        };

        buf::emit_addq_imm_reg(self, value as i32, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_sub(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        buf::emit_sub_reg_reg(self, x64, rhs, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_shl(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        if rhs != RCX {
            assert!(lhs != RCX);
            buf::emit_mov_reg_reg(self, x64, rhs, RCX);
        }

        buf::emit_shl_reg_cl(self, x64, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_shr(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        if rhs != RCX {
            assert!(lhs != RCX);
            buf::emit_mov_reg_reg(self, x64, rhs, RCX);
        }

        buf::emit_shr_reg_cl(self, x64, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_sar(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        if rhs != RCX {
            assert!(lhs != RCX);
            buf::emit_mov_reg_reg(self, x64, rhs, RCX);
        }

        buf::emit_sar_reg_cl(self, x64, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_or(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        buf::emit_or_reg_reg(self, x64, rhs, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_and(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        buf::emit_and_reg_reg(self, x64, rhs, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_xor(&mut self, mode: MachineMode, dest: Register, lhs: Register, rhs: Register) {
        let x64 = match mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unimplemented!(),
        };

        buf::emit_xor_reg_reg(self, x64, rhs, lhs);

        if dest != lhs {
            buf::emit_mov_reg_reg(self, x64, lhs, dest);
        }
    }

    pub fn int_to_float(
        &mut self,
        dest_mode: MachineMode,
        dest: XMMRegister,
        src_mode: MachineMode,
        src: Register,
    ) {
        buf::pxor(self, dest, dest);

        let x64 = match src_mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unreachable!(),
        };

        match dest_mode {
            MachineMode::Float32 => buf::cvtsi2ss(self, dest, x64, src),
            MachineMode::Float64 => buf::cvtsi2sd(self, dest, x64, src),
            _ => unreachable!(),
        }
    }

    pub fn float_to_int(
        &mut self,
        dest_mode: MachineMode,
        dest: Register,
        src_mode: MachineMode,
        src: XMMRegister,
    ) {
        let x64 = match dest_mode {
            MachineMode::Int32 => 0,
            MachineMode::Int64 => 1,
            _ => unreachable!(),
        };

        match src_mode {
            MachineMode::Float32 => buf::cvttss2si(self, x64, dest, src),
            MachineMode::Float64 => buf::cvttsd2si(self, x64, dest, src),
            _ => unreachable!(),
        }
    }

    pub fn float_to_double(&mut self, dest: XMMRegister, src: XMMRegister) {
        buf::cvtss2sd(self, dest, src);
    }

    pub fn double_to_float(&mut self, dest: XMMRegister, src: XMMRegister) {
        buf::cvtsd2ss(self, dest, src);
    }
}

pub fn emit_or_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    emit_alu_reg_reg(buf, x64, 0x09, src, dest);
}

pub fn emit_and_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    emit_alu_reg_reg(buf, x64, 0x21, src, dest);
}

pub fn emit_xor_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    emit_alu_reg_reg(buf, x64, 0x31, src, dest);
}

fn emit_alu_reg_reg(buf: &mut Assembler, x64: u8, opcode: u8, src: Register, dest: Register) {
    if x64 != 0 || src.msb() != 0 || dest.msb() != 0 {
        emit_rex(buf, x64, src.msb(), 0, dest.msb());
    }

    emit_op(buf, opcode);
    emit_modrm(buf, 0b11, src.and7(), dest.and7());
}

pub fn emit_movl_imm_reg(buf: &mut Assembler, imm: i32, reg: Register) {
    if reg.msb() != 0 {
        emit_rex(buf, 0, 0, 0, 1);
    }

    emit_op(buf, (0xB8 as u8) + reg.and7());
    emit32(buf, imm as u32);
}

// mov 32bit immediate and sign-extend into 64bit-register
pub fn emit_movq_imm_reg(buf: &mut Assembler, imm: i32, reg: Register) {
    emit_rex(buf, 1, 0, 0, reg.msb());
    emit_op(buf, 0xc7);
    emit_modrm(buf, 0b11, 0, reg.and7());
    emit32(buf, imm as u32);
}

pub fn emit_movq_imm64_reg(buf: &mut Assembler, imm: i64, reg: Register) {
    emit_rex(buf, 1, 0, 0, reg.msb());
    emit_op(buf, 0xb8 + reg.and7());
    emit64(buf, imm as u64);
}

pub fn emit_movb_memq_reg(buf: &mut Assembler, src: Register, disp: i32, dest: Register) {
    let rex_prefix = if dest != RAX && dest != RBX && dest != RCX && dest != RDX {
        1
    } else {
        0
    };

    emit_mov_memq_reg(buf, rex_prefix, 0, 0x8a, src, disp, dest);
}

pub fn emit_movl_memq_reg(buf: &mut Assembler, src: Register, disp: i32, dest: Register) {
    emit_mov_memq_reg(buf, 0, 0, 0x8b, src, disp, dest);
}

pub fn emit_movzbl_memq_reg(buf: &mut Assembler, src: Register, disp: i32, dest: Register) {
    let src_msb = if src == RIP { 0 } else { src.msb() };

    if dest.msb() != 0 || src_msb != 0 {
        emit_rex(buf, 0, dest.msb(), 0, src_msb);
    }

    emit_op(buf, 0x0F);
    emit_op(buf, 0xB6);
    emit_membase(buf, src, disp, dest);
}

pub fn emit_movq_memq_reg(buf: &mut Assembler, src: Register, disp: i32, dest: Register) {
    emit_mov_memq_reg(buf, 0, 1, 0x8b, src, disp, dest);
}

fn emit_mov_memq_reg(
    buf: &mut Assembler,
    rex_prefix: u8,
    x64: u8,
    opcode: u8,
    src: Register,
    disp: i32,
    dest: Register,
) {
    let src_msb = if src == RIP { 0 } else { src.msb() };

    if src_msb != 0 || dest.msb() != 0 || x64 != 0 || rex_prefix != 0 {
        emit_rex(buf, x64, dest.msb(), 0, src_msb);
    }

    emit_op(buf, opcode);
    emit_membase(buf, src, disp, dest);
}

pub fn emit_movq_reg_memq(buf: &mut Assembler, src: Register, dest: Register, disp: i32) {
    emit_mov_reg_memq(buf, 0x89, 1, src, dest, disp);
}

pub fn emit_movl_reg_memq(buf: &mut Assembler, src: Register, dest: Register, disp: i32) {
    emit_mov_reg_memq(buf, 0x89, 0, src, dest, disp);
}

pub fn emit_movb_reg_memq(buf: &mut Assembler, src: Register, dest: Register, disp: i32) {
    let dest_msb = if dest == RIP { 0 } else { dest.msb() };

    if dest_msb != 0 || src.msb() != 0 || (src != RAX && src != RBX && src != RCX && src != RDX) {
        emit_rex(buf, 0, src.msb(), 0, dest.msb());
    }

    emit_op(buf, 0x88);
    emit_membase(buf, dest, disp, src);
}

pub fn emit_movb_imm_memq(buf: &mut Assembler, imm: u8, dest: Register, disp: i32) {
    let dest_msb = if dest == RIP { 0 } else { dest.msb() };

    if dest_msb != 0 {
        emit_rex(buf, 0, 0, 0, dest.msb());
    }

    emit_op(buf, 0xC6);
    emit_membase(buf, dest, disp, RAX);
    emit(buf, imm);
}

pub fn emit_movb_imm_memscaleq(
    buf: &mut Assembler,
    imm: u8,
    base: Register,
    index: Register,
    scale: u8,
) {
    if index.msb() != 0 || base.msb() != 0 {
        emit_rex(buf, 0, 0, index.msb(), base.msb());
    }

    emit_op(buf, 0xC6);
    emit_modrm(buf, 0b00, 0b000, 0b100);
    emit_sib(buf, scale, index.and7(), base.and7());
    emit(buf, imm);
}

pub fn emit_movq_ar(
    buf: &mut Assembler,
    base: Register,
    index: Register,
    scale: u8,
    dest: Register,
) {
    emit_mov_ar(buf, 1, 0x8b, base, index, scale, dest);
}

pub fn emit_movl_ar(
    buf: &mut Assembler,
    base: Register,
    index: Register,
    scale: u8,
    dest: Register,
) {
    emit_mov_ar(buf, 0, 0x8b, base, index, scale, dest);
}

pub fn emit_movq_ra(
    buf: &mut Assembler,
    src: Register,
    base: Register,
    index: Register,
    scale: u8,
) {
    emit_mov_ar(buf, 1, 0x89, base, index, scale, src);
}

pub fn emit_movl_ra(
    buf: &mut Assembler,
    src: Register,
    base: Register,
    index: Register,
    scale: u8,
) {
    emit_mov_ar(buf, 0, 0x89, base, index, scale, src);
}

fn emit_mov_ar(
    buf: &mut Assembler,
    x64: u8,
    opcode: u8,
    base: Register,
    index: Register,
    scale: u8,
    dest: Register,
) {
    assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

    if x64 != 0 || dest.msb() != 0 || index.msb() != 0 || base.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), index.msb(), base.msb());
    }

    let scale = match scale {
        8 => 3,
        4 => 2,
        2 => 1,
        _ => 0,
    };

    emit_op(buf, opcode);
    emit_modrm(buf, 0b00, dest.and7(), 0b100);
    emit_sib(buf, scale, index.and7(), base.and7());
}

fn emit_mov_reg_memq(
    buf: &mut Assembler,
    opcode: u8,
    x64: u8,
    src: Register,
    dest: Register,
    disp: i32,
) {
    let dest_msb = if dest == RIP { 0 } else { dest.msb() };

    if dest_msb != 0 || src.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, src.msb(), 0, dest_msb);
    }

    emit_op(buf, opcode);
    emit_membase(buf, dest, disp, src);
}

fn emit_membase(buf: &mut Assembler, base: Register, disp: i32, dest: Register) {
    if base == RSP || base == R12 {
        if disp == 0 {
            emit_modrm(buf, 0, dest.and7(), RSP.and7());
            emit_sib(buf, 0, RSP.and7(), RSP.and7());
        } else if fits_i8(disp) {
            emit_modrm(buf, 1, dest.and7(), RSP.and7());
            emit_sib(buf, 0, RSP.and7(), RSP.and7());
            emit(buf, disp as u8);
        } else {
            emit_modrm(buf, 2, dest.and7(), RSP.and7());
            emit_sib(buf, 0, RSP.and7(), RSP.and7());
            emit32(buf, disp as u32);
        }
    } else if disp == 0 && base != RBP && base != R13 && base != RIP {
        emit_modrm(buf, 0, dest.and7(), base.and7());
    } else if base == RIP {
        emit_modrm(buf, 0, dest.and7(), RBP.and7());
        emit32(buf, disp as u32);
    } else if fits_i8(disp) {
        emit_modrm(buf, 1, dest.and7(), base.and7());
        emit(buf, disp as u8);
    } else {
        emit_modrm(buf, 2, dest.and7(), base.and7());
        emit32(buf, disp as u32);
    }
}

pub fn emit_cmp_imm_reg(buf: &mut Assembler, mode: MachineMode, imm: i32, reg: Register) {
    let x64 = match mode {
        MachineMode::Int8 | MachineMode::Int32 => 0,
        MachineMode::Int64 => unimplemented!(),
        MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
        MachineMode::Ptr => 1,
    };

    emit_aluq_imm_reg(buf, x64, imm, reg, 0x3d, 0b111);
}

pub fn emit_subq_imm_reg(buf: &mut Assembler, imm: i32, reg: Register) {
    emit_aluq_imm_reg(buf, 1, imm, reg, 0x2d, 0b101);
}

pub fn emit_addq_imm_reg(buf: &mut Assembler, imm: i32, reg: Register) {
    emit_aluq_imm_reg(buf, 1, imm, reg, 0x05, 0);
}

pub fn emit_andq_imm_reg(buf: &mut Assembler, imm: i32, reg: Register) {
    emit_aluq_imm_reg(buf, 1, imm, reg, 0x25, 4);
}

fn emit_aluq_imm_reg(
    buf: &mut Assembler,
    x64: u8,
    imm: i32,
    reg: Register,
    rax_opcode: u8,
    modrm_reg: u8,
) {
    assert!(x64 == 0 || x64 == 1);

    if x64 != 0 || reg.msb() != 0 {
        emit_rex(buf, x64, 0, 0, reg.msb());
    }

    if fits_i8(imm) {
        emit_op(buf, 0x83);
        emit_modrm(buf, 0b11, modrm_reg, reg.and7());
        emit(buf, imm as u8);
    } else if reg == RAX {
        emit_op(buf, rax_opcode);
        emit32(buf, imm as u32);
    } else {
        emit_op(buf, 0x81);
        emit_modrm(buf, 0b11, modrm_reg, reg.and7());
        emit32(buf, imm as u32);
    }
}

pub fn emit_mov_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    if x64 != 0 || src.msb() != 0 || dest.msb() != 0 {
        emit_rex(buf, x64, src.msb(), 0, dest.msb());
    }

    emit_op(buf, 0x89);
    emit_modrm(buf, 0b11, src.and7(), dest.and7());
}

pub fn emit_neg_reg(buf: &mut Assembler, x64: u8, reg: Register) {
    emit_alul_reg(buf, 0xf7, 0b11, x64, reg);
}

pub fn emit_not_reg(buf: &mut Assembler, x64: u8, reg: Register) {
    emit_alul_reg(buf, 0xf7, 0b10, x64, reg);
}

pub fn emit_not_reg_byte(buf: &mut Assembler, reg: Register) {
    emit_alul_reg(buf, 0xf6, 0b10, 0, reg);
}

fn emit_alul_reg(buf: &mut Assembler, opcode: u8, modrm_reg: u8, x64: u8, reg: Register) {
    if reg.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, 0, 0, reg.msb());
    }

    emit_op(buf, opcode);
    emit_modrm(buf, 0b11, modrm_reg, reg.and7());
}

pub fn emit_xorb_imm_reg(buf: &mut Assembler, imm: u8, dest: Register) {
    emit_alub_imm_reg(buf, 0x80, 0x34, 0b110, imm, dest);
}

pub fn emit_andb_imm_reg(buf: &mut Assembler, imm: u8, dest: Register) {
    emit_alub_imm_reg(buf, 0x80, 0x24, 0b100, imm, dest);
}

fn emit_alub_imm_reg(
    buf: &mut Assembler,
    opcode: u8,
    rax_opcode: u8,
    modrm_reg: u8,
    imm: u8,
    dest: Register,
) {
    if dest == RAX {
        emit_op(buf, rax_opcode);
        emit(buf, imm);
    } else {
        if dest.msb() != 0 || !dest.is_basic_reg() {
            emit_rex(buf, 0, 0, 0, dest.msb());
        }

        emit_op(buf, opcode);
        emit_modrm(buf, 0b11, modrm_reg, dest.and7());
        emit(buf, imm);
    }
}

pub fn emit_sub_imm_mem(buf: &mut Assembler, mode: MachineMode, base: Register, imm: u8) {
    let (x64, opcode) = match mode {
        MachineMode::Ptr => (1, 0x83),
        MachineMode::Int32 => (0, 0x83),
        MachineMode::Int64 => unimplemented!(),
        MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
        MachineMode::Int8 => (0, 0x80),
    };

    if x64 != 0 || base.msb() != 0 {
        emit_rex(buf, x64, 0, 0, base.msb());
    }

    emit_op(buf, opcode);
    emit_modrm(buf, 0b00, 0b101, base.and7());
    emit(buf, imm);
}

pub fn emit_pushq_reg(buf: &mut Assembler, reg: Register) {
    if reg.msb() != 0 {
        emit_rex(buf, 0, 0, 0, 1);
    }

    emit_op(buf, 0x50 + reg.and7());
}

pub fn emit_popq_reg(buf: &mut Assembler, reg: Register) {
    if reg.msb() != 0 {
        emit_rex(buf, 0, 0, 0, 1);
    }

    emit_op(buf, 0x58 + reg.and7());
}

pub fn emit_retq(buf: &mut Assembler) {
    emit_op(buf, 0xC3);
}

pub fn emit_nop(buf: &mut Assembler) {
    emit_op(buf, 0x90);
}

pub fn emit64(buf: &mut Assembler, val: u64) {
    buf.emit64(val)
}

pub fn emit32(buf: &mut Assembler, val: u32) {
    buf.emit32(val)
}

pub fn emit(buf: &mut Assembler, val: u8) {
    buf.emit(val)
}

pub fn emit_op(buf: &mut Assembler, opcode: u8) {
    buf.emit(opcode);
}

pub fn emit_rex(buf: &mut Assembler, w: u8, r: u8, x: u8, b: u8) {
    assert!(w == 0 || w == 1);
    assert!(r == 0 || r == 1);
    assert!(x == 0 || x == 1);
    assert!(b == 0 || b == 1);

    buf.emit(0x4 << 4 | w << 3 | r << 2 | x << 1 | b);
}

pub fn emit_modrm(buf: &mut Assembler, mode: u8, reg: u8, rm: u8) {
    assert!(mode < 4);
    assert!(reg < 8);
    assert!(rm < 8);

    buf.emit(mode << 6 | reg << 3 | rm);
}

pub fn emit_sib(buf: &mut Assembler, scale: u8, index: u8, base: u8) {
    assert!(scale < 4);
    assert!(index < 8);
    assert!(base < 8);

    buf.emit(scale << 6 | index << 3 | base);
}

pub fn fits_i8(imm: i32) -> bool {
    imm == (imm as i8) as i32
}

pub fn emit_jcc(buf: &mut Assembler, cond: CondCode, lbl: Label) {
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

    emit_op(buf, 0x0f);
    emit_op(buf, opcode);
    buf.emit_label(lbl);
}

pub fn emit_movsx(buf: &mut Assembler, src: Register, dest: Register) {
    emit_rex(buf, 1, dest.msb(), 0, src.msb());

    emit_op(buf, 0x63);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn emit_jmp(buf: &mut Assembler, lbl: Label) {
    emit_op(buf, 0xe9);
    buf.emit_label(lbl);
}

pub fn emit_jmp_reg(buf: &mut Assembler, reg: Register) {
    if reg.msb() != 0 {
        emit_rex(buf, 0, 0, 0, reg.msb());
    }
    emit_op(buf, 0xFF);
    emit_modrm(buf, 0b11, 0b100, reg.and7());
}

pub fn emit_testl_reg_reg(buf: &mut Assembler, op1: Register, op2: Register) {
    if op1.msb() != 0 || op2.msb() != 0 {
        emit_rex(buf, 0, op1.msb(), 0, op2.msb());
    }

    emit_op(buf, 0x85);
    emit_modrm(buf, 0b11, op1.and7(), op2.and7());
}

pub fn testl_reg_mem(buf: &mut Assembler, dest: Register, src: Mem) {
    emit_rex_mem(buf, 0, dest, &src);
    emit_op(buf, 0x85);
    emit_mem(buf, dest, &src);
}

pub fn lea(buf: &mut Assembler, dest: Register, src: Mem) {
    emit_rex_mem(buf, 1, dest, &src);
    emit_op(buf, 0x8D);
    emit_mem(buf, dest, &src);
}

pub fn emit_rex_mem(buf: &mut Assembler, x64: u8, dest: Register, src: &Mem) {
    assert!(x64 == 0 || x64 == 1);

    let (base_msb, index_msb) = match src {
        &Mem::Local(_) => (RBP.msb(), 0),
        &Mem::Base(base, _) => {
            let base_msb = if base == RIP { 0 } else { base.msb() };

            (base_msb, 0)
        }

        &Mem::Index(base, index, _, _) => (base.msb(), index.msb()),
        &Mem::Offset(index, _, _) => (0, index.msb()),
    };

    if dest.msb() != 0 || index_msb != 0 || base_msb != 0 || x64 != 0 {
        emit_rex(buf, x64, dest.msb(), index_msb, base_msb);
    }
}

pub fn emit_mem(buf: &mut Assembler, dest: Register, src: &Mem) {
    match src {
        &Mem::Local(offset) => {
            emit_membase(buf, RBP, offset, dest);
        }

        &Mem::Base(base, disp) => {
            emit_membase(buf, base, disp, dest);
        }

        &Mem::Index(base, index, scale, disp) => {
            emit_membase_with_index_and_scale(buf, base, index, scale, disp, dest);
        }

        &Mem::Offset(index, scale, disp) => {
            emit_membase_without_base(buf, index, scale, disp, dest);
        }
    }
}

pub fn emit_testq_reg_reg(buf: &mut Assembler, op1: Register, op2: Register) {
    emit_rex(buf, 1, op1.msb(), 0, op2.msb());

    emit_op(buf, 0x85);
    emit_modrm(buf, 0b11, op1.and7(), op2.and7());
}

pub fn emit_add_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, src.msb(), 0, dest.msb());
    }

    emit_op(buf, 0x01);
    emit_modrm(buf, 0b11, src.and7(), dest.and7());
}

pub fn emit_sub_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, src.msb(), 0, dest.msb());
    }

    emit_op(buf, 0x29);
    emit_modrm(buf, 0b11, src.and7(), dest.and7());
}

pub fn emit_imul_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0xaf);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn emit_idiv_reg_reg(buf: &mut Assembler, x64: u8, reg: Register) {
    if reg.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, 0, 0, reg.msb());
    }

    emit_op(buf, 0xf7);
    emit_modrm(buf, 0b11, 0b111, reg.and7());
}

pub fn emit_cmp_reg_reg(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    emit_alu_reg_reg(buf, x64, 0x39, src, dest);
}

pub fn emit_cmp_mem_reg(
    buf: &mut Assembler,
    mode: MachineMode,
    base: Register,
    disp: i32,
    dest: Register,
) {
    let base_msb = if base == RIP { 0 } else { base.msb() };

    let (x64, opcode) = match mode {
        MachineMode::Int8 => (0, 0x38),
        MachineMode::Int32 => (0, 0x39),
        MachineMode::Int64 => unimplemented!(),
        MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
        MachineMode::Ptr => (1, 0x39),
    };

    if x64 != 0 || dest.msb() != 0 || base_msb != 0 {
        emit_rex(buf, x64, dest.msb(), 0, base_msb);
    }

    emit_op(buf, opcode);
    emit_membase(buf, base, disp, dest);
}

pub fn emit_mov_memindex_reg(
    buf: &mut Assembler,
    mode: MachineMode,
    base: Register,
    index: Register,
    scale: i32,
    disp: i32,
    dest: Register,
) {
    assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);
    assert!(mode.size() as i32 == scale);

    let (x64, opcode) = match mode {
        MachineMode::Int8 => (0, 0x8a),
        MachineMode::Int32 => (0, 0x8b),
        MachineMode::Int64 | MachineMode::Ptr => (1, 0x8b),
        MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
    };

    if x64 != 0 || dest.msb() != 0 || index.msb() != 0 || base.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), index.msb(), base.msb());
    }

    emit_op(buf, opcode);
    emit_membase_with_index_and_scale(buf, base, index, scale, disp, dest);
}

pub fn emit_movzx_memindex_byte_reg(
    buf: &mut Assembler,
    x64: u8,
    base: Register,
    index: Register,
    disp: i32,
    dest: Register,
) {
    if x64 != 0 || dest.msb() != 0 || index.msb() != 0 || base.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), index.msb(), base.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0xb6);
    emit_membase_with_index_and_scale(buf, base, index, 1, disp, dest);
}

pub fn emit_mov_reg_memindex(
    buf: &mut Assembler,
    mode: MachineMode,
    src: Register,
    base: Register,
    index: Register,
    scale: i32,
    disp: i32,
) {
    assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

    let (x64, opcode) = match mode {
        MachineMode::Int8 => (0, 0x88),
        MachineMode::Int32 => (0, 0x89),
        MachineMode::Int64 | MachineMode::Ptr => (1, 0x89),
        MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
    };

    if x64 != 0 || src.msb() != 0 || index.msb() != 0 || base.msb() != 0 {
        emit_rex(buf, x64, src.msb(), index.msb(), base.msb());
    }

    emit_op(buf, opcode);
    emit_membase_with_index_and_scale(buf, base, index, scale, disp, src);
}

pub fn emit_cmp_mem_imm(
    buf: &mut Assembler,
    mode: MachineMode,
    base: Register,
    disp: i32,
    imm: i32,
) {
    let base_msb = if base == RIP { 0 } else { base.msb() };

    let opcode = if fits_i8(imm) { 0x83 } else { 0x81 };

    let (x64, opcode) = match mode {
        MachineMode::Int8 => (0, 0x80),
        MachineMode::Int32 => (0, opcode),
        MachineMode::Int64 => unimplemented!(),
        MachineMode::Ptr => (1, opcode),
        MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
    };

    if x64 != 0 || base_msb != 0 {
        emit_rex(buf, x64, 0, 0, base_msb);
    }

    emit_op(buf, opcode);
    emit_membase(buf, base, disp, RDI);

    if fits_i8(imm) {
        emit(buf, imm as u8);
    } else {
        if mode == MachineMode::Int8 {
            panic!("Int8 does not support 32 bit values");
        }

        emit32(buf, imm as u32);
    }
}

pub fn emit_cmp_memindex_reg(
    buf: &mut Assembler,
    mode: MachineMode,
    base: Register,
    index: Register,
    scale: i32,
    disp: i32,
    dest: Register,
) {
    assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

    let (x64, opcode) = match mode {
        MachineMode::Int8 => (0, 0x38),
        MachineMode::Int32 => (0, 0x39),
        MachineMode::Int64 => unimplemented!(),
        MachineMode::Ptr => (1, 0x39),
        MachineMode::Float32 | MachineMode::Float64 => unreachable!(),
    };

    if x64 != 0 || dest.msb() != 0 || index.msb() != 0 || base.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), index.msb(), base.msb());
    }

    emit_op(buf, opcode);
    emit_membase_with_index_and_scale(buf, base, index, scale, disp, dest);
}

fn emit_membase_without_base(
    buf: &mut Assembler,
    index: Register,
    scale: i32,
    disp: i32,
    dest: Register,
) {
    assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

    let scale = match scale {
        8 => 3,
        4 => 2,
        2 => 1,
        _ => 0,
    };

    emit_modrm(buf, 0, dest.and7(), 4);
    emit_sib(buf, scale, index.and7(), 5);
    emit32(buf, disp as u32);
}

fn emit_membase_with_index_and_scale(
    buf: &mut Assembler,
    base: Register,
    index: Register,
    scale: i32,
    disp: i32,
    dest: Register,
) {
    assert!(scale == 8 || scale == 4 || scale == 2 || scale == 1);

    let scale = match scale {
        8 => 3,
        4 => 2,
        2 => 1,
        _ => 0,
    };

    if disp == 0 {
        emit_modrm(buf, 0, dest.and7(), 4);
        emit_sib(buf, scale, index.and7(), base.and7());
    } else if fits_i8(disp) {
        emit_modrm(buf, 1, dest.and7(), 4);
        emit_sib(buf, scale, index.and7(), base.and7());
        emit(buf, disp as u8);
    } else {
        emit_modrm(buf, 2, dest.and7(), 4);
        emit_sib(buf, scale, index.and7(), base.and7());
        emit32(buf, disp as u32);
    }
}

pub fn emit_cdq(buf: &mut Assembler) {
    emit_op(buf, 0x99);
}

pub fn emit_cqo(buf: &mut Assembler) {
    emit_rex(buf, 1, 0, 0, 0);
    emit_op(buf, 0x99);
}

pub fn emit_setb_reg(buf: &mut Assembler, op: CondCode, reg: Register) {
    if reg.msb() != 0 || !reg.is_basic_reg() {
        emit_rex(buf, 0, 0, 0, reg.msb());
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

    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_modrm(buf, 0b11, 0, reg.and7());
}

pub fn emit_setb_reg_parity(buf: &mut Assembler, reg: Register, parity: bool) {
    if reg.msb() != 0 || !reg.is_basic_reg() {
        emit_rex(buf, 0, 0, 0, reg.msb());
    }

    let opcode = if parity { 0x9a } else { 0x9b };

    emit_op(buf, 0x0f);
    emit_op(buf, opcode);
    emit_modrm(buf, 0b11, 0, reg.and7());
}

pub fn emit_movb_reg_reg(buf: &mut Assembler, src: Register, dest: Register) {
    if src.msb() != 0 || dest.msb() != 0 || !src.is_basic_reg() {
        emit_rex(buf, 0, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x88);
    emit_modrm(buf, 0b11, src.and7(), dest.and7());
}

pub fn emit_movzbl_reg_reg(buf: &mut Assembler, src: Register, dest: Register) {
    if src.msb() != 0 || dest.msb() != 0 || !src.is_basic_reg() {
        emit_rex(buf, 0, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0xb6);

    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn emit_cmpb_imm_reg(buf: &mut Assembler, imm: u8, dest: Register) {
    if dest == RAX {
        emit_op(buf, 0x3c);
        emit(buf, imm);
        return;
    }

    if dest.msb() != 0 || !dest.is_basic_reg() {
        emit_rex(buf, 0, 0, 0, dest.msb());
    }

    emit_op(buf, 0x80);
    emit_modrm(buf, 0b11, 0b111, dest.and7());
    emit(buf, imm);
}

pub fn cmov(buf: &mut Assembler, x64: u8, dest: Register, src: Register, cond: CondCode) {
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
        emit_rex(buf, x64, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, opcode);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn emit_callq_reg(buf: &mut Assembler, dest: Register) {
    if dest.msb() != 0 {
        emit_rex(buf, 0, 0, 0, dest.msb());
    }

    emit_op(buf, 0xff);
    emit_modrm(buf, 0b11, 0b10, dest.and7());
}

pub fn emit_shlq_reg(buf: &mut Assembler, imm: u8, dest: Register) {
    emit_rex(buf, 1, 0, 0, dest.msb());
    emit_op(buf, 0xC1);
    emit_modrm(buf, 0b11, 0b100, dest.and7());
    emit(buf, imm);
}

pub fn emit_shll_reg(buf: &mut Assembler, imm: u8, dest: Register) {
    if dest.msb() != 0 {
        emit_rex(buf, 0, 0, 0, dest.msb());
    }

    emit_op(buf, 0xC1);
    emit_modrm(buf, 0b11, 0b100, dest.and7());
    emit(buf, imm);
}

pub fn emit_shl_reg_cl(buf: &mut Assembler, x64: u8, dest: Register) {
    if dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, 0, 0, dest.msb());
    }

    emit_op(buf, 0xD3);
    emit_modrm(buf, 0b11, 0b100, dest.and7());
}

pub fn emit_shr_reg_cl(buf: &mut Assembler, x64: u8, dest: Register) {
    if dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, 0, 0, dest.msb());
    }

    emit_op(buf, 0xD3);
    emit_modrm(buf, 0b11, 0b101, dest.and7());
}

pub fn emit_shr_reg_imm(buf: &mut Assembler, x64: u8, dest: Register, imm: u8) {
    if dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, 0, 0, dest.msb());
    }

    emit_op(buf, if imm == 1 { 0xD1 } else { 0xC1 });
    emit_modrm(buf, 0b11, 0b101, dest.and7());

    if imm != 1 {
        emit(buf, imm);
    }
}

pub fn emit_sar_reg_cl(buf: &mut Assembler, x64: u8, dest: Register) {
    if dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, 0, 0, dest.msb());
    }

    emit_op(buf, 0xD3);
    emit_modrm(buf, 0b11, 0b111, dest.and7());
}

pub fn emit_movzx_byte(buf: &mut Assembler, x64: u8, src: Register, dest: Register) {
    if src.msb() != 0 || dest.msb() != 0 || x64 != 0 {
        emit_rex(buf, x64, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0xb6);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn addss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, false, 0x58, dest, src);
}

pub fn addsd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, true, 0x58, dest, src);
}

pub fn subss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, false, 0x5c, dest, src);
}

pub fn subsd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, true, 0x5c, dest, src);
}

pub fn mulss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, false, 0x59, dest, src);
}

pub fn mulsd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, true, 0x59, dest, src);
}

pub fn divss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, false, 0x5e, dest, src);
}

pub fn divsd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, true, 0x5e, dest, src);
}

pub fn sqrtss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, false, 0x51, dest, src);
}

pub fn sqrtsd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, true, 0x51, dest, src);
}

pub fn movaps(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x28, dest, src);
}

pub fn movups(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x10, dest, src);
}

pub fn movups_load(buf: &mut Assembler, dest: XMMRegister, src: Mem) {
    sse_packed_freg_mem(buf, 0x10, dest, src)
}

pub fn movups_store(buf: &mut Assembler, dest: Mem, src: XMMRegister) {
    sse_packed_freg_mem(buf, 0x11, src, dest);
}

pub fn movaps_load(buf: &mut Assembler, dest: XMMRegister, src: Mem) {
    sse_packed_freg_mem(buf, 0x28, dest, src)
}

pub fn movaps_store(buf: &mut Assembler, dest: Mem, src: XMMRegister) {
    sse_packed_freg_mem(buf, 0x29, src, dest);
}

pub fn addps(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x58, dest, src);
}

pub fn subps(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x5c, dest, src);
}

pub fn mulps(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x59, dest, src);
}

pub fn divps(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x5e, dest, src);
}

pub fn sqrtps(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x51, dest, src);
}

pub fn movlps(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_packed_freg_freg(buf, 0x45, dest, src);
}

pub fn movss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, false, 0x10, dest, src);
}

pub fn movss_load(buf: &mut Assembler, dest: XMMRegister, mem: Mem) {
    sse_float_freg_mem(buf, false, 0x10, dest, mem);
}

pub fn movss_store(buf: &mut Assembler, mem: Mem, src: XMMRegister) {
    sse_float_freg_mem(buf, false, 0x11, src, mem);
}

pub fn movsd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, true, 0x10, dest, src);
}

pub fn movsd_load(buf: &mut Assembler, dest: XMMRegister, mem: Mem) {
    sse_float_freg_mem(buf, true, 0x10, dest, mem);
}

pub fn movsd_store(buf: &mut Assembler, mem: Mem, src: XMMRegister) {
    sse_float_freg_mem(buf, true, 0x11, src, mem);
}

pub fn cvtsd2ss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, true, 0x5a, dest, src);
}

pub fn cvtss2sd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_float_freg_freg(buf, false, 0x5a, dest, src);
}

pub fn cvtsi2ss(buf: &mut Assembler, dest: XMMRegister, x64: u8, src: Register) {
    sse_float_freg_reg(buf, false, 0x2a, dest, x64, src);
}

pub fn cvtsi2sd(buf: &mut Assembler, dest: XMMRegister, x64: u8, src: Register) {
    sse_float_freg_reg(buf, true, 0x2a, dest, x64, src);
}

pub fn cvttss2si(buf: &mut Assembler, x64: u8, dest: Register, src: XMMRegister) {
    sse_float_reg_freg(buf, false, 0x2c, x64, dest, src);
}

pub fn movd_reg_freg(buf: &mut Assembler, dest: Register, src: XMMRegister) {
    sse_reg_freg(buf, 0x66, 0, dest, src);
}

pub fn movq_reg_freg(buf: &mut Assembler, dest: Register, src: XMMRegister) {
    sse_reg_freg(buf, 0x66, 1, dest, src);
}

pub fn movd_freg_reg(buf: &mut Assembler, dest: XMMRegister, src: Register) {
    sse_freg_reg(buf, 0x66, 0, dest, src);
}

pub fn movq_freg_reg(buf: &mut Assembler, dest: XMMRegister, src: Register) {
    sse_freg_reg(buf, 0x66, 1, dest, src);
}

fn sse_reg_freg(buf: &mut Assembler, op: u8, x64: u8, dest: Register, src: XMMRegister) {
    emit_op(buf, op);
    if x64 != 0 || dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0x7e);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

fn sse_freg_reg(buf: &mut Assembler, op: u8, x64: u8, dest: XMMRegister, src: Register) {
    emit_op(buf, op);
    if x64 != 0 || dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0x6e);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn cvttsd2si(buf: &mut Assembler, x64: u8, dest: Register, src: XMMRegister) {
    sse_float_reg_freg(buf, true, 0x2c, x64, dest, src);
}

pub fn xorps(buf: &mut Assembler, dest: XMMRegister, src: Mem) {
    sse_float_freg_mem_66(buf, false, 0x57, dest, src);
}

pub fn xorpd(buf: &mut Assembler, dest: XMMRegister, src: Mem) {
    sse_float_freg_mem_66(buf, true, 0x57, dest, src);
}

pub fn sse_packed_freg_freg(buf: &mut Assembler, op: u8, dest: XMMRegister, src: XMMRegister) {
    if dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, 0, dest.msb(), 0, src.msb());
    }
    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn sse_float_freg_freg(
    buf: &mut Assembler,
    dbl: bool,
    op: u8,
    dest: XMMRegister,
    src: XMMRegister,
) {
    let prefix = if dbl { 0xf2 } else { 0xf3 };

    emit_op(buf, prefix);

    if dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, 0, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn sse_float_freg_mem(buf: &mut Assembler, dbl: bool, op: u8, dest: XMMRegister, src: Mem) {
    let prefix = if dbl { 0xf2 } else { 0xf3 };

    emit_op(buf, prefix);
    emit_rex_mem(buf, 0, unsafe { ::core::mem::transmute(dest) }, &src);
    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_mem(buf, unsafe { ::core::mem::transmute(dest) }, &src);
}

pub fn sse_packed_freg_mem(buf: &mut Assembler, op: u8, dest: XMMRegister, src: Mem) {
    emit_rex_mem(buf, 0, unsafe { ::core::mem::transmute(dest) }, &src);
    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_mem(buf, unsafe { ::core::mem::transmute(dest) }, &src);
}

pub fn sse_float_freg_mem_66(buf: &mut Assembler, dbl: bool, op: u8, dest: XMMRegister, src: Mem) {
    if dbl {
        emit_op(buf, 0x66);
    }

    emit_rex_mem(buf, 0, unsafe { ::core::mem::transmute(dest) }, &src);
    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_mem(
        buf,
        /*Register(dest.0)*/ unsafe { ::core::mem::transmute(dest) },
        &src,
    );
}

pub fn sse_float_freg_reg(
    buf: &mut Assembler,
    dbl: bool,
    op: u8,
    dest: XMMRegister,
    x64: u8,
    src: Register,
) {
    let prefix = if dbl { 0xf2 } else { 0xf3 };

    emit_op(buf, prefix);

    if x64 != 0 || dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

pub fn sse_float_reg_freg(
    buf: &mut Assembler,
    dbl: bool,
    op: u8,
    x64: u8,
    dest: Register,
    src: XMMRegister,
) {
    let prefix = if dbl { 0xf2 } else { 0xf3 };

    emit_op(buf, prefix);

    if x64 != 0 || dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, x64, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, op);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

/*pub fn pxor(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    emit_op(buf, 0x66);

    if dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, 0, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0xef);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}*/

pub fn ucomiss(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_cmp(buf, false, dest, src);
}

pub fn ucomisd(buf: &mut Assembler, dest: XMMRegister, src: XMMRegister) {
    sse_cmp(buf, true, dest, src);
}

fn sse_cmp(buf: &mut Assembler, dbl: bool, dest: XMMRegister, src: XMMRegister) {
    if dbl {
        emit_op(buf, 0x66);
    }

    if dest.msb() != 0 || src.msb() != 0 {
        emit_rex(buf, 0, dest.msb(), 0, src.msb());
    }

    emit_op(buf, 0x0f);
    emit_op(buf, 0x2e);
    emit_modrm(buf, 0b11, dest.and7(), src.and7());
}

fn sse_optional_rex_32_ff(buf: &mut Assembler, reg: XMMRegister, base: XMMRegister) {
    let rex_bits = (reg as u8 & 0x8) >> 1 | (base as u8 & 0x8) >> 3;
    if rex_bits != 0 {
        buf.emit(0x40 | rex_bits);
    }
}

fn sse_optional_rex32_fm(buf: &mut Assembler, reg: XMMRegister, base: Mem) {
    use super::avx::*;
    let rex_bits = emit_rex_memv(buf, 0, unsafe { std::mem::transmute(reg) }, &base);
    if rex_bits != 0 {
        buf.emit(0x40 | rex_bits);
    }
}

fn ssse3_or_4_instr(
    buf: &mut Assembler,
    dst: XMMRegister,
    src: XMMRegister,
    prefix: u8,
    escape1: u8,
    escape2: u8,
    opcode: u8,
) {
    buf.emit(prefix);
    sse_optional_rex_32_ff(buf, dst, src);
    buf.emit(escape1);
    buf.emit(escape2);
    buf.emit(opcode);
    emit_sse_ff(buf, dst, src)
}

fn ssse3_or_4_instr_mem(
    buf: &mut Assembler,
    dst: XMMRegister,
    src: Mem,
    prefix: u8,
    escape1: u8,
    escape2: u8,
    opcode: u8,
) {
    buf.emit(prefix);
    sse_optional_rex32_fm(buf, dst, src);
    buf.emit(escape1);
    buf.emit(escape2);
    buf.emit(opcode);
    emit_sse_mem_f(buf, dst, src)
}

fn sse2_instr(
    buf: &mut Assembler,
    dst: XMMRegister,
    src: XMMRegister,
    prefix: u8,
    escape: u8,
    opcode: u8,
) {
    use super::avx::emit_sse_ff;
    buf.emit(prefix);
    sse_optional_rex_32_ff(buf, dst, src);
    buf.emit(escape);
    buf.emit(opcode);
    emit_sse_ff(buf, dst, src);
}

fn sse2_instr_mem(
    buf: &mut Assembler,
    dst: XMMRegister,
    src: Mem,
    prefix: u8,
    escape: u8,
    opcode: u8,
) {
    buf.emit(prefix);
    sse_optional_rex32_fm(buf, dst, src);
    buf.emit(escape);
    buf.emit(opcode);
    super::avx::emit_sse_mem_f(buf, dst, src);
}

pub fn cvtps2dq(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 91);
}

pub fn punpcklbw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 96);
}

pub fn punpcklwd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 97);
}

pub fn punpckldq(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 98);
}

pub fn packsswb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 99);
}

pub fn packuswb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 103);
}

pub fn punpckhbw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 104);
}

pub fn punpckhwd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 105);
}

pub fn punpckhdq(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 106);
}

pub fn packssdw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 107);
}

pub fn punpcklqdq(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 108);
}

pub fn punpckhqdq(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 109);
}

pub fn paddb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 252);
}

pub fn paddw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 253);
}

pub fn paddd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 254);
}

pub fn paddsb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 236);
}

pub fn paddsw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 237);
}

pub fn paddusb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 220);
}

pub fn paddusw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 221);
}

pub fn pcmpeqb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 116);
}

pub fn pcmpeqw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 117);
}

pub fn pcmpeqd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 118);
}

pub fn pcmpgtb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 100);
}

pub fn pcmpgtw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 101);
}

pub fn pcmpgtd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 102);
}

pub fn pmaxsw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 238);
}

pub fn pmaxub(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 222);
}

pub fn pminsw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 234);
}

pub fn pminub(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 218);
}

pub fn pmullw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 213);
}

pub fn pmuludq(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 244);
}

pub fn psllw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 241);
}

pub fn pslld(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 242);
}

pub fn psraw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 225);
}

pub fn psrad(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 226);
}

pub fn psrlw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 209);
}

pub fn psrld(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 210);
}

pub fn psubb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 248);
}

pub fn psubw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 249);
}

pub fn psubd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 250);
}

pub fn psubsb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 232);
}

pub fn psubsw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 233);
}

pub fn psubusb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 216);
}

pub fn psubusw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 217);
}

pub fn pand(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 219);
}

pub fn por(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 235);
}

pub fn pxor(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    sse2_instr(buf, dst, src, 102, 15, 239);
}

pub fn cvtps2dq_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 91);
}

pub fn punpcklbw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 96);
}

pub fn punpcklwd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 97);
}

pub fn punpckldq_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 98);
}

pub fn packsswb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 99);
}

pub fn packuswb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 103);
}

pub fn punpckhbw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 104);
}

pub fn punpckhwd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 105);
}

pub fn punpckhdq_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 106);
}

pub fn packssdw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 107);
}

pub fn punpcklqdq_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 108);
}

pub fn punpckhqdq_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 109);
}

pub fn paddb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 252);
}

pub fn paddw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 253);
}

pub fn paddd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 254);
}

pub fn paddsb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 236);
}

pub fn paddsw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 237);
}

pub fn paddusb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 220);
}

pub fn paddusw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 221);
}

pub fn pcmpeqb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 116);
}

pub fn pcmpeqw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 117);
}

pub fn pcmpeqd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 118);
}

pub fn pcmpgtb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 100);
}

pub fn pcmpgtw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 101);
}

pub fn pcmpgtd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 102);
}

pub fn pmaxsw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 238);
}

pub fn pmaxub_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 222);
}

pub fn pminsw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 234);
}

pub fn pminub_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 218);
}

pub fn pmullw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 213);
}

pub fn pmuludq_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 244);
}

pub fn psllw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 241);
}

pub fn pslld_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 242);
}

pub fn psraw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 225);
}

pub fn psrad_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 226);
}

pub fn psrlw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 209);
}

pub fn psrld_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 210);
}

pub fn psubb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 248);
}

pub fn psubw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 249);
}

pub fn psubd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 250);
}

pub fn psubsb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 232);
}

pub fn psubsw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 233);
}

pub fn psubusb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 216);
}

pub fn psubusw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 217);
}

pub fn pand_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 219);
}

pub fn por_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 235);
}

pub fn pxor_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    sse2_instr_mem(buf, dst, src, 102, 15, 239);
}

use super::avx::*;

use super::avx::SIMDPrefix::*;

//sse2_instr_list!(sse2_instr_def);
pub fn vcvtps2dq_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        91,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vcvtps2dq(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        91,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpcklbw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        96,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpcklbw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        96,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpcklwd_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        97,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpcklwd(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        97,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckldq_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        98,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckldq(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        98,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpacksswb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        99,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpacksswb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        99,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpackuswb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        103,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpackuswb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        103,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhbw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        104,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhbw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        104,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhwd_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        105,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhwd(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        105,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhdq_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        106,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhdq(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        106,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpackssdw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        107,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpackssdw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        107,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpcklqdq_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        108,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpcklqdq(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        108,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhqdq_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        109,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpunpckhqdq(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        109,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        252,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        252,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        253,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        253,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddd_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        254,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddd(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        254,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddsb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        236,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddsb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        236,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddsw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        237,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddsw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        237,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddusb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        220,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddusb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        220,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddusw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        221,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpaddusw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        221,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpeqb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        116,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpeqb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        116,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpeqw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        117,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpeqw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        117,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpeqd_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        118,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpeqd(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        118,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpgtb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        100,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpgtb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        100,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpgtw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        101,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpgtw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        101,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpgtd_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        102,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpcmpgtd(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        102,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmaxsw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        238,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmaxsw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        238,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmaxub_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        222,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmaxub(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        222,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpminsw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        234,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpminsw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        234,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpminub_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        218,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpminub(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        218,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmullw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        213,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmullw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        213,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmuludq_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        244,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpmuludq(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        244,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsllw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        241,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsllw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        241,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpslld_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        242,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpslld(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        242,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsraw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        225,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsraw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        225,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsrad_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        226,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsrad(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        226,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsrlw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        209,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsrlw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        209,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsrld_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        210,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsrld(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        210,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        248,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        248,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        249,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        249,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubd_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        250,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubd(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        250,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubsb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        232,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubsb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        232,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubsw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        233,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubsw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        233,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubusb_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        216,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubusb(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        216,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubusw_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        217,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpsubusw(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        217,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpand_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        219,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpand(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        219,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpor_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        235,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpor(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        235,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpxor_mem(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: Mem) {
    vinstrm(
        buf,
        239,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn vpxor(buf: &mut Assembler, dst: XMMRegister, src1: XMMRegister, src2: XMMRegister) {
    vinstr(
        buf,
        239,
        dst,
        src1,
        src2,
        k0x66,
        super::avx::LeadingOpcode::k0F,
        super::avx::VexW::W0,
    );
}

pub fn pabsb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 28);
}

pub fn pabsb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 28);
}

pub fn pabsw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 29);
}

pub fn pabsw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 29);
}

pub fn pabsd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 30);
}

pub fn pabsd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 30);
}

pub fn phaddd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 2);
}

pub fn phaddd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 2);
}

pub fn phaddw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 1);
}

pub fn phaddw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 1);
}

pub fn pshufb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 0);
}

pub fn pshufb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 0);
}

pub fn psignb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 8);
}

pub fn psignb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 8);
}

pub fn psignw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 9);
}

pub fn psignw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 9);
}

pub fn psignd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 10);
}

pub fn psignd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 10);
}

pub fn ptest_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 23);
}

pub fn ptest(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 23);
}

pub fn pmovsxbw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 32);
}

pub fn pmovsxbw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 32);
}

pub fn pmovsxwd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 35);
}

pub fn pmovsxwd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 35);
}

pub fn packusdw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 43);
}

pub fn packusdw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 43);
}

pub fn pmovzxbw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 48);
}

pub fn pmovzxbw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 48);
}

pub fn pmovzxwd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 51);
}

pub fn pmovzxwd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 51);
}

pub fn pminsb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 56);
}

pub fn pminsb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 56);
}

pub fn pminsd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 57);
}

pub fn pminsd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 57);
}

pub fn pminuw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 58);
}

pub fn pminuw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 58);
}

pub fn pminud_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 59);
}

pub fn pminud(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 59);
}

pub fn pmaxsb_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 60);
}

pub fn pmaxsb(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 60);
}

pub fn pmaxsd_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 61);
}

pub fn pmaxsd(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 61);
}

pub fn pmaxuw_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 62);
}

pub fn pmaxuw(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 62);
}

pub fn pmaxud_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 63);
}

pub fn pmaxud(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 63);
}

pub fn pmulld_mem(buf: &mut Assembler, dst: XMMRegister, src: Mem) {
    ssse3_or_4_instr_mem(buf, dst, src, 102 as u8, 15, 56, 64);
}

pub fn pmulld(buf: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    ssse3_or_4_instr(buf, dst, src, 102 as u8, 15, 56, 64);
}
