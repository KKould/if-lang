use if_lang::eval::{Args, BuiltinContext, BuiltinFn, EvalError, Value, register_builtin_args};
use std::collections::HashMap;

#[unsafe(no_mangle)]
pub extern "C" fn if_lang_register(builtins: *mut HashMap<String, BuiltinFn>) {
    let builtins = unsafe { &mut *builtins };
    register_builtin_args(builtins, "list_len", list_len);
    register_builtin_args(builtins, "list_get", list_get);
    register_builtin_args(builtins, "list_set", list_set);
    register_builtin_args(builtins, "list_push", list_push);
    register_builtin_args(builtins, "str_split_ws", str_split_ws);
    register_builtin_args(builtins, "str_trim_end_char", str_trim_end_char);
    register_builtin_args(builtins, "str_to_int", str_to_int);
    register_builtin_args(builtins, "assert_eq", assert_eq);
}

fn list_len(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(1)?;
    let items = args.list(0)?;
    Ok(Value::Int(items.len() as i64))
}

fn list_get(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let items = args.list(0)?;
    let idx = args.int(1)?;
    if idx < 0 || idx as usize >= items.len() {
        return Err(EvalError::new("list_get index out of bounds"));
    }
    Ok(items[idx as usize].clone())
}

fn list_set(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(3)?;
    let items = args.list(0)?.to_vec();
    let idx = args.int(1)?;
    if idx < 0 || idx as usize >= items.len() {
        return Err(EvalError::new("list_set index out of bounds"));
    }
    let mut out = items;
    out[idx as usize] = args.value_ref(2)?.clone();
    Ok(Value::List(out))
}

fn list_push(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let mut items = args.list(0)?.to_vec();
    items.push(args.value_ref(1)?.clone());
    Ok(Value::List(items))
}

fn str_split_ws(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(1)?;
    let value = args.str(0)?;
    let parts = value
        .split_whitespace()
        .map(|s| Value::Str(s.to_string()))
        .collect::<Vec<_>>();
    Ok(Value::List(parts))
}

fn str_trim_end_char(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let value = args.str(0)?;
    let suffix = args.str(1)?;
    let mut chars = suffix.chars();
    let ch = match (chars.next(), chars.next()) {
        (Some(ch), None) => ch,
        _ => return Err(EvalError::new("str_trim_end_char expects 1-char suffix")),
    };
    let trimmed = value.strip_suffix(ch).unwrap_or(value);
    Ok(Value::Str(trimmed.to_string()))
}

fn str_to_int(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(1)?;
    let value = args.str(0)?;
    let parsed = value
        .parse::<i64>()
        .map_err(|_| EvalError::new("str_to_int invalid Int"))?;
    Ok(Value::Int(parsed))
}

fn assert_eq(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let left = args.value_ref(0)?;
    let right = args.value_ref(1)?;
    if left == right {
        Ok(Value::Bool(true))
    } else {
        Err(EvalError::new(format!(
            "assert_eq failed: left={:?} right={:?}",
            left, right
        )))
    }
}

fn main() {}
