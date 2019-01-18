use capstone::prelude::*;
use std::mem;

use peak_compiler::module::*;
use peak_compiler::types::*;

fn main() {
    
    let mut module = Module::new();
    module.declare_function("main", Linkage::Local);
    module.declare_function("puts",Linkage::Import);


    let func = module.get_function("main");
    func.prolog();
    let str = func.iconst(I64,b"Hello,world!\0".as_ptr() as i64);
    let ret = func.call("puts",&[str],I32);
    func.ret(ret);


    func.fix_prolog();

    module.finish();

    let (ptr, size) = module.get_finalized_data("main");

    let mut cs: Capstone = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Intel)
        .detail(true)
        .build()
        .unwrap();

    let slice = unsafe { ::std::slice::from_raw_parts(ptr, size) };

    let ins = cs.disasm_all(slice, 0);

    for i in &ins {
        println!("{}", i);
    }

    let ptr = module.get_finalized_function("main");

    let f: fn() -> i32 = unsafe { mem::transmute(ptr) };
    println!("{}", f());
}
