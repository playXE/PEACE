use libc::{dlsym, RTLD_DEFAULT};

use std::ffi::CString;

#[cfg(not(windows))]
fn find_symbol(name: &str) -> *const u8 {
    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();
    let sym = unsafe { dlsym(RTLD_DEFAULT, c_str_ptr) };

    if sym.is_null() {
        panic!("can't resolve symbol {}", name);
    }

    sym as *const u8
}

pub fn flush_icache(_: *const u8, _: usize) {
    use std::sync::atomic::{compiler_fence, Ordering};

    // no flushing needed on x86_64, but emit compiler barrier
    compiler_fence(Ordering::SeqCst);
}

#[cfg(windows)]
fn find_symbol(name: &str) -> *const u8 {
    const MSVCRT_DLL: &[u8] = b"msvcrt.dll\0";

    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();

    unsafe {
        let handles = [
            // try to find the searched symbol in the currently running executable
            ptr::null_mut(),
            // try to find the searched symbol in local c runtime
            winapi::um::libloaderapi::GetModuleHandleA(MSVCRT_DLL.as_ptr() as *const i8),
        ];

        for handle in &handles {
            let addr = winapi::um::libloaderapi::GetProcAddress(*handle, c_str_ptr);
            if addr.is_null() {
                continue;
            }
            return addr as *const u8;
        }

        let msg = if handles[1].is_null() {
            "(msvcrt not loaded)"
        } else {
            ""
        };
        panic!("cannot resolve address of symbol {} {}", name, msg);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum Linkage {
    Import,
    Local,
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum DataKind {
    Function,
    Data,
}

#[derive(Clone, Debug)]
pub struct DataContext {
    pub kind: DataKind,
    pub data: *const u8,
    pub is_sized: bool,
    pub size: usize,
    pub linkage: Linkage,
}
use crate::function::*;

use crate::backend::get_executable_memory;
use std::collections::HashMap;
use std::mem;


pub struct Module {
    pub data: HashMap<String, DataContext>,
    pub uncompiled_functions: HashMap<String, Function>,
    pub uncompiled_data: HashMap<String, DataContext>,
}

impl Module {
    pub fn new() -> Module {
        Module {
            uncompiled_data: HashMap::default(),
            uncompiled_functions: HashMap::default(),
            data: HashMap::default(),
        }
    }

    pub fn get_function<'r>(&'r mut self, fname: &str) -> &'r mut Function {
        self.uncompiled_functions
            .get_mut(fname)
            .expect("function not found")
    }

    pub fn declare_function(&mut self, name: &str, linkage: Linkage) {
        let func = Function::new(name, linkage);
        self.uncompiled_functions.insert(name.to_owned(), func);
    }

    pub fn declare_data(&mut self, _name: String, _linkage: Linkage) {
        let ctx = DataContext {
            data: 0 as *const u8,
            kind: DataKind::Data,
            is_sized: false,
            size: 0,
            linkage: _linkage,
        };
        self.data.insert(_name.clone(), ctx.clone());
        self.uncompiled_data.insert(_name, ctx);
    }

    pub fn define_data(&mut self, name: String, data: &[u8]) {
        let data = DataContext {
            data: data.as_ptr(),
            kind: DataKind::Data,
            is_sized: true,
            size: data.len(),
            linkage: Linkage::Local,
        };
        self.data.insert(name, data);
    }

    pub fn reloc_fix(&mut self) {
        let funcs = self.uncompiled_functions.clone();

        for (_, func) in funcs.iter() {
            let fct: &Function = func;

            for reloc in fct.relocs.iter() {
                let reloc: &Reloc = reloc;

                let name = &fct.name;

                let (data, _) = self.get_finalized_data(&reloc.global_name);
                let (curr, _) = self.get_finalized_data(&name).clone();

                unsafe {
                    let offset = data.offset(0);
                    let slice: [u8; 8] = mem::transmute(offset);

                    let mut pc = 0;
                    for i in reloc.at..reloc.to {
                        let byte = &mut *(curr.offset(i as isize) as *mut u8);
                        *byte = slice[pc];
                        pc += 1;
                    }
                }
            }
        }
    }
    pub fn get_finalized_data(&mut self, f: &str) -> (*mut u8, usize) {
        let data = self.data.get(f).expect("Data not found");

        if data.is_sized {
            return (data.data as *mut _, data.size);
        } else {
            return (data.data as *mut _, 0);
        }
    }

    pub fn get_finalized_function(&mut self, f: &str) -> *mut u8 {
        let data: &DataContext = self.data.get(f).expect("Data not found");
        if data.kind != DataKind::Function {
            panic!("Data is not a function");
        } else {
            return data.data as *mut _;
        }
    }

    pub fn finish(&mut self) {
        for (name, ctx) in self.uncompiled_data.iter_mut() {
            let data: &mut DataContext = ctx;

            match &data.linkage {
                Linkage::Local => continue,
                Linkage::Import => {
                    let symbol = find_symbol(name);
                    data.data = symbol;
                }
            }
        }

        for (name, func) in self.uncompiled_functions.iter_mut() {
            match &func.linkage {
                Linkage::Local => (),
                Linkage::Import => {
                    let func: extern "C" fn() = unsafe { mem::transmute(find_symbol(name)) };

                    let data = DataContext {
                        data: func as *const u8,
                        size: 0,
                        is_sized: false,
                        linkage: Linkage::Import,
                        kind: DataKind::Function,
                    };
                    self.data.insert(name.to_owned(), data);
                    continue;
                }
            }
            let asm = func.asm_mut();

            asm.fix_forward_jumps();
            let data = asm.data().clone();
            let dseg = &asm.dseg;
            let total_size = data.len() + dseg.size() as usize;
            let ptr = get_executable_memory(asm).ptr();
            flush_icache(ptr, total_size);
            let data = DataContext {
                data: ptr,
                size: total_size,
                is_sized: true,
                kind: DataKind::Function,
                linkage: func.linkage,
            };
            self.data.insert(name.to_owned(), data);
        }

        self.reloc_fix();
    }
}

fn round_up_to_page_size(size: usize, page_size: usize) -> usize {
    (size + (page_size - 1)) & !(page_size - 1)
}

/// A simple struct consisting of a pointer and length.
struct PtrLen {
    ptr: *mut u8,
    len: usize,
}

impl PtrLen {
    /// Create a new empty `PtrLen`.
    fn new() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
        }
    }

    /// Create a new `PtrLen` pointing to at least `size` bytes of memory,
    /// suitably sized and aligned for memory protection.
    #[cfg(not(target_os = "windows"))]
    fn with_size(size: usize) -> Result<Self, String> {
        let page_size = region::page::size();
        let alloc_size = round_up_to_page_size(size, page_size);
        unsafe {
            let mut ptr: *mut libc::c_void = mem::uninitialized();
            let err = libc::posix_memalign(&mut ptr, page_size, alloc_size);
            if err == 0 {
                Ok(Self {
                    ptr: ptr as *mut u8,
                    len: alloc_size,
                })
            } else {
                Err(errno::Errno(err).to_string())
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn with_size(size: usize) -> Result<Self, String> {
        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE};

        let page_size = region::page::size();

        // VirtualAlloc always rounds up to the next multiple of the page size
        let ptr = unsafe {
            VirtualAlloc(
                ptr::null_mut(),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if !ptr.is_null() {
            Ok(Self {
                ptr: ptr as *mut u8,
                len: round_up_to_page_size(size, page_size),
            })
        } else {
            Err(errno::errno().to_string())
        }
    }
}

/// JIT memory manager. This manages pages of suitably aligned and
/// accessible memory.
pub struct Memory {
    allocations: Vec<PtrLen>,
    executable: usize,
    current: PtrLen,
    position: usize,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            allocations: Vec::new(),
            executable: 0,
            current: PtrLen::new(),
            position: 0,
        }
    }

    fn finish_current(&mut self) {
        self.allocations
            .push(mem::replace(&mut self.current, PtrLen::new()));
        self.position = 0;
    }

    /// TODO: Use a proper error type.
    pub fn allocate(&mut self, size: usize) -> Result<*mut u8, String> {
        if size <= self.current.len - self.position {
            // TODO: Ensure overflow is not possible.
            let ptr = unsafe { self.current.ptr.add(self.position) };
            self.position += size;
            return Ok(ptr);
        }

        self.finish_current();

        // TODO: Allocate more at a time.
        self.current = PtrLen::with_size(size)?;
        self.position = size;
        Ok(self.current.ptr)
    }

    /// Set all memory allocated in this `Memory` up to now as readable and executable.
    pub fn set_readable_and_executable(&mut self) {
        self.finish_current();

        for &PtrLen { ptr, len } in &self.allocations[self.executable..] {
            if len != 0 {
                unsafe {
                    region::protect(ptr, len, region::Protection::ReadExecute)
                        .expect("unable to make memory readable+executable");
                }
            }
        }
    }

    /// Set all memory allocated in this `Memory` up to now as readonly.
    pub fn set_readonly(&mut self) {
        self.finish_current();

        for &PtrLen { ptr, len } in &self.allocations[self.executable..] {
            if len != 0 {
                unsafe {
                    region::protect(ptr, len, region::Protection::Read)
                        .expect("unable to make memory readonly");
                }
            }
        }
    }
}