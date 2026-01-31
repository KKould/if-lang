use if_lang::eval::{Args, BuiltinContext, BuiltinFn, EvalError, Value, register_builtin_args};
use std::collections::HashMap;

#[unsafe(no_mangle)]
pub extern "C" fn if_lang_register(builtins: *mut HashMap<String, BuiltinFn>) {
    let builtins = unsafe { &mut *builtins };
    register_builtin_args(builtins, "append", append);
    register_builtin_args(builtins, "take_k", take_k);
}

fn append(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let left = args.list(0)?.to_vec();
    let right = args.list(1)?.to_vec();
    let mut combined = left;
    combined.extend(right);
    Ok(Value::List(combined))
}

fn take_k(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let list = args.list(0)?.to_vec();
    let k = args.int(1)?;
    let k = if k < 0 { 0 } else { k as usize };
    Ok(Value::List(list.into_iter().take(k).collect()))
}

fn main() {}
