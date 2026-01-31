# IF Lang VS Code Extension

Features:
- Syntax highlighting for `.if` files.
- Diagnostics via `if_lang check` (parse + validation errors).
- Go to Definition / Find References (workspace-wide, best-effort).

## Usage
1) Build or install the CLI so `if_lang` is on your PATH (e.g. `cargo install if_lang`).
2) In VS Code, run “Developer: Install Extension from Location...” and select `editors/vscode`.
3) Open a `.if` file. Diagnostics run on save by default.

## Settings
- `iflang.cliPath`: Path to the `if_lang` CLI.
- `iflang.diagnosticsMode`: `onSave` (default), `onType`, or `off`.
- `iflang.diagnosticsDebounceMs`: Debounce for `onType` diagnostics.
