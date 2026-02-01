# IF Lang Syntax Reference

This is the complete syntax reference for the current lexer/parser.

## Program structure

- A program is a sequence of top-level items, each ending with `;`.
- An optional final expression may appear after items. It may end with `;` (to discard it) or be left without `;` (to be the program result).
- Line comments start with `//` and run to end-of-line.
- Whitespace is insignificant outside of tokens.

EBNF (simplified):
```
program  = { item ";" } [ expr [";"] ] ;
item     = data_def | extern_fn | fn_def | let_def ;
```

## Lexical elements

Identifiers:
- Pattern: `[A-Za-z_][A-Za-z0-9_]*`
- `explain` is reserved (cannot be used as an identifier).

Keywords:
- `data`, `extern`, `fn`, `let`, `for`, `match`, `if`, `else`, `true`, `false`, `explain`

## Literals

- Int: decimal digits, parsed as `i64` (e.g. `0`, `42`).
- Bool: `true`, `false`.
- String: `"..."` with escapes `\n`, `\r`, `\t`, `\0`, `\\`, `\"`, `\xNN`.
- Bytes: `b"..."` with the same escapes as strings.

## Top-level items

### Data definitions
```
data Tree = Empty | Node { value, left, right };
```
- Variants may optionally declare fields: `Variant { field, field }`.
- By convention, variant names are Uppercase.
- Trailing commas are allowed in variant field lists.

Grammar:
```
data_def = "data" Ident "=" variant { "|" variant } ;
variant  = Ident [ "{" ident_list "}" ] ;
ident_list = Ident { "," Ident } ;
```

### Extern declarations
```
extern fn take_k(xs, k) explain { Returns first k items. };
```
- `explain { ... }` is required.
- `explain { ... }` may contain nested braces; the content is trimmed.

Grammar:
```
extern_fn = "extern" "fn" Ident "(" [params] ")" explain_block ;
```

### Functions
```
fn add(x, y) = x + y;
```

Grammar:
```
fn_def = "fn" Ident "(" [params] ")" "=" expr ;
```

### Let bindings
```
let n = 10;
```

Grammar:
```
let_def = "let" Ident "=" expr ;
```

### Params
```
params = Ident { "," Ident } ;
```
Note: trailing commas are not allowed in parameter lists.

## Expressions

EBNF (simplified):
```
expr        = pipe ;
pipe        = logic_or { "|>" pipe_target } ;
pipe_target = Ident [ "(" [args] ")" ] ;
logic_or    = logic_and { "||" logic_and } ;
logic_and   = equality { "&&" equality } ;
equality    = comparison { ("==" | "!=") comparison } ;
comparison  = term { ("<" | "<=" | ">" | ">=") term } ;
term        = factor { ("+" | "-") factor } ;
factor      = unary { ("*" | "/" | "%") unary } ;
unary       = ("-" | "!") unary | postfix ;
postfix     = primary [ "(" [args] ")" ] ;
primary     = literal
           | Ident
           | list
           | map
           | construct
           | "(" expr ")"
           | if_expr
           | match_expr
           | for_expr ;
args        = expr { "," expr } ;
range_list  = expr ".." expr ;
```

### Variables
```
x
handle_request
```

### Lists
```
[1, 2, 3]
[]
[1..10]
```
- Trailing comma is allowed: `[1, 2, ]`.
- `[start..end]` is a range list (both ends inclusive).

### Maps
```
#{ 1: 2, 3: 4 }
#{}
```
- Trailing comma is allowed: `#{ 1: 2, }`.

### Constructors
```
Node { value: 1, left: Empty, right: Empty }
Node { value, left, right }
```
- Field shorthand: `value` is equivalent to `value: value`.
- Constructor literals are parsed only when the name starts with an Uppercase letter.
- If a variant has zero fields, it can be referenced by name alone: `Empty`.
- Trailing commas are allowed in constructor field lists.

Grammar:
```
construct = UpperIdent "{" [construct_fields] "}" ;
construct_fields = construct_field { "," construct_field } ;
construct_field = Ident [ ":" expr ] ;
```
UpperIdent means an identifier starting with an uppercase letter.

### Function calls
```
add(1, 2)
```
- Calls require the callee to be a bare identifier, not an arbitrary expression.
- Trailing commas are not allowed in call argument lists.

### Pipe (intent-first)
```
xs |> take_k(2)
xs |> normalize |> score(0.8)
```
- `x |> f(a)` desugars to `f(x, a)`.
- `x |> f` desugars to `f(x)`.

### If expression
```
if x < 0 { 0 - x } else { x }
```

### Match expression
```
match x {
  >= 80 => 1;
  _ => 0;
}
```
- Arms may be separated by `;` or `,`.
- A trailing `;` or `,` before `}` is allowed.
- `_` may appear at most once and must be last.

### For expression
```
for x in xs { f(x) }
for x in xs if cond { f(x) }
```
- Evaluates `xs` (must be a `List`) and returns a list of the body results.
- The loop variable is scoped to the body expression.
- Guard expression (`if cond`) must evaluate to `Bool`; if false, the element is skipped.

Grammar:
```
for_expr = "for" Ident "in" expr ["if" expr] "{" expr "}" ;
```

## Match patterns

```
match_pattern = "_"
              | compare_pattern
              | variant_pattern
              | expr ;
```

### Compare patterns
```
< 10
<= 10
> 10
>= 10
```

### Variant patterns
```
Node { value, left, right }
Node { value: v, left: l, right: _ }
Empty
```
- Field shorthand: `value` is equivalent to `value: value`.
- `field: _` ignores a field.
- Pattern fields are a subset match: only listed fields are checked/bound.
- Trailing commas are allowed in pattern field lists.

Grammar:
```
variant_pattern = Ident [ "{" pattern_fields "}" ] ;
pattern_fields = pattern_field { "," pattern_field } ;
pattern_field = Ident [ ":" (Ident | "_") ] ;
```

## Operator precedence (highest to lowest)

1) Call: `f(x)`
2) Unary: `-x`, `!x`
3) `*` `/` `%`
4) `+` `-`
5) `<` `<=` `>` `>=`
6) `==` `!=`
7) `&&`
8) `||`
9) Pipe: `|>`

## Notes and limitations

- Constructor literals with braces are not allowed directly in `match` scrutinee position.
  Use a binding instead: `let t = Node { ... }; match t { ... }`.
- `explain` is reserved and must be followed by `explain { ... }` in extern declarations.
- Only `//` line comments are supported (no block comments).
