use crate::socket::{FunctionCall, FunctionResult};

pub fn print_function_result(call: &FunctionCall, result: &FunctionResult) {
    dbg!(call);
    dbg!(result);
}
