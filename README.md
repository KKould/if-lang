# IF Lang

<p align="left">
    <a href="https://crates.io/crates/if_lang/"><img src="https://img.shields.io/crates/v/if_lang.svg"></a>
    <a href="https://github.com/KKould/if-lang" target="_blank">
    <img src="https://img.shields.io/github/stars/KKould/if-lang.svg?style=social" alt="github star"/>
    <img src="https://img.shields.io/github/forks/KKould/if-lang.svg?style=social" alt="github fork"/>
  </a>
</p>

An intent-first, constrained general-purpose functional language designed for **clear meaning** and **low token usage**. It favors pure functions, explicit data flow, and no hidden state.

Goal: help LLMs generate minimal detail and clear computational intent, with strict constraints and compile/eval layers ensuring deterministic semantics.

For the full language notes, see [AGENTS.md](AGENTS.md).

## Why this language
> Think of IF Lang as “SQL for logic”, not for data.

This project is built to address three practical pain points when using LLMs for code generation:
1) unclear expression of intent (high review cost, overgrown scaffolding),
2) hallucinated or noisy implementation details (diluted intent, extra context),
3) inflated token usage and higher cost from the above.

IF Lang is intended as a **pre‑implementation logic layer**: LLMs first generate a small, precise
POC or draft in IF Lang for human review, then either a human or an LLM produces the concrete
implementation in the target language.

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

Function references:
```
fn handle(req) = req;
extern fn serve(port, handler);
serve(8080, handle)
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

Strings and bytes:
```
"hello"
b"hello"
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
- BST Top-K (IF Lang source): [examples/bst_topk.if](examples/bst_topk.if)
- Extra methods (Rust source): [examples/bst_topk_extra.rs](examples/bst_topk_extra.rs)
- Mini web (IF Lang source): [examples/mini_web.if](examples/mini_web.if)
- Mini web extra (Rust source): [examples/mini_web_extra.rs](examples/mini_web_extra.rs)

One-line scripts (requires `cargo install if_lang`):
```
./scripts/run_bst_topk.sh
./scripts/run_mini_web.sh
```
Note: you can also pass a prebuilt dylib (`.so`, `.dylib`, `.dll`).

## Notes
- Constructors are **Uppercase** and use field syntax: `Node { value, left, right }`
- `match` supports field destructuring and comparison patterns (e.g. `>= 80`)
- Extern functions must be **declared** and **registered** at runtime
