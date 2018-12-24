extern crate frontend;

use self::frontend::abi::Linkage;
use self::frontend::function::{Function, Variable};
use self::frontend::kind::*;
use self::frontend::module::{Module,ModuleTrait};

extern crate capstone;

#[derive(Debug, Clone, Copy)]
struct P {
    x: i32,
    y: i32,
}

use self::capstone::arch::*;
use self::capstone::*;

fn printi(i: i32) -> i32 {
    println!("number: {}", i);
    return i;
}

extern "C" {
    fn malloc(c: usize) -> *const u8;
}

fn main() {
    let mut module = Module::new();

    module.add_function(Function::new("main", Linkage::Local));
    module.add_function(Function::new(
        "printi",
        Linkage::Extern(printi as *const u8),
    ));
    module.add_function(Function::new(
        "malloc",
        Linkage::Extern(malloc as *const u8),
    ));
    let builder = module.get_mut_func("main".into());
    let size = builder.iconst(8, Int32);
    let ptr = builder.iconst(unsafe { malloc(8) as i64 }, Pointer);
    let point = builder.declare_variable(0, Pointer);
    builder.def_var(point, ptr);

    let point = builder.use_var(point);
    let x = builder.iconst(8, Int32);
    builder.store(Int32, point, 4, x);
    builder.store(Int32, point, 0, x);
    let point = builder.use_var(Variable::new(0));
    builder.ret(point);

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

    let f: fn() -> &'static P = unsafe { ::std::mem::transmute(mem.ptr()) };
    println!("{:?}", f());
}
