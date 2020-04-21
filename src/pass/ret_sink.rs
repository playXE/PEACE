use super::*;
use crate::ir::*;
/// Bytecode the client gives us may contain several Return instructions. However,
/// internally we want a single exit point for a function. In this pass, we
/// create a return sink (a block), and rewrite all the Return instruction into
/// a Branch with return values.
///
/// TODO: This pass is target dependent.
pub struct RetSinkPass;
