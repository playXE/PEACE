#[derive(Debug, Clone)]
#[repr(C)]
pub struct ForwardJump {
    pub at: usize,
    pub to: usize,
}
use super::constants_x64::Register;
use super::dseg::DSeg;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
pub type Label = usize;

trait Idx {
    fn index(&self) -> usize;
}

impl Idx for usize {
    fn index(&self) -> usize {
        self.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
#[repr(C)]
pub enum Mem {
    // rbp + val1
    Local(i32),

    // reg1 + val1
    Base(Register, i32),

    // reg1 + reg2 * val1 + val2
    Index(Register, Register, i32, i32),

    // reg1 * val1 + val2
    Offset(Register, i32, i32),
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Assembler {
    pub(crate) data: Vec<u8>,
    pub dseg: DSeg,
    pub jumps: Vec<ForwardJump>,
    pub labels: Vec<Option<usize>>,
}

impl Assembler {
    pub fn emit_u32_at(&mut self, pos: i32, value: u32) {
        let buf = &mut self.data[pos as usize..];
        LittleEndian::write_u32(buf, value);
    }

    pub fn new() -> Assembler {
        Assembler {
            data: Vec::new(),
            dseg: DSeg::new(),
            jumps: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn create_label(&mut self) -> usize {
        let idx = self.labels.len();

        self.labels.push(None);
        idx
    }

    pub fn data<'r>(&'r self) -> &'r Vec<u8> {
        &self.data
    }

    pub fn bind_label(&mut self, lbl: usize) {
        let lbl_idx = lbl;

        assert!(self.labels[lbl_idx].is_none());
        self.labels[lbl_idx] = Some(self.data.len());
    }

    pub fn emit_label(&mut self, lbl: Label) {
        let value = self.labels[lbl.index()];

        match value {
            // backward jumps already know their target
            Some(idx) => {
                let current = self.data.len() + 4;
                let target = idx;

                let diff = -((current - target) as i32);
                self.emit32(diff as u32);
            }

            // forward jumps do not know their target yet
            // we need to do this later...
            None => {
                let pos = self.data.len();
                self.emit32(0);
                self.jumps.push(ForwardJump { at: pos, to: lbl });
            }
        }
    }

    pub fn fix_forward_jumps(&mut self) {
        for jmp in &self.jumps {
            let target = self.labels[jmp.to].expect("Label not defined");
            let diff = (target - jmp.at - 4) as i32;

            let mut slice = &mut self.data[jmp.at..];
            slice.write_u32::<LittleEndian>(diff as u32).unwrap();
        }
    }

    pub fn pos(&self) -> usize {
        self.data.len()
    }
    pub fn emit(&mut self, byte: u8) {
        self.data.write_u8(byte).unwrap();
    }
    pub fn emit32(&mut self, uint: u32) {
        self.data.write_u32::<LittleEndian>(uint).unwrap();
    }
    pub fn emit64(&mut self, ulong: u64) {
        self.data.write_u64::<LittleEndian>(ulong).unwrap();
    }
}
