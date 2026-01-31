# IF Lang

An intent-first, constrained general-purpose functional language designed for **clear meaning** and **low token usage**. It favors pure functions, explicit data flow, and no hidden state.

Goal: help LLMs generate minimal detail and clear computational intent, with strict constraints and compile/eval layers ensuring deterministic semantics.

For the full language notes, see [AGENTS.md](AGENTS.md).

## Why this language
- **Intent-first**: pipelines (`x |> f(a)`) read like steps.
- **Pure by default**: no implicit mutation or IO.
- **Explicit externs**: host-provided functions must be declared.
- **Strong constraints**: parameter usage order must match signature order.

## Syntax highlights (short)

Data definitions:
```
data Tree = Empty | Node { value, left, right };
```

Externs:
```
extern fn take_k(xs, k);
```

Functions and bindings:
```
fn add(x, y) = x + y;
let n = 10;
```

Constructor and field shorthand:
```
Node { value: 1, left: Empty, right: Empty }
Node { value, left, right }
```

Lists and maps:
```
[1, 2, 3]
#{ 1: 2, 3: 4 }
```

If expression:
```
if x < 0 { 0 - x } else { x }
```

Pipe (intent-first):
```
xs |> take_k(2)
```

Match with compare + destructuring:
```
match x {
  >= 80 => 1;
  _ => 0;
}

match t {
  Node { value, left, right } => value;
  Empty => 0;
  _ => 0;
}
```

## Example
- BST Top-K: [examples/bst_topk.rs](examples/bst_topk.rs)

Run it:
```
cargo run --example bst_topk
```

## Externs in Rust (host implementation)
The DSL declares externs with `extern fn`. At runtime, you register the Rust
implementations and evaluate the program:

```rust
use std::collections::HashMap;
use std::sync::Arc;

use if_lang::eval::{eval_program_with_builtins, BuiltinFn, EvalError, Value, ValueKey};
use if_lang::lexer::Lexer;
use if_lang::lower::lower_program;
use if_lang::parser::parse_program;
use if_lang::validate::validate_program;

fn main() {
    let source = r#"
        extern fn take_k(xs, k);
        extern fn get(m, key);
        let xs = [9, 7, 5, 3];
        let cfg = #{ 0: 2 };
        let k = get(cfg, 0);
        xs |> take_k(k)
    "#;

    let tokens = Lexer::new(source).lex_all();
    let surface = parse_program(&tokens).expect("parse");
    validate_program(&surface).expect("validate");
    let core = lower_program(surface);

    let mut builtins: HashMap<String, BuiltinFn> = HashMap::new();
    builtins.insert("take_k".into(), Arc::new(take_k));
    builtins.insert("get".into(), Arc::new(get));

    let result = eval_program_with_builtins(&core, &builtins)
        .expect("eval")
        .expect("value");
    println!("{:?}", result);
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

fn get(args: &[Value]) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::new("get expects 2 args"));
    }
    let map = match &args[0] {
        Value::Map(m) => m,
        _ => return Err(EvalError::new("get expects Map")),
    };
    let key = match &args[1] {
        Value::Int(v) => ValueKey::Int(*v),
        Value::Bool(v) => ValueKey::Bool(*v),
        _ => return Err(EvalError::new("get expects Int/Bool key")),
    };
    map.get(&key)
        .cloned()
        .ok_or_else(|| EvalError::new("key not found"))
}
```

## Notes
- Constructors are **Uppercase** and use field syntax: `Node { value, left, right }`
- `match` supports field destructuring and comparison patterns (e.g. `>= 80`)
- Extern functions must be **declared** and **registered** at runtime
