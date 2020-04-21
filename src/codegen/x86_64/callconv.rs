use crate::codegen::*;
use crate::ir::*;
#[derive(Clone, Debug)]
pub enum CallConvResult {
    GPR(Box<Node>),
    GPREX(Box<Node>, Box<Node>),
    FPR(Box<Node>),
    Stack,
}

pub mod c_callconv {
    use super::*;
    pub fn compute_stack_args(tys: &Vec<Type>) -> (usize, Vec<usize>) {
        let callconv = compute_arguments(tys);
        let mut stack_arg_tys = vec![];
        for i in 0..callconv.len() {
            let ref cc = callconv[i];
            match cc {
                &CallConvResult::Stack => stack_arg_tys.push(tys[i].clone()),
                _ => {}
            }
        }

        compute_stack_locations(&stack_arg_tys)
    }

    pub fn compute_stack_locations(stack_tys: &Vec<Type>) -> (usize, Vec<usize>) {
        let (stack_arg_size, _, stack_arg_offsets) = sequential_layout(stack_tys);

        let mut stack_arg_size_with_padding = stack_arg_size;
        if stack_arg_size % 16 == 0 {
            // do not need to adjust rsp
        } else if stack_arg_size % 8 == 0 {
            // adjust rsp by -8
            stack_arg_size_with_padding += 8;
        } else {
            let rem = stack_arg_size % 16;
            let stack_arg_padding = 16 - rem;
            stack_arg_size_with_padding += stack_arg_padding;
        }

        (stack_arg_size_with_padding, stack_arg_offsets)
    }

    pub fn compute_arguments(tys: &Vec<Type>) -> Vec<CallConvResult> {
        let mut ret = vec![];

        let mut gpr_arg_count = 0;
        let mut fpr_arg_count = 0;

        for ty in tys.iter() {
            let arg_reg_group = RegGroup::from_ty(*ty).unwrap();

            if arg_reg_group == RegGroup::GPR {
                if gpr_arg_count < x86_64::ARGUMENT_GPRS.len() {
                    let arg_gpr = {
                        let ref reg64 = x86_64::ARGUMENT_GPRS[gpr_arg_count];
                        let expected_len = ty.int_length();
                        x86_64::get_alias_for_length(reg64.any_reg_id(), expected_len)
                    };

                    ret.push(CallConvResult::GPR(Box::new((*arg_gpr).clone())));
                    gpr_arg_count += 1;
                } else {
                    // use stack to pass argument
                    ret.push(CallConvResult::Stack);
                }
            } else if arg_reg_group == RegGroup::GPREX {
                // need two regsiters for this, otherwise, we need to pass on
                // stack
                /*if gpr_arg_count + 1 < x86_64::ARGUMENT_GPRS.len() {
                    let arg_gpr1 = x86_64::ARGUMENT_GPRS[gpr_arg_count].clone();
                    let arg_gpr2 = x86_64::ARGUMENT_GPRS[gpr_arg_count + 1].clone();

                    ret.push(CallConvResult::GPREX(arg_gpr1, arg_gpr2));
                    gpr_arg_count += 2;
                } else {
                    ret.push(CallConvResult::STACK);
                }*/
                unimplemented!()
            } else if arg_reg_group == RegGroup::FPR {
                if fpr_arg_count < x86_64::ARGUMENT_FPRS.len() {
                    let arg_fpr = x86_64::ARGUMENT_FPRS[fpr_arg_count].clone();

                    ret.push(CallConvResult::FPR(Box::new((*arg_fpr).clone())));
                    fpr_arg_count += 1;
                } else {
                    ret.push(CallConvResult::Stack);
                }
            } else {
                // fp const, struct, etc
                unimplemented!();
            }
        }

        ret
    }
}
