pub mod loopanalysis;
pub mod param_load;
pub mod ret_sink;
pub mod simplify_cfg;
use crate::{ir::*, module::*};
pub trait FunctionPass {
    type Output;
    type Err;
    fn run(&mut self, f: &mut LIRFunction) -> Result<Self::Output, Self::Err>;
}

pub trait ModulePass {
    type Output;
    type Err;
    fn run(&mut self, module: &mut Module) -> Result<Self::Output, Self::Err>;
}
