use super::*;
use crate::ir::*;

pub struct PeepholePass;

impl<'b> FunctionPass<'b> for PeepholePass {
    type Output = ();
    type Err = ();
    fn run<'a: 'b>(&mut self, f: &'a mut LIRFunction) -> Result<Self::Output, Self::Err> {
        for bb in f.code.iter_mut() {
            bb.instructions.retain(|x| {
                if let Instruction::Move(x, y) = x {
                    if x == y {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            });
        }
        Ok(())
    }
}
