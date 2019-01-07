use crate::ir::{Function,Fixup};
use crate::*;
use std::collections::HashMap;
use jazz_jit::get_executable_memory;
use std::mem;


#[derive(Clone,Debug,PartialEq,Eq,Copy)]
pub enum DataKind {
    Function,
    Data,
}

#[derive(Clone,Debug)]
pub struct DataContext {
    pub kind: DataKind,
    pub data: *const u8,
    pub is_sized: bool,
    pub size: usize,
    pub linkage: Linkage
}



#[derive(Clone,Debug)]
pub struct Module {
    pub data: HashMap<String,DataContext>,
    pub uncompiled_functions: HashMap<String,Function>,  
    pub uncompiled_data: HashMap<String,DataContext>,
}



impl Module {
    pub fn new() -> Module {
        Module {
            data: HashMap::new(),
            uncompiled_functions: HashMap::new(),
            uncompiled_data: HashMap::new(),
        }
    }

    fn fix_fixups(&mut self) {
        let funcs = self.uncompiled_functions.clone();
        for (_,func) in funcs.iter() {
            let fct: &Function = func;
            for fixup in fct.fixups.iter() {
                let fixup: &Fixup = fixup;
                let name = fct.name.clone();

                let (data,_) = self.get_finalized_data(&fixup.global_name.clone());
                let (curr,_) = self.get_finalized_data(&name);
                
                unsafe {
                    let offset = data.offset(0);
                    let slice: [u8;8] = mem::transmute(offset);
                    let mut pc = 0;
                    for i in fixup.pos..fixup.pos + 8 {
                        let byte = &mut *(curr.offset(i as isize) as *mut u8);
                        *byte = slice[pc].clone();
                        pc+= 1; 
                    }
                }
            }
        }
    }

    pub fn get_function<'r>(&'r mut self,fname: &String) -> &'r mut Function {
        self.uncompiled_functions.get_mut(fname).expect("function not found")
    }
}

pub trait Backend<'a> {
    type FinalizedData;
    type FinalizedFunction;
    type CompiledFunction;
    type CompiledData;
    /// Declare function in current context
    fn declare_function(&mut self,_name: String,_linkage: Linkage) {
        
    }
    /// Declare data in current context
    fn declare_data(&mut self,_name: String,_linkage: Linkage) {}
    /// Initialize data to some value
    fn define_data(&mut self,_name: String,_data: &[u8]) {}

    /// Returns the finalized function from backend
    fn get_finalized_function(&mut self,_f: Self::CompiledFunction) -> Self::FinalizedFunction {unimplemented!()}
    /// Returns the finalized data from backend
    fn get_finalized_data(&mut self,_f: Self::CompiledData) -> Self::FinalizedData {unimplemented!()}
    fn finish(&mut self) {unimplemented!()}
}

impl<'a> Backend<'a> for Module {
    type FinalizedData = (*const u8,usize);
    type CompiledFunction = &'a str;
    type CompiledData = &'a str;
    type FinalizedFunction = *const u8;

    fn declare_function(&mut self,name: String, linkage: Linkage) {
        let func = Function::new(name.clone(),linkage);
        self.uncompiled_functions.insert(name,func);
    }

    fn declare_data(&mut self,_name: String, _linkage: Linkage) {
        let ctx = DataContext {
            data: 0 as *const u8,
            kind: DataKind::Data,
            is_sized: false,
            size: 0,
            linkage: _linkage
        };
        self.data.insert(_name.clone(),ctx.clone());
        self.uncompiled_data.insert(_name,ctx);

    }
    fn define_data(&mut self,name: String,data: &[u8]) {
        let data = DataContext {
            data: data.as_ptr(),
            kind: DataKind::Data,
            is_sized: true,
            size: data.len(),
            linkage: Linkage::Local,
        };
        self.data.insert(name,data);
    }

    

    fn get_finalized_data(&mut self,f: Self::CompiledData) -> (*const u8,usize) {
        let data = self.data.get(f).expect("Data not found");

        if data.is_sized {
            return (data.data,data.size);
        } else {
            return (data.data,0);
        }
    }

    fn get_finalized_function(&mut self,f: Self::CompiledFunction) -> *const u8 {
        let data: &DataContext = self.data.get(f).expect("Data not found");
        if data.kind != DataKind::Function {
            panic!("Data is not a function");
        } else {
            return data.data;
        }
    }
    fn finish(&mut self) {
        for (name,ctx) in self.uncompiled_data.iter_mut() {
            let data: &mut DataContext = ctx;
            match &data.linkage {
                Linkage::Local => (),
                Linkage::Extern(ptr) => {
                    data.data = *ptr;
                }
                Linkage::Libc => {
                    use crate::dylib as lib;
                    unsafe {
                        let lib = lib::Library::new("libc++.so.1").expect("Libc not found");
                        let symbol: lib::Symbol<*const u8> = lib.get(name.as_bytes()).unwrap();
                        data.data = *symbol;
                    }
                }
                Linkage::Dylib(libname) => {
                    use crate::dylib as lib;

                    unsafe {
                        let lib = lib::Library::new(libname).expect("failed to load library");
                        let symbol: lib::Symbol<*const u8> = lib.get(name.as_bytes()).unwrap();
                        data.data = *symbol;
                    }
                }
            }
        }

        for (name,func) in self.uncompiled_functions.iter_mut() {
            let func: &mut Function = func;
            match &func.linkage {
                Linkage::Local => (),
                Linkage::Extern(ptr) => {
                    let data = DataContext {
                        data: *ptr,
                        size: 0,
                        is_sized: false,
                        kind: DataKind::Function,
                        linkage: func.linkage.clone()
                    };
                    self.data.insert(name.to_owned(),data);
                    continue;
                }
                Linkage::Libc => {
                    use crate::dylib as lib;
                    
                    unsafe {
                        let lib = lib::Library::new("libc++.so.1").expect("Failed to open library");
                        let func: lib::Symbol<unsafe extern fn()> = lib.get(func.name.as_bytes()).expect("function not found");
                        let data = DataContext {
                            data: func.clone().into_raw().get_ptr(),
                            size: 0,
                            is_sized: false,
                            linkage: Linkage::Extern(func.into_raw().get_ptr()),
                            kind: DataKind::Function,
                        };
                        self.data.insert(name.to_owned(),data);
                        continue;
                    }
                }
                Linkage::Dylib(libname) => {
                    use crate::dylib as lib;
                    
                    unsafe {
                        let lib = lib::Library::new(libname).expect("Failed to open library");
                        let func: lib::Symbol<unsafe extern fn()> = lib.get(func.name.as_bytes()).expect("function not found");
                        let data = DataContext {
                            data: func.clone().into_raw().get_ptr(),
                            size: 0,
                            is_sized: false,
                            linkage: Linkage::Extern(func.into_raw().get_ptr()),
                            kind: DataKind::Function,
                        };
                        self.data.insert(name.to_owned(),data);
                        continue;
                    }
                }
            }

            let asm = func.asm_mut();
            asm.fix_forward_jumps();
            let memory = get_executable_memory(&asm);      

            let data = DataContext {
                data: memory.ptr(),
                size: memory.size(),
                is_sized: true,
                kind: DataKind::Function,
                linkage: func.linkage.clone(),
            };

            self.data.insert(name.to_owned(),data);
        }

        self.fix_fixups();
    }
}