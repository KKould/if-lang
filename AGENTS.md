# IF Lang Overview

This project is an intent-first, constrained general-purpose functional language with a small
compiler pipeline: **lex/parse -> validate -> lower -> eval**. The language emphasizes clear
intent, low token usage, and no implicit state or side effects.

## Core ideas
- **Pure functions by default**: no implicit state, mutation, or IO.
- **Intent-first pipelines**: `x |> f(a)` means `f(x, a)` (input becomes the first argument).
- **Externs are explicit**: any host-provided function must be declared with `extern fn`.
- **Parameter order constraint**: for each function, the **first appearance order** of parameters
  in the body must match the signature order (repeats allowed).
- **Match clarity**: `match` is a high-level construct with field destructuring and comparisons.

## Program structure
Top-level items must end with `;`:
- `data` type declarations
- `extern fn` declarations
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
- Literals: `Int`, `Bool`
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

## Known limitations
- Constructors are parsed only for **Uppercase** names.
- `match` scrutinee does not accept constructor literals with braces directly;
  bind first with `let` if needed.

## Examples
- `examples/bst_topk.rs`: BST Top-K with `data` + field destructuring.
