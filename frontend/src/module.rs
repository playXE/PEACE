use crate::function::Function;
use std::collections::HashMap;
//use std::ptr;
use std::mem;
use peace_backend::sink::Memory;
use crate::abi::Linkage;
//use libloading::{Library,Symbol};


 pub trait ModuleTrait {
    fn new() -> Self;

    fn add_function(&mut self,f: Function);

    fn get_func<'r>(&'r self,fname: String) -> &'r Function;

    fn get_mut_func<'r>(&'r mut self,fname: String) -> &'r mut Function;

    //fn get_data<'r>(&'r mut self,datan: String) -> &'r mut Memory;

    fn finish(&mut self);

    fn fix_calls(&mut self);
}

impl ModuleTrait for Module {
     fn new() -> Module {
        Module {
            functions: HashMap::new(),
            data: HashMap::new(),
        }
    }

     fn add_function(&mut self,f: Function) {
        self.functions.insert(f.name.clone(),f);
    }
     fn get_mut_func<'r>(&'r mut self,fname: String) -> &'r mut Function {
        self.functions.get_mut(&fname).unwrap()
    }

    fn get_func<'r>(&'r self,fname: String) -> &'r Function {
        self.functions.get(&fname).unwrap()
    }

    /* fn get_data<'r>(&'r mut self,n: String) -> &'r Memory {
        self.data.get(&n).unwrap()
    }*/

     fn finish(&mut self) {
        for (name,function) in self.functions.iter_mut() {
            let func: &mut Function = function;
            match func.linkage() {
                Linkage::Local => {},
                Linkage::Extern(fptr) => {
                    let memory = Memory::new(fptr);
                    self.data.insert(name.to_string(), memory);
                    continue;
                }
                Linkage::DynamicImport(_fname) => {
                    unimplemented!()
                }
                _ => unimplemented!()
            }
            let memory = func.sink().finish();
            self.data.insert(name.to_string(), memory);
        }

        self.fix_calls();
    }

    fn fix_calls(&mut self) {
        let fcts = self.functions.clone();
        use crate::function::CallFixup;
        for (_,fct) in fcts.iter() {
            let fct: &Function = fct;
            for fixup in fct.fixups.iter() {
                let fixup: CallFixup = fixup.clone();
                let f: Memory = self.data.get(&fixup.name).unwrap().clone();
                let curr: &mut Memory = self.data.get_mut(&fct.name).unwrap();
                let curr_ptr = curr.ptr();
                let ptr = f.ptr();
                unsafe {
                    let off = ptr.offset(0);
                    let slice: [u8;8] = mem::transmute(off);
                    let mut pc = 0;
                    for i in fixup.pos..fixup.pos + 8 {
                        let byte = &mut *(curr_ptr.offset(i as isize) as *mut u8);
                        *byte = slice[pc].clone();
                        pc += 1;
                    }
                }
            }
        }

    }
}

pub struct Module {
    functions: HashMap<String,Function>,
    data: HashMap<String,Memory>,
}

impl Module {
    pub fn get_data<'r>(&'r mut self,n: String) -> &'r Memory {
        self.data.get(&n).unwrap()
    }
}

//TODO: AOT Backend
/*use faerie::*;
use target_lexicon::triple;
use std::str::FromStr;

pub struct FaerieModule {
    functions: HashMap<String,Function>,
    object: Artifact,
    data: HashMap<String,Memory>,
}

impl FaerieModule {
    pub fn new() -> FaerieModule {
        FaerieModule {
            functions: HashMap::new(),
            data: HashMap::new(),
            object: ArtifactBuilder::new(triple!("x86_64-unknown-linux-gnu")).name("main".into()).finish(),

        }
    }

    pub fn add_function(&mut self,f: Function) {
        if let Linkage::Extern(_) = f.linkage() {
            self.object.declare(f.name.clone(), Decl::FunctionImport).expect(&format!("Failed to declare extern function `{}`",f.name));
        } else {
            self.object.declare(f.name.clone(),Decl::Function{global: true});
        }

        self.functions.insert(f.name.clone(), f);
    }
    pub fn get_mut_func<'r>(&'r mut self,s: String) -> &'r mut Function {
        self.functions.get_mut(&s).unwrap()
    }

    pub fn get_func<'r>(&'r self,s: String) -> &'r Function {
        self.functions.get(&s).unwrap()
    }

    pub fn finish(&mut self) {
        for (name,fct) in self.functions.iter_mut() {
            let func: &mut Function = fct;

            match func.linkage() {
                Linkage::Local => {},
                _ => continue,
            }


        }
        
        self.fix_calls();

        
    }

    pub fn fix_calls(&mut self) {
        let fcts = self.functions.clone();
        use crate::function::CallFixup;

        for (_,fct) in fcts.iter() {
            let fct: &Function = fct;
            for fixup in fct.fixups.iter() {
                let fixup: CallFixup = fixup.clone();
                let f: Memory = self.data.get(&fixup.name).unwrap().clone();

                let current: &mut Memory = self.data.get_mut(&fct.name).unwrap();
                let curr_ptr = current.ptr();
                let ptr = f.ptr();


                unsafe {
                    let off = ptr.offset(0);
                    let slice: [u8;8] = mem::transmute(off);
                    let mut pc = 0;
                    for i in fixup.pos..fixup.pos + 8 {
                        let byte = &mut *(curr_ptr.offset(i as isize) as *mut u8);
                        *byte = slice[pc].clone();
                        pc += 1;
                    }
                }
            }
        }

    }
}

*/