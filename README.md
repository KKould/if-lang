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

## Syntax

Full syntax reference: [SYNTAX.md](SYNTAX.md).

## Codex skill

This repo includes a Codex skill for IF Lang: [SKILL.md](SKILL.md). Use it to guide IF Lang edits, enforce constraints (externs, parameter order, pipeline style), and keep logic in IF Lang with minimal extras.

## Example
- BST Top-K
  - IF: [examples/bst_topk.if](examples/bst_topk.if)
  - Rust extra: [examples/bst_topk_extra.rs](examples/bst_topk_extra.rs)
  - Python extra: [examples/bst_topk_extra.py](examples/bst_topk_extra.py)
- Mini web
  - IF: [examples/mini_web.if](examples/mini_web.if)
  - Rust extra: [examples/mini_web_extra.rs](examples/mini_web_extra.rs)
  - Python extra: [examples/mini_web_extra.py](examples/mini_web_extra.py)
- Mini SQL
  - IF: [examples/mini_sql.if](examples/mini_sql.if)
  - Rust extra: [examples/mini_sql_extra.rs](examples/mini_sql_extra.rs)
  - Python extra: [examples/mini_sql_extra.py](examples/mini_sql_extra.py)

One-line scripts (requires `cargo install if_lang`) — Python extra by default:
```
./scripts/run_bst_topk.sh
./scripts/run_mini_web.sh
./scripts/run_mini_sql.sh
```
Tip: pass `rust` as the optional parameter to run the Rust extra(e.g. `./scripts/run_mini_sql.sh rust`).

Run directly with extras:
```
# Python extra (.py)
if_lang extra examples/bst_topk_extra.py examples/bst_topk.if

# Rust extra (.rs, compiled to dylib on the fly)
if_lang extra examples/bst_topk_extra.rs examples/bst_topk.if

# Rust extra (prebuilt dylib)
if_lang extra path/to/libbst_topk_extra.so examples/bst_topk.if
```

## Notes
- Constructors are **Uppercase** and use field syntax: `Node { value, left, right }`
- `match` supports field destructuring and comparison patterns (e.g. `>= 80`)
- Extern functions must be **declared** with an `explain { ... }` block and **registered** at runtime

## Editor support
- VS Code extension (syntax highlighting + diagnostics): `editors/vscode`

### VS Code usage
1) Build or install the CLI so `if_lang` is available on your PATH (e.g. `cargo install if_lang`).
2) In VS Code, run “Developer: Install Extension from Location...” and select `editors/vscode`.
3) Open a `.if` file. Diagnostics run on save by default.

Settings:
- `iflang.cliPath`: Path to the `if_lang` CLI (default: `if_lang`).
- `iflang.diagnosticsMode`: `onSave` (default), `onType`, or `off`.
- `iflang.diagnosticsDebounceMs`: Debounce time (ms) for `onType`.

Features:
- Syntax highlighting.
- Diagnostics via `if_lang check`.
- Go to Definition / Find References (workspace-wide, best-effort).
