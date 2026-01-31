use std::collections::HashMap;
use std::sync::Arc;

use if_lang::eval::{eval_program_with_builtins, BuiltinFn, EvalError, Value};
use if_lang::lexer::Lexer;
use if_lang::lower::lower_program;
use if_lang::parser::parse_program;
use if_lang::validate::validate_program;

const SOURCE: &str = r#"
// Ordered binary search tree with intent-first match + field destructuring.
data Tree = Empty | Node { value, left, right };

extern fn append(a, b);
extern fn take_k(xs, k);

fn insert(t, x) =
  match t {
    Empty => Node { value: x, left: Empty, right: Empty };
    Node { value, left, right } =>
      if x < value {
        Node { value, left: insert(left, x), right }
      } else {
        Node { value, left, right: insert(right, x) }
      };
  };

fn to_desc_list(t) =
  match t {
    Empty => [];
    Node { value, left, right } =>
      append(to_desc_list(right), append([value], to_desc_list(left)));
  };

fn topk(t, k) =
  t
    |> to_desc_list
    |> take_k(k);

let t0 = Empty;
let t1 = insert(t0, 5);
let t2 = insert(t1, 2);
let t3 = insert(t2, 8);
let t4 = insert(t3, 1);
let t5 = insert(t4, 3);
let t6 = insert(t5, 7);

t6 |> topk(3)
"#;

fn main() {
    let tokens = Lexer::new(SOURCE).lex_all();
    let surface = parse_program(&tokens).expect("parse");
    validate_program(&surface).expect("validate");
    let core = lower_program(surface);

    let builtins = register_builtins();
    let result = eval_program_with_builtins(&core, &builtins)
        .expect("eval")
        .expect("value");
    assert_eq!(
        result,
        Value::List(vec![Value::Int(8), Value::Int(7), Value::Int(5)])
    );
}

fn register_builtins() -> HashMap<String, BuiltinFn> {
    let mut builtins: HashMap<String, BuiltinFn> = HashMap::new();
    builtins.insert("append".to_string(), Arc::new(append));
    builtins.insert("take_k".to_string(), Arc::new(take_k));
    builtins
}

fn append(args: &[Value]) -> Result<Value, EvalError> {
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

fn take_k(args: &[Value]) -> Result<Value, EvalError> {
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
