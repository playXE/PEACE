/// Lower some IR instructions into other.
///
/// Example:
/// `%r200 = load_param.32 0`
/// will be lowered to
/// `move %r200, %edi`
pub struct LowerFunctionPass<'a> {
    new_code: Vec<BasicBlock>,
    func: Option<&'a mut LIRFunction>,
    current_block: usize,
}

use super::callconv::c_callconv::*;
use super::callconv::*;
use super::*;
use crate::codegen::*;
use crate::ir::*;
use crate::pass::*;
impl<'a> LowerFunctionPass<'a> {
    fn emit_store_stack_values(
        &mut self,
        stack_vals: &Vec<Box<Node>>,
        base: Option<(&Box<Node>, i32)>,
    ) -> usize {
        let stack_arg_tys = stack_vals
            .iter()
            .map(|x| LIRFunctionBuilder::ty_info(&**x))
            .collect::<Vec<_>>();
        let (stack_arg_size_with_padding, stack_arg_offsets) =
            compute_stack_locations(&stack_arg_tys);

        {
            if stack_arg_size_with_padding != 0 {
                let mut index = 0;
                let rsp_offset_before_call = -(stack_arg_size_with_padding as i32);

                for arg in stack_vals {
                    if let Some((base, offset)) = base {
                        self.emit_store_base_offset(
                            base,
                            offset + (stack_arg_offsets[index]) as i32,
                            &arg,
                        );
                    } else {
                        self.emit_store_base_offset(
                            &**x86_64::RSP,
                            rsp_offset_before_call + (stack_arg_offsets[index] as i32),
                            &arg,
                        );
                    }
                    index += 1;
                }
            }
            stack_arg_size_with_padding
        }
    }
    fn emit(&mut self, ins: Instruction) {
        self.new_code[self.current_block].instructions.push(ins);
    }
    fn emit_store_base_offset(&mut self, base: &Node, offset: i32, src: &Box<Node>) {
        self.emit(Instruction::Store(
            Box::new(base.clone()),
            Box::new(Node::Operand(Operand::Immediate32(offset))),
            src.clone(),
            LIRFunctionBuilder::ty_info(src),
        ))
    }

    fn emit_load_base_offset(&mut self, dest: &Node, base: &Node, offset: i32) -> Box<Node> {
        self.emit(Instruction::Load(
            Box::new(dest.clone()),
            Box::new(base.clone()),
            Box::new(Node::Operand(Operand::Immediate32(offset))),
            Type::UInt8,
        ));
        Box::new(Node::Operand(Operand::Memory(MemoryLocation::Address {
            base: Box::new(base.clone()),
            offset: Some(Box::new(Node::Operand(Operand::Immediate32(offset)))),
            index: None,
            scale: None,
        })))
    }

    fn lower_call(
        &mut self,
        func: Box<Node>,
        sig: &FunctionSignature,
        args: Vec<Box<Node>>,
        rets: Box<Node>,
    ) {
        let (stack_args, args) = self.emit_precall_convention(&sig, &args);
        self.emit(Instruction::RawCall(rets, func));
    }

    fn emit_precall_convention(
        &mut self,
        sig: &FunctionSignature,
        args: &Vec<Box<Node>>,
    ) -> (usize, Vec<Box<Node>>) {
        let callconv = compute_arguments(&sig.params);
        let (reg_args, stack_args) = self.emit_precall_convention_regs_only(args, &callconv);

        if !stack_args.is_empty() {
            let size = self.emit_store_stack_values(&stack_args, None);
            let rsp = Box::new((**RSP).clone());
            self.emit(Instruction::IntBinary(
                IntBinaryOperation::Sub,
                rsp.clone(),
                rsp.clone(),
                Box::new(Node::Operand(Operand::Immediate32(size as _))),
            ));
            (size, reg_args)
        } else {
            (0, reg_args)
        }

        //self.emit_unload_values(, callconv: &[CallConvResult], stack_arg_offsets: &Vec<usize>, stack_pointer: Option<(&Box<Node>, i32)>, is_unloading_args: bool)
    }
    fn emit_precall_convention_regs_only(
        &mut self,
        args: &Vec<Box<Node>>,
        callconv: &[CallConvResult],
    ) -> (Vec<Box<Node>>, Vec<Box<Node>>) {
        let mut stack_args = vec![];
        let mut reg_args = vec![];

        for i in 0..callconv.len() {
            let ref arg = args[i];
            let ref cc = callconv[i];
            match cc {
                &CallConvResult::GPR(ref reg) => {
                    reg_args.push(reg.clone());
                    match &**arg {
                        Node::Operand(op) => match op {
                            Operand::Immediate64(_)
                            | Operand::Immediate32(_)
                            | Operand::Immediate16(_)
                            | Operand::Immediate8(_) => {
                                self.emit(Instruction::LoadImm(
                                    reg.clone(),
                                    arg.clone(),
                                    Type::Int64,
                                ));
                            }
                            Operand::Register(_, _) => {
                                self.emit(Instruction::Move(reg.clone(), arg.clone()));
                            }
                            _ => panic!("arg {} is put to GPR, but it is neither reg or const"),
                        },
                        _ => panic!("arg {} is put to GPR, but it is neither reg or const"),
                    }
                }
                &CallConvResult::FPR(ref reg) => {
                    reg_args.push(reg.clone());
                    match &**arg {
                        Node::Operand(op) => match op {
                            Operand::Register(_, _) => {
                                self.emit(Instruction::Move(reg.clone(), arg.clone()));
                            }
                            _ => unimplemented!(),
                        },
                        _ => unreachable!(),
                    };
                }
                &CallConvResult::Stack => {
                    stack_args.push(arg.clone());
                }
                _ => unreachable!(),
            }
        }
        (reg_args, stack_args)
    }

    fn emit_unload_values(
        &mut self,
        rets: &[Box<Node>],
        callconv: &[CallConvResult],
        stack_arg_offsets: &Vec<usize>,
        stack_pointer: Option<(&Box<Node>, i32)>,
        is_unloading_args: bool,
    ) {
        let mut stack_args = vec![];

        for i in 0..callconv.len() {
            let ref cc = callconv[i];
            let ref val = rets[i];
            match cc {
                &CallConvResult::GPR(ref reg) => {
                    self.emit(Instruction::Move(val.clone(), reg.clone()));
                    if is_unloading_args {
                        self.func
                            .as_mut()
                            .unwrap()
                            .frame
                            .add_argument_by_reg(val.any_reg_id(), reg.clone());
                    }
                }
                &CallConvResult::FPR(ref reg) => {
                    self.emit(Instruction::Move(val.clone(), reg.clone()));
                    if is_unloading_args {
                        self.func
                            .as_mut()
                            .unwrap()
                            .frame
                            .add_argument_by_reg(val.any_reg_id(), reg.clone());
                    }
                }
                &CallConvResult::Stack => stack_args.push(val.clone()),
                _ => unreachable!(),
            }
        }
        if !stack_args.is_empty() {
            for i in 0..stack_args.len() {
                let ref arg = stack_args[i];
                let offset = stack_arg_offsets[i] as i32;

                let stack_slot = if let Some((base, base_offset)) = stack_pointer {
                    self.emit_load_base_offset(arg, base, base_offset + offset)
                } else {
                    self.emit_load_base_offset(arg, &x86_64::RSP, offset)
                };

                if is_unloading_args {
                    self.func
                        .as_mut()
                        .unwrap()
                        .frame
                        .add_argument_by_stack(arg.any_reg_id(), stack_slot);
                }
            }
        }
    }

    pub fn new() -> Self {
        Self {
            new_code: vec![],
            func: None,
            current_block: 0,
        }
    }
}

impl<'a> FunctionPass<'a> for LowerFunctionPass<'a> {
    type Output = ();
    type Err = ();
    fn run<'b: 'a>(&mut self, f: &'b mut LIRFunction) -> Result<Self::Output, Self::Err> {
        let old = f.code.clone();
        let arguments = compute_arguments(&f.signature.params);
        self.func = Some(f);
        self.new_code.clear();
        /*self.new_code.push(BasicBlock::new(0, vec![]));
        self.current_block = 0;*/
        for (i, bb) in old.iter().enumerate() {
            self.new_code.push(BasicBlock::new(i, vec![]));
            self.current_block = i;
            for ins in bb.instructions.iter() {
                match ins {
                    Instruction::Call(dst, f, args, sig)
                    | Instruction::CallIndirect(dst, f, args, sig) => {
                        self.lower_call(f.clone(), sig, args.clone(), dst.clone());
                    }
                    Instruction::LoadParam(dst, n, t) => match &arguments[i] {
                        CallConvResult::GPR(r) | CallConvResult::FPR(r) => {
                            self.emit(Instruction::Move(dst.clone(), r.clone()));
                        }
                        CallConvResult::Stack => {
                            self.emit_load_base_offset(&dst, &**RSP, i as i32 * (t.size() as i32));
                        }
                        _ => unimplemented!(),
                    },
                    Instruction::IntBinary(op, dst, lhs, rhs) => {
                        self.emit(Instruction::RawIntBinary(*op, lhs.clone(), rhs.clone()));
                        if lhs != dst {
                            self.emit(Instruction::Move(dst.clone(), lhs.clone()));
                        }
                    }
                    Instruction::FloatBinary(op, dst, lhs, rhs) => {
                        self.emit(Instruction::RawFloatBinary(*op, lhs.clone(), rhs.clone()));
                        if lhs != dst {
                            self.emit(Instruction::Move(dst.clone(), lhs.clone()));
                        }
                    }
                    x => self.emit(x.clone()),
                }
            }
        }
        self.func.as_mut().unwrap().code = self.new_code.clone();
        Ok(())
    }
}
