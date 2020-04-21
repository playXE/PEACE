extern crate peace;
use peace::codegen::lower_function::LowerFunctionPass;
use peace::codegen::regalloc::RegisterAllocation;
use peace::ir::*;
use peace::pass::*;
fn main() {
    simple_logger::init().unwrap();
    let mut b = LIRFunctionBuilder::new("add2", &[Type::Int32], Type::Int32);
    b.add_function(FunctionSignature::new(
        "square",
        &[Type::Int32],
        Type::Int32,
    ));
    let p = b.load_param(0);
    let imm = b.load_imm32(2);
    let res = b.int_binary(IntBinaryOperation::Add, imm, p);
    let res = b.call("square", &[res]);
    b.return_(res);
    let mut f = b.finish();
    f.print_to_stdout();
    let mut pass = LowerFunctionPass::new();
    let _ = pass.run(&mut f);
    let mut ra = RegisterAllocation;
    let _ = ra.run(&mut f);
    let mut peephole = peephole::PeepholePass;
    let _ = peephole.run(&mut f);
    f.print_to_stdout();
}
