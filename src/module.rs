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

use crate::compiler::get_executable_memory;
use crate::extsymbol::find_symbol;
use crate::function::Function;
use crate::function::Reloc;
use fnv::FnvHashMap;
use std::mem;

pub struct Module {
    pub data: FnvHashMap<String, DataContext>,
    pub uncompiled_functions: FnvHashMap<String, Function>,
    pub uncompiled_data: FnvHashMap<String, DataContext>,
}

impl Module {
    pub fn new() -> Module {
        Module {
            uncompiled_data: FnvHashMap::default(),
            uncompiled_functions: FnvHashMap::default(),
            data: FnvHashMap::default(),
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
            return (data.data  as *mut _, data.size);
        } else {
            return (data.data  as *mut _, 0);
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
            let ptr = unsafe { libc::malloc(total_size) };
            let ptr = ptr as *const u8;
            dseg.finish(ptr);
            unsafe {
                libc::mprotect(ptr as *mut _, total_size, libc::PROT_WRITE | libc::PROT_READ | libc::PROT_EXEC);
            };
            let start;
            use std::ptr;
            unsafe {
                start = ptr.offset(dseg.size() as isize);
                ptr::copy_nonoverlapping(data.as_ptr(), start as *mut u8, data.len());
            }

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
