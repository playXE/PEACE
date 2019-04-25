extern crate peace;

use peace::module::*;
use peace::types::Type;


use capstone::prelude::*;
use std::mem;

fn main() {
    let mut module = Module::new();

    module.declare_function("printf", Linkage::Import);
    module.declare_function("main", Linkage::Local);


    let builder = module.get_function("main");
    let int = Type::I32;
    let v0 = builder.iconst(int, 4);
    let v1 = builder.iconst(int, 5);
    let v2 = builder.iadd(v0, v1);
    builder.ret(v2);
    builder.finalize();
    module.finish();

    let (data, size) = module.get_finalized_data("main");

    let cs: Capstone = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Intel)
        .detail(true)
        .build()
        .unwrap();

    let slice = unsafe { ::std::slice::from_raw_parts(data, size) };

    let ins = cs.disasm_all(slice, 0);

    for i in &ins {
        println!("{}", i);
    }


    let f: fn() -> i64 = unsafe { mem::transmute(data) };
    println!("{}", f());

}
