use std::collections::HashMap;
use std::sync::Arc;

use if_lang::eval::{BuiltinContext, BuiltinFn, EvalError, Value, register_builtin};

#[unsafe(no_mangle)]
pub extern "C" fn if_lang_register(builtins: *mut HashMap<String, BuiltinFn>) {
    let builtins = unsafe { &mut *builtins };
    register_builtin(builtins, "append", Arc::new(append));
    register_builtin(builtins, "take_k", Arc::new(take_k));
}

fn append(args: &[Value], _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::new("append expects 2 args"));
    }
    let left = match &args[0] {
        Value::List(items) => items.clone(),
        _ => return Err(EvalError::new("append expects List")),
    };
    let right = match &args[1] {
        Value::List(items) => items.clone(),
        _ => return Err(EvalError::new("append expects List")),
    };
    let mut combined = left;
    combined.extend(right);
    Ok(Value::List(combined))
}

fn take_k(args: &[Value], _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::new("take_k expects 2 args"));
    }
    let list = match &args[0] {
        Value::List(items) => items.clone(),
        _ => return Err(EvalError::new("take_k expects List")),
    };
    let k = match &args[1] {
        Value::Int(v) => *v,
        _ => return Err(EvalError::new("take_k expects Int")),
    };
    let k = if k < 0 { 0 } else { k as usize };
    Ok(Value::List(list.into_iter().take(k).collect()))
}

fn main() {}
