pub mod loopanalysis;
pub mod param_load;
pub mod peephole;
pub mod ret_sink;
pub mod simplify_cfg;
use crate::{ir::*, module::*};
pub trait FunctionPass<'b> {
    type Output;
    type Err;
    fn run<'a: 'b>(&mut self, f: &'a mut LIRFunction) -> Result<Self::Output, Self::Err>;
}

pub trait ModulePass {
    type Output;
    type Err;
    fn run<'a>(&mut self, module: &mut Module) -> Result<Self::Output, Self::Err>;
}
