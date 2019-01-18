pub mod asm;
pub mod dseg;
pub mod native_x64;
pub mod registers;

pub mod prelude {
    pub use super::asm::*;
    pub use super::native_x64::*;
    pub use super::registers::*;
    pub use super::*;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MachineMode {
    Int8,
    Int32,
    Int64,
    Float32,
    Float64,
    Ptr,
}

impl MachineMode {
    pub fn size(self) -> usize {
        match self {
            MachineMode::Int8 => 1,
            MachineMode::Int32 => 4,
            MachineMode::Int64 => 8,
            MachineMode::Ptr => 8,
            MachineMode::Float32 => 4,
            MachineMode::Float64 => 8,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CondCode {
    Zero,
    NonZero,
    Equal,
    NotEqual,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    UnsignedGreater,
    UnsignedGreaterEq,
    UnsignedLess,
    UnsignedLessEq,
}

const PAGE_SIZE: usize = 4096;

use core::mem;

#[cfg(target_family = "unix")]
fn setup(size: usize) -> *mut u8 {
    unsafe {
        let size = size * PAGE_SIZE;
        let content: *mut libc::c_void = mem::uninitialized();
        let result = libc::mmap(
            content,
            size,
            libc::PROT_EXEC | libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        );

        if result == libc::MAP_FAILED {
            panic!("mmap failed");
        }
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
    size: usize,
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

    pub fn ptr(&self) -> *const u8 {
        self.pointer
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

use self::asm::Assembler;

pub fn get_executable_memory(asm: &Assembler) -> Memory {
    let data = asm.data().clone();
    let dseg = &asm.dseg;
    let total_size = data.len() + dseg.size() as usize;
    let ptr = setup(total_size);

    dseg.finish(ptr);

    let start;
    unsafe {
        start = ptr.offset(dseg.size() as isize);
        ::core::ptr::copy_nonoverlapping(data.as_ptr(), start as *mut u8, data.len());
    };

    let memory = Memory {
        start,
        end: unsafe { ptr.offset(total_size as isize) },
        pointer: ptr,
        size: total_size,
    };

    memory
}
