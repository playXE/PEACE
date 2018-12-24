extern crate backend;

use self::backend::registers::*;
use self::backend::sink::*;
use self::backend::types::*;

extern crate capstone;

use self::capstone::arch::*;
use self::capstone::*;

extern "C" {
    fn printf();
}
fn main() {
    let mut sink = Sink::new();
    sink.emit_prolog();
    sink.load_int(I64, RDI, b"Hello,world!\n\0".as_ptr() as i64);
    sink.load_int(I64, RAX, printf as i64);
    sink.emit_call_reg(RAX);
    sink.emit_epilog();
    sink.ret();

    let mem = get_executable_memory(&sink);
    let f = mem.ptr();

    let buf: &[u8] = unsafe { ::std::slice::from_raw_parts(mem.ptr(), mem.size()) };

    let mut cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Att)
        .detail(true)
        .build()
        .expect("Failed to create Capstone object");

    let insns = cs.disasm_all(buf, mem.ptr() as u64);
    for i in insns.iter() {
        println!("{}", i);
    }

    let f: fn() -> i32 = unsafe { ::std::mem::transmute(f) };
    f();
}
