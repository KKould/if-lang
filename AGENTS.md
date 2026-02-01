# IF Lang Overview

This project is an intent-first, constrained general-purpose functional language with a small compiler pipeline: **lex/parse -> validate -> lower -> eval**. The language emphasizes clear intent, low token usage, and no implicit state or side effects.

## Core ideas
- **Pure functions by default**: no implicit state, mutation, or IO.
- **Intent-first pipelines**: `x |> f(a)` means `f(x, a)` (input becomes the first argument).
- **Pipeline style rule**: use `|>` only to flatten nested calls; for simple single-call cases, prefer `f(x)` over `x |> f`.
- **Externs are explicit**: any host-provided function must be declared with `extern fn` and an `explain { ... }` block.
- **Parameter order constraint**: for each function, the **first appearance order** of parameters in the body must match the signature order (repeats allowed).
- **Match clarity**: `match` is a high-level construct with field destructuring and comparisons.
- **Avoid deep if nesting**: when branching gets multi-level, prefer `match` (or a helper fn) to keep intent clear.
- **Agent rule (important)**: keep IF Lang as the core logic expression; implement only the minimal required methods in `extra`.
- **Logic placement**: all core logic must live in IF Lang; extras provide only the minimal necessary operations.
- **Structure**: group code by responsibility and mark sections with concise comments.

## Program structure
Top-level items must end with `;`:
- `data` type declarations
- `extern fn` declarations (must include `explain { ... }`)
- `fn` function definitions
- `let` immutable bindings

## Syntax summary

### Data definitions (untyped ADT)
```
data Tree = Empty | Node { value, left, right };
```
- Variants are **Uppercase**.
- Fields are **untyped** and **named**.

### Constructors (field-based)
```
Node { value: x, left, right }
```
- Field shorthand allowed: `left` == `left: left`.

### Functions and bindings
```
fn add(x, y) = x + y;
let n = 10;
```

### Expressions
- Literals: `Int`, `Bool`, `String`, `Bytes`
- Lists: `[1, 2, 3]`
- Maps: `#{ 1: 2, 3: 4 }`
- Unary: `-x`, `!x`
- Binary: `+ - * / % == != < <= > >= && ||`
- If: `if cond { expr } else { expr }`
- Call: `f(x, y)`
- Pipe: `x |> f(y)` (desugars to `f(x, y)`)

### Match
```
match x {
  >= 160 => 5;
  Node { value, left, right } => value;
  _ => 0;
}
```
Patterns:
- `_` wildcard (if present, must be **last**)
- Comparisons: `< expr`, `<= expr`, `> expr`, `>= expr`
- Variant patterns with field destructuring:
  - `Node { value, left, right }`
  - `Node { value: v, left: l, right: _ }`

## Evaluation model
- All functions are pure.
- Externs must be declared and **registered at runtime**.
- ADT values are represented as variants with field maps.
- Extras can be implemented in **Rust** (compiled to dylib) or **Python** (`.py`).

## Known limitations
- Constructors are parsed only for **Uppercase** names.
- `match` scrutinee does not accept constructor literals with braces directly;
  bind first with `let` if needed.

## Examples
- `examples/bst_topk.if`: BST Top-K IF Lang source.
- `examples/bst_topk_extra.rs`: Rust extra for externs used by bst_topk.
- `examples/bst_topk_extra.py`: Python extra for externs used by bst_topk.

## Python extra contract
- Provide `if_lang_register(registry)` and assign `registry["name"] = func`.
- Builtins are called as `func(args, ctx)` where `args` is a list of values.
- `ctx.call_fn(name, args)` can call back into IF functions.
