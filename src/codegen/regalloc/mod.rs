pub mod graph_coloring;
pub mod liveness;
pub mod minira;
use graph_coloring::GraphColoring;
pub struct RegisterAllocation;

use crate::ir::*;
use crate::pass::*;

impl<'a> FunctionPass<'a> for RegisterAllocation {
    type Output = ();
    type Err = ();
    fn run<'b: 'a>(&mut self, f: &'a mut LIRFunction) -> Result<Self::Output, Self::Err> {
        if f.loop_analysis.is_none() {
            let _ = crate::pass::loopanalysis::LoopAnalysisPass.run(f);
        }
        let coloring = GraphColoring::start(f);

        for (temp, machine_reg) in coloring.get_assignments() {
            for bb in coloring.cf.code.iter_mut() {
                for ins in bb.instructions.iter_mut() {
                    ins.replace_reg(temp, machine_reg);
                }
            }
        }
        Ok(())
    }
}
