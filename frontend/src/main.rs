extern crate peace_frontend;

use self::peace_frontend::abi::Linkage;
use self::peace_frontend::function::{Function};
use self::peace_frontend::kind::*;
use self::peace_frontend::module::{Module, ModuleTrait};

extern crate capstone;


use self::capstone::arch::*;
use self::capstone::*;



extern "C" {
    fn puts();
}

fn main() {
    let mut module = Module::new();

    module.add_function(Function::new("main", Linkage::Local));
    module.add_function(Function::new("puts",Linkage::Extern(puts as *const u8)));

    let builder = module.get_mut_func("main".into());

    let cstring = builder.iconst(b"Hello,world!\0".as_ptr() as i64, Int64);
    builder.call_indirect("puts", &[cstring], Int32);
    let iconst = builder.iconst(0, Int32);
    builder.ret(iconst);

    module.finish();

    let mem = module.get_data("main".into());
    let buf: &[u8] = unsafe { ::std::slice::from_raw_parts(mem.ptr(), mem.size()) };

    let mut cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Intel)
        .detail(true)
        .build()
        .expect("Failed to create Capstone object");

    let insns = cs.disasm_all(buf, mem.ptr() as u64);
    for i in insns.iter() {
        println!("{}", i);
    }

    let f: fn() -> i32 = unsafe { ::std::mem::transmute(mem.ptr()) };
    println!("{:?}", f());
}
