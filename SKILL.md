---
name: if-lang
description: Work with IF Lang source, syntax, and compiler pipeline in this repo; use when writing or editing .if programs, defining data/extern/fn/let items, or implementing minimal Rust/Python extras for externs under IF Lang rules.
---

# IF Lang Skill

## Core rules
- Keep IF Lang as the core logic; implement only the minimal required extern glue in extras.
- Functions are pure by default; no implicit state, mutation, or IO.
- Externs must be explicit: declare with `extern fn` and include `explain { ... }`.
- Enforce parameter order constraint: the first appearance order of parameters in the body must match the signature order (repeats allowed).
- Avoid deep `if` nesting; prefer `match` (or a helper fn) when branching grows.
- Keep all core logic in IF Lang; extras should only provide the minimal necessary operations.
- Prefer grouping code by responsibility and mark each block with concise comments.

## Program structure
- Top-level items must end with `;` (data, extern fn, fn, let).
- An optional final expression may appear after items; it may end with `;` (discard) or omit it (program result).

## Syntax essentials
- Variants are Uppercase; constructor literals are only parsed for Uppercase names.
- Constructors use named fields; field shorthand is allowed.
- Calls require a bare identifier callee; trailing commas are not allowed in call args or parameter lists.
- `explain` is reserved and cannot be used as an identifier.

## Expressions and patterns
- Pipe is intent-first: `x |> f(a)` desugars to `f(x, a)` and `x |> f` to `f(x)`.
- Use `|>` only to flatten nested calls; for simple single-call cases, prefer `f(x)`.
- Match arms may be separated by `;` or `,`; trailing separator is allowed.
- `_` wildcard may appear at most once and must be last.
- Compare patterns (`<`, `<=`, `>`, `>=`) and variant patterns with field destructuring are supported.
- `match` scrutinee cannot be a constructor literal with braces; bind first with `let`.

## Extras (host implementations)
- Rust or Python extras are allowed; keep them minimal and focused on externs.
- Python contract: define `if_lang_register(registry)`, assign `registry["name"] = func`.
- Externs are called as `func(args, ctx)`; call back into IF with `ctx.call_fn(name, args)`.

## Validation checklist
- Every top-level item ends with `;`.
- Externs include `explain { ... }`.
- Parameter order constraint is satisfied.
- Constructors and variants use Uppercase names.
- Calls use bare identifiers and have no trailing commas.
- Pipe used only to flatten nesting.
