use crate::ast::surface::{
    DataDef, DataVariant, Expr, ExternFnDef, FieldPattern, FnDef, Item, LetDef, MatchArm,
    MatchPattern, PipeTarget, Program,
};
use crate::ast::{BinaryOp, UnaryOp};
use crate::lexer::{Lexer, TokenKind};

const INDENT: &str = "  ";
const MAX_LIST_INLINE_WIDTH: usize = 80;
const MAX_LIST_INLINE_ITEMS: usize = 4;
const MAX_INLINE_WIDTH: usize = 100;
const MAX_CALL_INLINE_WIDTH: usize = 80;
const MAX_CALL_INLINE_ITEMS: usize = 4;
const MAX_CONSTRUCT_INLINE_WIDTH: usize = 80;

pub fn format_program(program: &Program) -> String {
    let mut formatter = Formatter::new();
    formatter.fmt_program(program);
    formatter.out
}

pub fn format_source(source: &str, program: &Program) -> String {
    let comments = collect_comments(source);
    let tokens = Lexer::new(source).lex_all();
    let line_starts = line_starts(source);
    let item_lines = scan_top_level_item_lines(&tokens, &line_starts);
    let (inline_comments, item_comments) =
        distribute_comments(comments, &item_lines, program.items.len());
    let mut formatter = Formatter::with_comments(inline_comments, item_comments);
    formatter.fmt_program(program);
    formatter.emit_remaining_comments();
    formatter.out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Data,
    ExternFn,
    Fn,
    Let,
}

#[derive(Debug, Clone)]
struct Comment {
    indent: usize,
    text: String,
    line: usize,
}

struct Formatter {
    out: String,
    indent: usize,
    comments: Vec<Comment>,
    comment_idx: usize,
    item_comments: Vec<Vec<Comment>>,
    item_index: usize,
}

impl Formatter {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
            comments: Vec::new(),
            comment_idx: 0,
            item_comments: Vec::new(),
            item_index: 0,
        }
    }

    fn with_comments(comments: Vec<Comment>, item_comments: Vec<Vec<Comment>>) -> Self {
        Self {
            out: String::new(),
            indent: 0,
            comments,
            comment_idx: 0,
            item_comments,
            item_index: 0,
        }
    }

    fn fmt_program(&mut self, program: &Program) {
        let mut prev_kind: Option<ItemKind> = None;
        for item in &program.items {
            let kind = item_kind(item);
            let is_multiline = item_is_multiline(item, self.indent);
            let has_comments = self.has_item_comments();
            if let Some(prev) = prev_kind {
                if kind != prev || is_multiline || has_comments {
                    self.newline();
                }
            }
            self.emit_item_comments();
            self.fmt_item(item);
            prev_kind = Some(kind);
            self.item_index = self.item_index.saturating_add(1);
        }
        if let Some(expr) = &program.expr {
            if !program.items.is_empty() {
                self.newline();
            }
            self.emit_item_comments();
            self.fmt_expr_stmt(expr);
            self.newline();
        }
        if program.expr.is_none() && !self.out.ends_with('\n') {
            self.newline();
        }
        self.emit_comments(self.indent);
    }

    fn fmt_item(&mut self, item: &Item) {
        match item {
            Item::Data(def) => self.fmt_data_def(def),
            Item::ExternFn(def) => self.fmt_extern_fn(def),
            Item::Fn(def) => self.fmt_fn(def),
            Item::Let(def) => self.fmt_let(def),
        }
    }

    fn fmt_data_def(&mut self, def: &DataDef) {
        self.write_indent();
        self.out.push_str("data ");
        self.out.push_str(&def.name);
        self.out.push_str(" = ");
        for (idx, variant) in def.variants.iter().enumerate() {
            if idx > 0 {
                self.out.push_str(" | ");
            }
            self.out.push_str(&format_data_variant(variant));
        }
        self.out.push(';');
        self.newline();
    }

    fn fmt_extern_fn(&mut self, def: &ExternFnDef) {
        let params = join_names(&def.params);
        if def.explain.contains('\n') {
            self.write_indent();
            self.out.push_str("extern fn ");
            self.out.push_str(&def.name);
            self.out.push('(');
            self.out.push_str(&params);
            self.out.push_str(") explain {");
            self.newline();
            self.indent += 1;
            for line in def.explain.lines() {
                self.write_indent();
                self.out.push_str(line.trim());
                self.newline();
            }
            self.indent -= 1;
            self.write_indent();
            self.out.push_str("};");
            self.newline();
        } else {
            self.write_indent();
            self.out.push_str("extern fn ");
            self.out.push_str(&def.name);
            self.out.push('(');
            self.out.push_str(&params);
            self.out.push_str(") explain { ");
            self.out.push_str(def.explain.trim());
            self.out.push_str(" };");
            self.newline();
        }
    }

    fn fmt_fn(&mut self, def: &FnDef) {
        let params = join_names(&def.params);
        if expr_is_block(&def.body) {
            self.write_indent();
            self.out.push_str("fn ");
            self.out.push_str(&def.name);
            self.out.push('(');
            self.out.push_str(&params);
            self.out.push_str(") =");
            self.newline();
            self.indent += 1;
            self.fmt_expr_stmt(&def.body);
            self.indent -= 1;
            self.out.push(';');
            self.newline();
        } else {
            let expr_inline = format_expr_inline_with_indent(&def.body, self.indent + 1);
            let line_len = self.indent * INDENT.len()
                + "fn ".len()
                + def.name.len()
                + 2
                + params.len()
                + ") = ".len()
                + expr_inline.len()
                + 1;
            self.write_indent();
            self.out.push_str("fn ");
            self.out.push_str(&def.name);
            self.out.push('(');
            self.out.push_str(&params);
            if expr_inline.contains('\n') || line_len > MAX_INLINE_WIDTH {
                self.out.push_str(") =");
                self.newline();
                self.indent += 1;
                self.write_indent();
                self.out.push_str(&expr_inline);
                self.indent -= 1;
                self.out.push(';');
                self.newline();
                return;
            }
            self.out.push_str(") = ");
            self.out.push_str(&expr_inline);
            self.out.push(';');
            self.newline();
        }
    }

    fn fmt_let(&mut self, def: &LetDef) {
        if expr_is_block(&def.expr) {
            self.write_indent();
            self.out.push_str("let ");
            self.out.push_str(&def.name);
            self.out.push_str(" =");
            self.newline();
            self.indent += 1;
            self.fmt_expr_stmt(&def.expr);
            self.indent -= 1;
            self.out.push(';');
            self.newline();
        } else {
            let expr_inline = format_expr_inline_with_indent(&def.expr, self.indent + 1);
            let line_len = self.indent * INDENT.len()
                + "let ".len()
                + def.name.len()
                + " = ".len()
                + expr_inline.len()
                + 1;
            self.write_indent();
            self.out.push_str("let ");
            self.out.push_str(&def.name);
            if expr_inline.contains('\n') || line_len > MAX_INLINE_WIDTH {
                self.out.push_str(" =");
                self.newline();
                self.indent += 1;
                self.write_indent();
                self.out.push_str(&expr_inline);
                self.indent -= 1;
                self.out.push(';');
                self.newline();
                return;
            }
            self.out.push_str(" = ");
            self.out.push_str(&expr_inline);
            self.out.push(';');
            self.newline();
        }
    }

    fn fmt_expr_stmt(&mut self, expr: &Expr) {
        self.emit_comments(self.indent);
        match expr {
            Expr::Match { .. } => self.fmt_match_expr(expr),
            Expr::If { .. } => self.fmt_if_expr(expr),
            Expr::For { .. } => self.fmt_for_expr(expr),
            Expr::Pipe { .. } => self.fmt_pipe_expr(expr),
            _ => {
                self.write_indent();
                self.out
                    .push_str(&format_expr_inline_with_indent(expr, self.indent));
            }
        }
    }

    fn fmt_pipe_expr(&mut self, expr: &Expr) {
        let mut targets = Vec::new();
        let base = flatten_pipe(expr, &mut targets);
        self.write_indent();
        self.out
            .push_str(&format_expr_inline_with_indent(base, self.indent));
        if targets.is_empty() {
            return;
        }
        self.newline();
        self.indent += 1;
        for (idx, target) in targets.iter().enumerate() {
            self.write_indent();
            self.out.push_str("|> ");
            self.out
                .push_str(&format_pipe_target(target, self.indent, true));
            if idx + 1 < targets.len() {
                self.newline();
            }
        }
        self.indent -= 1;
    }

    fn fmt_if_expr(&mut self, expr: &Expr) {
        let Expr::If {
            cond,
            then_branch,
            else_branch,
        } = expr
        else {
            return;
        };
        self.write_indent();
        self.out.push_str("if ");
        self.out.push_str(&format_expr_inline_flat(cond));
        self.out.push_str(" {");
        self.newline();
        self.indent += 1;
        self.emit_comments(self.indent);
        self.fmt_expr_stmt(then_branch);
        self.newline();
        self.indent -= 1;
        self.write_indent();
        self.out.push_str("} else {");
        self.newline();
        self.indent += 1;
        self.emit_comments(self.indent);
        self.fmt_expr_stmt(else_branch);
        self.newline();
        self.indent -= 1;
        self.emit_comments(self.indent + 1);
        self.write_indent();
        self.out.push('}');
    }

    fn fmt_for_expr(&mut self, expr: &Expr) {
        let Expr::For {
            name,
            iter,
            guard,
            body,
        } = expr
        else {
            return;
        };
        self.write_indent();
        self.out.push_str("for ");
        self.out.push_str(name);
        self.out.push_str(" in ");
        self.out.push_str(&format_expr_inline_flat(iter));
        if let Some(guard) = guard {
            self.out.push_str(" if ");
            self.out.push_str(&format_expr_inline_flat(guard));
        }
        self.out.push_str(" {");
        self.newline();
        self.indent += 1;
        self.emit_comments(self.indent);
        self.fmt_expr_stmt(body);
        self.newline();
        self.indent -= 1;
        self.emit_comments(self.indent + 1);
        self.write_indent();
        self.out.push('}');
    }

    fn fmt_match_expr(&mut self, expr: &Expr) {
        let Expr::Match { scrutinee, arms } = expr else {
            return;
        };
        self.write_indent();
        self.out.push_str("match ");
        self.out.push_str(&format_expr_inline_flat(scrutinee));
        self.out.push_str(" {");
        self.newline();
        self.indent += 1;
        self.emit_comments(self.indent);
        for arm in arms {
            self.fmt_match_arm(arm);
        }
        self.emit_comments(self.indent);
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn fmt_match_arm(&mut self, arm: &MatchArm) {
        self.write_indent();
        let pattern_text = format_match_pattern(&arm.pattern);
        self.out.push_str(&pattern_text);
        self.out.push_str(" =>");
        let expr_inline = format_expr_inline_with_indent(&arm.body, self.indent);
        let line_len =
            self.indent * INDENT.len() + pattern_text.len() + " => ".len() + expr_inline.len() + 1;
        let needs_multiline = expr_inline.contains('\n') || line_len > MAX_INLINE_WIDTH;
        if expr_is_block(&arm.body) || needs_multiline {
            self.newline();
            self.indent += 1;
            self.emit_comments(self.indent);
            self.fmt_expr_stmt(&arm.body);
            self.indent -= 1;
            self.out.push(';');
            self.newline();
        } else {
            self.out.push(' ');
            self.out
                .push_str(&format_expr_inline_with_indent(&arm.body, self.indent));
            self.out.push(';');
            self.newline();
        }
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str(INDENT);
        }
    }

    fn newline(&mut self) {
        self.out.push('\n');
    }

    fn emit_comments(&mut self, indent: usize) {
        while let Some(comment) = self.comments.get(self.comment_idx) {
            if comment.indent != indent {
                break;
            }
            for _ in 0..indent {
                self.out.push_str(INDENT);
            }
            self.out.push_str("//");
            if !comment.text.is_empty() {
                self.out.push(' ');
                self.out.push_str(&comment.text);
            }
            self.out.push('\n');
            self.comment_idx += 1;
        }
    }

    fn emit_remaining_comments(&mut self) {
        while let Some(comment) = self.comments.get(self.comment_idx) {
            for _ in 0..comment.indent {
                self.out.push_str(INDENT);
            }
            self.out.push_str("//");
            if !comment.text.is_empty() {
                self.out.push(' ');
                self.out.push_str(&comment.text);
            }
            self.out.push('\n');
            self.comment_idx += 1;
        }
    }

    fn emit_item_comments(&mut self) {
        if self.item_index >= self.item_comments.len() {
            return;
        }
        let comments = &self.item_comments[self.item_index];
        for comment in comments {
            for _ in 0..comment.indent {
                self.out.push_str(INDENT);
            }
            self.out.push_str("//");
            if !comment.text.is_empty() {
                self.out.push(' ');
                self.out.push_str(&comment.text);
            }
            self.out.push('\n');
        }
    }

    fn has_item_comments(&self) -> bool {
        self.item_comments
            .get(self.item_index)
            .map(|comments| !comments.is_empty())
            .unwrap_or(false)
    }
}

fn expr_is_block(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Match { .. } | Expr::If { .. } | Expr::For { .. } | Expr::Pipe { .. }
    )
}

fn format_data_variant(variant: &DataVariant) -> String {
    if variant.fields.is_empty() {
        variant.name.clone()
    } else {
        format!("{} {{ {} }}", variant.name, variant.fields.join(", "))
    }
}

fn format_match_pattern(pattern: &MatchPattern) -> String {
    match pattern {
        MatchPattern::Wildcard => "_".to_string(),
        MatchPattern::Expr(expr) => format_expr_inline_flat(expr),
        MatchPattern::Compare { op, expr } => {
            format!("{} {}", binary_op_str(op), format_expr_inline_flat(expr))
        }
        MatchPattern::Variant { name, fields } => {
            if fields.is_empty() {
                return name.clone();
            }
            let mut out = String::new();
            out.push_str(name);
            out.push_str(" { ");
            for (idx, field) in fields.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format_field_pattern(field));
            }
            out.push_str(" }");
            out
        }
    }
}

fn format_field_pattern(pattern: &FieldPattern) -> String {
    match &pattern.bind {
        None => format!("{}: _", pattern.field),
        Some(bind) if bind == &pattern.field => pattern.field.clone(),
        Some(bind) => format!("{}: {}", pattern.field, bind),
    }
}

fn format_expr_inline_flat(expr: &Expr) -> String {
    format_expr_inline_prec(expr, 0, false, 0, false)
}

fn format_expr_inline_with_indent(expr: &Expr, indent: usize) -> String {
    format_expr_inline_prec(expr, 0, false, indent, true)
}

fn format_expr_inline_prec(
    expr: &Expr,
    parent_prec: u8,
    is_right: bool,
    indent: usize,
    allow_multiline: bool,
) -> String {
    let prec = expr_prec(expr);
    let mut out = match expr {
        Expr::Int(value) => value.to_string(),
        Expr::Bool(value) => value.to_string(),
        Expr::Str(value) => format!("\"{}\"", escape_string(value)),
        Expr::Bytes(value) => format!("b\"{}\"", escape_bytes(value)),
        Expr::List(items) => format_list(items, indent, allow_multiline),
        Expr::RangeList { start, end } => format!(
            "[{}..{}]",
            format_expr_inline_flat(start),
            format_expr_inline_flat(end)
        ),
        Expr::Map(entries) => format_map(entries, indent, allow_multiline),
        Expr::Var(name) => name.clone(),
        Expr::Construct { name, fields } => format_construct(name, fields, indent, allow_multiline),
        Expr::For {
            name,
            iter,
            guard,
            body,
        } => {
            let mut out = format!("for {} in {}", name, format_expr_inline_flat(iter));
            if let Some(guard) = guard {
                out.push_str(" if ");
                out.push_str(&format_expr_inline_flat(guard));
            }
            out.push_str(" { ");
            out.push_str(&format_expr_inline_flat(body));
            out.push_str(" }");
            out
        }
        Expr::Unary { op, expr } => {
            let inner = format_expr_inline_prec(expr, PREC_UNARY, false, indent, allow_multiline);
            format!("{}{}", unary_op_str(op), inner)
        }
        Expr::Binary { op, left, right } => {
            let left = format_expr_inline_prec(left, prec, false, indent, allow_multiline);
            let right = format_expr_inline_prec(right, prec, true, indent, allow_multiline);
            format!("{left} {} {right}", binary_op_str(op))
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => format!(
            "if {} {{ {} }} else {{ {} }}",
            format_expr_inline_flat(cond),
            format_expr_inline_flat(then_branch),
            format_expr_inline_flat(else_branch)
        ),
        Expr::Call { callee, args } => format_call(callee, args, indent, allow_multiline),
        Expr::Pipe { .. } => format_pipe_inline(expr, indent, allow_multiline),
        Expr::Match { scrutinee, arms } => format_match_inline(scrutinee, arms),
    };

    if prec < parent_prec
        || (prec == parent_prec
            && is_right
            && matches!(expr, Expr::Binary { .. } | Expr::Pipe { .. }))
    {
        out = format!("({out})");
    }
    out
}

fn format_list(items: &[Expr], indent: usize, allow_multiline: bool) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    if !allow_multiline || !list_needs_multiline(items) {
        return format!("[{}]", join_exprs(items, indent, false));
    }
    let mut out = String::new();
    out.push_str("[\n");
    let item_indent = indent + 1;
    for (idx, item) in items.iter().enumerate() {
        out.push_str(&indent_str(item_indent));
        out.push_str(&format_expr_inline_prec(item, 0, false, item_indent, true));
        if idx + 1 < items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&indent_str(indent));
    out.push(']');
    out
}

fn format_map(entries: &[(Expr, Expr)], indent: usize, allow_multiline: bool) -> String {
    if entries.is_empty() {
        return "#{}".to_string();
    }
    let mut out = String::new();
    out.push_str("#{ ");
    for (idx, (key, value)) in entries.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&format_expr_inline_prec(
            key,
            0,
            false,
            indent,
            allow_multiline,
        ));
        out.push_str(": ");
        out.push_str(&format_expr_inline_prec(
            value,
            0,
            false,
            indent,
            allow_multiline,
        ));
    }
    out.push_str(" }");
    out
}

fn format_construct(
    name: &str,
    fields: &[(String, Expr)],
    indent: usize,
    allow_multiline: bool,
) -> String {
    if fields.is_empty() {
        return format!("{name} {{}}");
    }
    if allow_multiline && construct_needs_multiline(name, fields, indent) {
        return format_construct_multiline(name, fields, indent);
    }
    format_construct_inline(name, fields, indent, allow_multiline)
}

fn format_construct_inline(
    name: &str,
    fields: &[(String, Expr)],
    indent: usize,
    allow_multiline: bool,
) -> String {
    let mut out = String::new();
    out.push_str(name);
    out.push_str(" { ");
    for (idx, (field, expr)) in fields.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        if matches!(expr, Expr::Var(var) if var == field) {
            out.push_str(field);
        } else {
            out.push_str(field);
            out.push_str(": ");
            out.push_str(&format_expr_inline_prec(
                expr,
                0,
                false,
                indent,
                allow_multiline,
            ));
        }
    }
    out.push_str(" }");
    out
}

fn format_construct_multiline(name: &str, fields: &[(String, Expr)], indent: usize) -> String {
    let mut out = String::new();
    out.push_str(name);
    out.push_str(" {\n");
    let field_indent = indent + 1;
    for (idx, (field, expr)) in fields.iter().enumerate() {
        let is_last = idx + 1 == fields.len();
        out.push_str(&indent_str(field_indent));
        if matches!(expr, Expr::Var(var) if var == field) {
            out.push_str(field);
        } else {
            out.push_str(field);
            out.push_str(": ");
            let rendered = format_expr_inline_prec(expr, 0, false, field_indent, true);
            out.push_str(&rendered);
        }
        if !is_last {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&indent_str(indent));
    out.push('}');
    out
}

fn format_pipe_inline(expr: &Expr, indent: usize, allow_multiline: bool) -> String {
    let mut targets = Vec::new();
    let base = flatten_pipe(expr, &mut targets);
    let mut out = String::new();
    out.push_str(&format_expr_inline_prec(
        base,
        PREC_PIPE,
        false,
        indent,
        allow_multiline,
    ));
    for target in targets {
        out.push_str(" |> ");
        out.push_str(&format_pipe_target(target, indent, allow_multiline));
    }
    out
}

fn format_pipe_target(target: &PipeTarget, indent: usize, allow_multiline: bool) -> String {
    match target {
        PipeTarget::Ident(name) => name.clone(),
        PipeTarget::Call { name, args } => {
            format!("{}({})", name, join_exprs(args, indent, allow_multiline))
        }
    }
}

fn format_match_inline(scrutinee: &Expr, arms: &[MatchArm]) -> String {
    let mut out = String::new();
    out.push_str("match ");
    out.push_str(&format_expr_inline_flat(scrutinee));
    out.push_str(" { ");
    for (idx, arm) in arms.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(&format_match_pattern(&arm.pattern));
        out.push_str(" => ");
        out.push_str(&format_expr_inline_flat(&arm.body));
        out.push(';');
    }
    out.push_str(" }");
    out
}

fn flatten_pipe<'a>(expr: &'a Expr, targets: &mut Vec<&'a PipeTarget>) -> &'a Expr {
    match expr {
        Expr::Pipe { input, target } => {
            let base = flatten_pipe(input, targets);
            targets.push(target);
            base
        }
        _ => expr,
    }
}

fn join_names(names: &[String]) -> String {
    names.join(", ")
}

fn join_exprs(exprs: &[Expr], indent: usize, allow_multiline: bool) -> String {
    exprs
        .iter()
        .map(|expr| format_expr_inline_prec(expr, 0, false, indent, allow_multiline))
        .collect::<Vec<_>>()
        .join(", ")
}

fn unary_op_str(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}

fn binary_op_str(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::Neq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Lte => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Gte => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

const PREC_PIPE: u8 = 1;
const PREC_OR: u8 = 2;
const PREC_AND: u8 = 3;
const PREC_EQ: u8 = 4;
const PREC_CMP: u8 = 5;
const PREC_ADD: u8 = 6;
const PREC_MUL: u8 = 7;
const PREC_UNARY: u8 = 8;
const PREC_PRIMARY: u8 = 9;

fn expr_prec(expr: &Expr) -> u8 {
    match expr {
        Expr::Pipe { .. } => PREC_PIPE,
        Expr::Binary { op, .. } => match *op {
            BinaryOp::Or => PREC_OR,
            BinaryOp::And => PREC_AND,
            BinaryOp::Eq | BinaryOp::Neq => PREC_EQ,
            BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => PREC_CMP,
            BinaryOp::Add | BinaryOp::Sub => PREC_ADD,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => PREC_MUL,
        },
        Expr::Unary { .. } => PREC_UNARY,
        _ => PREC_PRIMARY,
    }
}

fn list_needs_multiline(items: &[Expr]) -> bool {
    if items.len() > MAX_LIST_INLINE_ITEMS {
        return true;
    }
    let inline = format_list_inline(items);
    inline.len() > MAX_LIST_INLINE_WIDTH
}

fn format_list_inline(items: &[Expr]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    format!("[{}]", join_exprs_flat(items))
}

fn join_exprs_flat(exprs: &[Expr]) -> String {
    exprs
        .iter()
        .map(format_expr_inline_flat)
        .collect::<Vec<_>>()
        .join(", ")
}

fn indent_str(level: usize) -> String {
    INDENT.repeat(level)
}

fn format_call(callee: &str, args: &[Expr], indent: usize, allow_multiline: bool) -> String {
    if args.is_empty() {
        return format!("{callee}()");
    }
    if !allow_multiline || !call_needs_multiline(callee, args, indent) {
        return format!("{callee}({})", join_exprs(args, indent, false));
    }
    let mut out = String::new();
    out.push_str(callee);
    out.push_str("(\n");
    let arg_indent = indent + 1;
    for (idx, arg) in args.iter().enumerate() {
        out.push_str(&indent_str(arg_indent));
        out.push_str(&format_expr_inline_prec(arg, 0, false, arg_indent, true));
        if idx + 1 < args.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&indent_str(indent));
    out.push(')');
    out
}

fn call_needs_multiline(callee: &str, args: &[Expr], indent: usize) -> bool {
    if args.len() > MAX_CALL_INLINE_ITEMS {
        return true;
    }
    let inline = format!("{callee}({})", join_exprs(args, indent, false));
    inline.len() > MAX_CALL_INLINE_WIDTH
}

fn construct_needs_multiline(name: &str, fields: &[(String, Expr)], indent: usize) -> bool {
    if fields.len() > 1 {
        for (_, expr) in fields {
            if format_expr_inline_prec(expr, 0, false, indent, true).contains('\n') {
                return true;
            }
            if !expr_is_simple_inline(expr) {
                return true;
            }
        }
    }
    let inline = format_construct_inline(name, fields, indent, false);
    inline.len() > MAX_CONSTRUCT_INLINE_WIDTH
}

fn expr_is_simple_inline(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Int(_) | Expr::Bool(_) | Expr::Str(_) | Expr::Bytes(_) | Expr::Var(_)
    )
}

fn item_kind(item: &Item) -> ItemKind {
    match item {
        Item::Data(_) => ItemKind::Data,
        Item::ExternFn(_) => ItemKind::ExternFn,
        Item::Fn(_) => ItemKind::Fn,
        Item::Let(_) => ItemKind::Let,
    }
}

fn item_is_multiline(item: &Item, indent: usize) -> bool {
    match item {
        Item::Data(_) => false,
        Item::ExternFn(def) => def.explain.contains('\n'),
        Item::Fn(def) => {
            if expr_is_block(&def.body) {
                true
            } else {
                format_expr_inline_with_indent(&def.body, indent).contains('\n')
            }
        }
        Item::Let(def) => {
            if expr_is_block(&def.expr) {
                true
            } else {
                format_expr_inline_with_indent(&def.expr, indent).contains('\n')
            }
        }
    }
}

fn collect_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        if let Some((indent, text)) = scan_line_comment(line) {
            comments.push(Comment {
                indent,
                text,
                line: idx,
            });
        }
    }
    comments
}

fn scan_line_comment(line: &str) -> Option<(usize, String)> {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_bytes = false;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            if ch == b'\\' {
                i = (i + 2).min(bytes.len());
                continue;
            }
            if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if in_bytes {
            if ch == b'\\' {
                i = (i + 2).min(bytes.len());
                continue;
            }
            if ch == b'"' {
                in_bytes = false;
            }
            i += 1;
            continue;
        }
        if ch == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
            in_bytes = true;
            i += 2;
            continue;
        }
        if ch == b'"' {
            in_string = true;
            i += 1;
            continue;
        }
        if ch == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            let text = line[i + 2..].trim_end().trim_start().to_string();
            let indent = leading_indent_level(line);
            return Some((indent, text));
        }
        i += 1;
    }
    None
}

fn leading_indent_level(line: &str) -> usize {
    let mut columns = 0usize;
    for ch in line.chars() {
        match ch {
            ' ' => columns += 1,
            '\t' => columns += 2,
            _ => break,
        }
    }
    columns / INDENT.len()
}

fn distribute_comments(
    comments: Vec<Comment>,
    item_lines: &[usize],
    item_count: usize,
) -> (Vec<Comment>, Vec<Vec<Comment>>) {
    let mut inline = Vec::new();
    let mut buckets = vec![Vec::new(); item_count + 1];
    if item_lines.len() != item_count {
        inline.extend(comments);
        return (inline, buckets);
    }
    for comment in comments {
        if comment.indent == 0 {
            let mut placed = false;
            for (idx, line) in item_lines.iter().enumerate() {
                if comment.line <= *line {
                    buckets[idx].push(comment.clone());
                    placed = true;
                    break;
                }
            }
            if !placed {
                buckets[item_count].push(comment);
            }
        } else {
            inline.push(comment);
        }
    }
    (inline, buckets)
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (idx, ch) in source.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn byte_to_line(pos: usize, line_starts: &[usize]) -> usize {
    match line_starts.binary_search(&pos) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    }
}

fn scan_top_level_item_lines(tokens: &[crate::lexer::Token], line_starts: &[usize]) -> Vec<usize> {
    let mut lines = Vec::new();
    let mut depth_paren = 0i32;
    let mut depth_brace = 0i32;
    let mut depth_bracket = 0i32;
    let mut in_item = false;

    for token in tokens {
        let is_top = depth_paren == 0 && depth_brace == 0 && depth_bracket == 0;
        if !in_item
            && is_top
            && matches!(
                token.kind,
                TokenKind::KwData | TokenKind::KwExtern | TokenKind::KwFn | TokenKind::KwLet
            )
        {
            lines.push(byte_to_line(token.position, line_starts));
            in_item = true;
        }

        match token.kind {
            TokenKind::LParen => depth_paren += 1,
            TokenKind::RParen => depth_paren = (depth_paren - 1).max(0),
            TokenKind::LBrace => depth_brace += 1,
            TokenKind::RBrace => depth_brace = (depth_brace - 1).max(0),
            TokenKind::LBracket => depth_bracket += 1,
            TokenKind::RBracket => depth_bracket = (depth_bracket - 1).max(0),
            _ => {}
        }

        if in_item
            && depth_paren == 0
            && depth_brace == 0
            && depth_bracket == 0
            && matches!(token.kind, TokenKind::Semicolon)
        {
            in_item = false;
        }
    }

    lines
}

fn escape_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ if ch.is_ascii() && !ch.is_control() => out.push(ch),
            _ => {
                let code = ch as u32;
                if code <= 0xFF {
                    out.push_str(&format!("\\x{code:02X}"));
                } else {
                    let mut buf = [0u8; 4];
                    for byte in ch.encode_utf8(&mut buf).as_bytes() {
                        out.push_str(&format!("\\x{byte:02X}"));
                    }
                }
            }
        }
    }
    out
}

fn escape_bytes(value: &[u8]) -> String {
    let mut out = String::new();
    for &byte in value {
        match byte {
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b'\0' => out.push_str("\\0"),
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            0x20..=0x7E => out.push(byte as char),
            _ => out.push_str(&format!("\\x{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{format_program, format_source};
    use crate::lexer::Lexer;
    use crate::parser::parse_program;
    use crate::validate::validate_program;

    #[test]
    fn formats_pipe_and_match() {
        let source = r#"
            data Tree = Empty | Node { value, left, right };
            fn topk(t, k) = t |> to_desc_list |> take_k(k);
            let xs = match t { Empty => []; _ => [1, 2] };
            Empty |> insert(1) |> insert(2)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let formatted = format_program(&program);
        assert!(formatted.contains("fn topk(t, k) ="));
        assert!(formatted.contains("|> to_desc_list"));
        assert!(formatted.contains("match t {"));
    }

    #[test]
    fn formats_explain_block() {
        let source = r#"
            extern fn add(x, y) explain {
              Adds.
              More.
            };
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let formatted = format_program(&program);
        assert!(formatted.contains("extern fn add(x, y) explain {"));
        assert!(formatted.contains("Adds."));
    }

    #[test]
    fn preserves_line_comments() {
        let source = r#"
            // top level
            let x = 1; // trailing
            fn f(a) = if a { 1 } else { 2 };
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let formatted = format_source(source, &program);
        assert!(formatted.contains("// top level"));
        assert!(formatted.contains("// trailing"));
    }

    #[test]
    fn formats_for_expr() {
        let source = r#"
            fn inc(x) = x + 1;
            for x in [1, 2] { inc(x) }
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let formatted = format_program(&program);
        assert!(formatted.contains("for x in [1, 2] {"));
        assert!(formatted.contains("inc(x)"));
    }

    #[test]
    fn formats_for_with_guard() {
        let source = r#"
            for x in [1, 2, 3] if x > 1 { x }
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let formatted = format_program(&program);
        assert!(formatted.contains("for x in [1, 2, 3] if x > 1 {"));
    }

    #[test]
    fn formats_range_list() {
        let source = r#"
            [1..10]
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let formatted = format_program(&program);
        assert!(formatted.contains("[1..10]"));
    }

    #[test]
    fn avoids_trailing_commas_in_multiline() {
        let source = r#"
            data Foo = Foo { a, b };
            extern fn build(a, b, c, d, e) explain { ok. };
            let xs = [1, 2, 3, 4, 5];
            let ys = build(1, 2, 3, 4, 5);
            let f = Foo { a: [1, 2, 3, 4, 5], b: build(1, 2, 3, 4, 5) };
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let formatted = format_program(&program);
        assert!(!formatted.contains(",\n]"));
        assert!(!formatted.contains(",\n)"));
        assert!(!formatted.contains(",\n}"));
    }
}
