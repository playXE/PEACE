extern crate peace;
use peace::ir::*;
fn main() {
    let mut b = LIRFunctionBuilder::new("add2", &[Type::Int64], Type::Int64);
    let p = b.load_param(0);
    let imm = b.load_imm64(2);
    let res = b.int_binary(IntBinaryOperation::Add, p, imm);
    b.return_(res);

    b.finish().print_to_stdout();
}
