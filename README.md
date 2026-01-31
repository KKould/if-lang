# IF Lang

An intent-first, constrained general-purpose functional language designed for **clear meaning**
and **low token usage**. It favors pure functions, explicit data flow, and no hidden state.

Goal: help LLMs generate minimal detail and clear computational intent, with strict constraints
and compile/eval layers ensuring deterministic semantics.

For the full language notes, see [AGENTS.md](AGENTS.md).

## Why this DSL
- **Intent-first**: pipelines (`x |> f(a)`) read like steps.
- **Pure by default**: no implicit mutation or IO.
- **Explicit externs**: host-provided functions must be declared.
- **Strong constraints**: parameter usage order must match signature order.

## Syntax highlights (short)
```
data Tree = Empty | Node { value, left, right };

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
```

## Example
- BST Top-K: [examples/bst_topk.rs](examples/bst_topk.rs)

Run it:
```
cargo run --example bst_topk
```

## Notes
- Constructors are **Uppercase** and use field syntax: `Node { value, left, right }`
- `match` supports field destructuring and comparison patterns (e.g. `>= 80`)
- Extern functions must be **declared** and **registered** at runtime
