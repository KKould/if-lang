use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use crate::ast::core;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    List(Vec<Value>),
    Map(BTreeMap<ValueKey, Value>),
    Variant {
        name: String,
        fields: BTreeMap<String, Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueKey {
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalError {
    pub message: String,
}

impl EvalError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub type BuiltinFn = Arc<dyn Fn(&[Value]) -> Result<Value, EvalError> + Send + Sync>;

pub fn eval_program(program: &core::Program) -> Result<Option<Value>, EvalError> {
    eval_program_with_builtins(program, &HashMap::new())
}

pub fn eval_program_with_builtins(
    program: &core::Program,
    builtins: &HashMap<String, BuiltinFn>,
) -> Result<Option<Value>, EvalError> {
    let mut ctx = EvalContext::default();
    ctx.builtins = builtins.clone();
    for item in &program.items {
        match item {
            core::Item::Fn(def) => {
                ctx.funcs.insert(def.name.clone(), def.clone());
            }
            core::Item::ExternFn(def) => {
                ctx.externs.insert(def.name.clone());
            }
            core::Item::Data(def) => {
                for variant in &def.variants {
                    ctx.constructors
                        .insert(variant.name.clone(), variant.fields.clone());
                }
            }
            _ => {}
        }
    }

    for item in &program.items {
        if let core::Item::Let(def) = item {
            let value = eval_expr(&def.expr, &ctx, &HashMap::new())?;
            ctx.globals.insert(def.name.clone(), value);
        }
    }

    if let Some(expr) = &program.expr {
        let value = eval_expr(expr, &ctx, &HashMap::new())?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

#[derive(Default)]
struct EvalContext {
    globals: HashMap<String, Value>,
    funcs: HashMap<String, core::FnDef>,
    externs: HashSet<String>,
    builtins: HashMap<String, BuiltinFn>,
    constructors: HashMap<String, Vec<String>>,
}

fn eval_expr(
    expr: &core::Expr,
    ctx: &EvalContext,
    locals: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    match expr {
        core::Expr::Int(value) => Ok(Value::Int(*value)),
        core::Expr::Bool(value) => Ok(Value::Bool(*value)),
        core::Expr::List(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(eval_expr(item, ctx, locals)?);
            }
            Ok(Value::List(values))
        }
        core::Expr::Map(entries) => {
            let mut map = BTreeMap::new();
            for (key_expr, value_expr) in entries {
                let key_value = eval_expr(key_expr, ctx, locals)?;
                let value = eval_expr(value_expr, ctx, locals)?;
                let key = value_to_key(&key_value)?;
                map.insert(key, value);
            }
            Ok(Value::Map(map))
        }
        core::Expr::Var(name) => lookup_var(name, ctx, locals),
        core::Expr::Construct { name, fields } => {
            let mut evaluated = BTreeMap::new();
            for (field, expr) in fields {
                let value = eval_expr(expr, ctx, locals)?;
                evaluated.insert(field.clone(), value);
            }
            if let Some(expected_fields) = ctx.constructors.get(name) {
                let mut missing = Vec::new();
                for field in expected_fields {
                    if !evaluated.contains_key(field) {
                        missing.push(field.clone());
                    }
                }
                if !missing.is_empty() {
                    return Err(EvalError::new(format!(
                        "constructor '{}' missing fields {:?}",
                        name, missing
                    )));
                }
                let mut extra = Vec::new();
                for field in evaluated.keys() {
                    if !expected_fields.contains(field) {
                        extra.push(field.clone());
                    }
                }
                if !extra.is_empty() {
                    return Err(EvalError::new(format!(
                        "constructor '{}' has unknown fields {:?}",
                        name, extra
                    )));
                }
            } else {
                return Err(EvalError::new(format!(
                    "unknown constructor '{}'",
                    name
                )));
            }
            Ok(Value::Variant {
                name: name.clone(),
                fields: evaluated,
            })
        }
        core::Expr::Unary { op, expr } => {
            let value = eval_expr(expr, ctx, locals)?;
            eval_unary(op, value)
        }
        core::Expr::Binary { op, left, right } => {
            if matches!(op, crate::ast::BinaryOp::And) {
                let left_value = eval_expr(left, ctx, locals)?;
                let left_bool = expect_bool(left_value)?;
                if !left_bool {
                    return Ok(Value::Bool(false));
                }
                let right_value = eval_expr(right, ctx, locals)?;
                let right_bool = expect_bool(right_value)?;
                return Ok(Value::Bool(right_bool));
            }
            if matches!(op, crate::ast::BinaryOp::Or) {
                let left_value = eval_expr(left, ctx, locals)?;
                let left_bool = expect_bool(left_value)?;
                if left_bool {
                    return Ok(Value::Bool(true));
                }
                let right_value = eval_expr(right, ctx, locals)?;
                let right_bool = expect_bool(right_value)?;
                return Ok(Value::Bool(right_bool));
            }
            let left_value = eval_expr(left, ctx, locals)?;
            let right_value = eval_expr(right, ctx, locals)?;
            eval_binary(op, left_value, right_value)
        }
        core::Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_value = eval_expr(cond, ctx, locals)?;
            let is_true = expect_bool(cond_value)?;
            if is_true {
                eval_expr(then_branch, ctx, locals)
            } else {
                eval_expr(else_branch, ctx, locals)
            }
        }
        core::Expr::Call { callee, args } => {
            let mut evaluated_args = Vec::with_capacity(args.len());
            for arg in args {
                evaluated_args.push(eval_expr(arg, ctx, locals)?);
            }
            eval_call(callee, evaluated_args, ctx)
        }
        core::Expr::Match { scrutinee, arms } => {
            let value = eval_expr(scrutinee, ctx, locals)?;
            for arm in arms {
                if let Some(bindings) = match_pattern(&arm.pattern, &value, ctx, locals)? {
                    let mut new_locals = locals.clone();
                    for (name, value) in bindings {
                        new_locals.insert(name, value);
                    }
                    return eval_expr(&arm.body, ctx, &new_locals);
                }
            }
            Err(EvalError::new("no match arm matched"))
        }
    }
}

fn lookup_var(
    name: &str,
    ctx: &EvalContext,
    locals: &HashMap<String, Value>,
) -> Result<Value, EvalError> {
    if let Some(value) = locals.get(name) {
        return Ok(value.clone());
    }
    if let Some(value) = ctx.globals.get(name) {
        return Ok(value.clone());
    }
    if let Some(fields) = ctx.constructors.get(name) {
        if fields.is_empty() {
            return Ok(Value::Variant {
                name: name.to_string(),
                fields: BTreeMap::new(),
            });
        }
    }
    Err(EvalError::new(format!(
        "unknown variable '{}'",
        name
    )))
}

fn eval_unary(op: &crate::ast::UnaryOp, value: Value) -> Result<Value, EvalError> {
    match op {
        crate::ast::UnaryOp::Neg => {
            let v = expect_int(value)?;
            Ok(Value::Int(-v))
        }
        crate::ast::UnaryOp::Not => {
            let v = expect_bool(value)?;
            Ok(Value::Bool(!v))
        }
    }
}

fn eval_binary(
    op: &crate::ast::BinaryOp,
    left: Value,
    right: Value,
) -> Result<Value, EvalError> {
    use crate::ast::BinaryOp::*;
    match op {
        Add => Ok(Value::Int(expect_int(left)? + expect_int(right)?)),
        Sub => Ok(Value::Int(expect_int(left)? - expect_int(right)?)),
        Mul => Ok(Value::Int(expect_int(left)? * expect_int(right)?)),
        Div => {
            let denom = expect_int(right)?;
            if denom == 0 {
                return Err(EvalError::new("division by zero"));
            }
            Ok(Value::Int(expect_int(left)? / denom))
        }
        Mod => {
            let denom = expect_int(right)?;
            if denom == 0 {
                return Err(EvalError::new("mod by zero"));
            }
            Ok(Value::Int(expect_int(left)? % denom))
        }
        Eq => Ok(Value::Bool(equal_values(left, right)?)),
        Neq => Ok(Value::Bool(!equal_values(left, right)?)),
        Lt => Ok(Value::Bool(expect_int(left)? < expect_int(right)?)),
        Lte => Ok(Value::Bool(expect_int(left)? <= expect_int(right)?)),
        Gt => Ok(Value::Bool(expect_int(left)? > expect_int(right)?)),
        Gte => Ok(Value::Bool(expect_int(left)? >= expect_int(right)?)),
        And | Or => Err(EvalError::new("internal: short-circuit handled")),
    }
}

fn equal_values(left: Value, right: Value) -> Result<bool, EvalError> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(a == b),
        (Value::Bool(a), Value::Bool(b)) => Ok(a == b),
        (
            Value::Variant {
                name: a_name,
                fields: a_fields,
            },
            Value::Variant {
                name: b_name,
                fields: b_fields,
            },
        ) => Ok(a_name == b_name && a_fields == b_fields),
        _ => Err(EvalError::new("== only supports Int/Bool/Variant")),
    }
}

fn eval_call(
    callee: &str,
    args: Vec<Value>,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    if let Some(func) = ctx.funcs.get(callee) {
        return eval_user_fn(func, args, ctx);
    }
    if let Some(builtin) = ctx.builtins.get(callee) {
        if !ctx.externs.contains(callee) {
            return Err(EvalError::new(format!(
                "builtin '{}' must be declared extern",
                callee
            )));
        }
        return builtin(&args);
    }
    if ctx.externs.contains(callee) {
        return Err(EvalError::new(format!(
            "extern function '{}' not registered",
            callee
        )));
    }
    Err(EvalError::new(format!("unknown function '{}'", callee)))
}

fn eval_user_fn(
    def: &core::FnDef,
    args: Vec<Value>,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    if def.params.len() != args.len() {
        return Err(EvalError::new(format!(
            "function '{}' expects {} args, got {}",
            def.name,
            def.params.len(),
            args.len()
        )));
    }
    let mut locals = HashMap::new();
    for (name, value) in def.params.iter().zip(args.into_iter()) {
        locals.insert(name.clone(), value);
    }
    eval_expr(&def.body, ctx, &locals)
}

fn match_pattern(
    pattern: &core::MatchPattern,
    value: &Value,
    ctx: &EvalContext,
    locals: &HashMap<String, Value>,
) -> Result<Option<HashMap<String, Value>>, EvalError> {
    match pattern {
        core::MatchPattern::Wildcard => Ok(Some(HashMap::new())),
        core::MatchPattern::Expr(expr) => {
            let expected = eval_expr(expr, ctx, locals)?;
            if equal_values(value.clone(), expected)? {
                Ok(Some(HashMap::new()))
            } else {
                Ok(None)
            }
        }
        core::MatchPattern::Compare { op, expr } => {
            let rhs = eval_expr(expr, ctx, locals)?;
            let left = match value {
                Value::Int(v) => *v,
                _ => return Err(EvalError::new("compare pattern expects Int")),
            };
            let right = expect_int(rhs)?;
            let matched = match op {
                crate::ast::BinaryOp::Lt => left < right,
                crate::ast::BinaryOp::Lte => left <= right,
                crate::ast::BinaryOp::Gt => left > right,
                crate::ast::BinaryOp::Gte => left >= right,
                _ => return Err(EvalError::new("invalid compare operator")),
            };
            if matched {
                Ok(Some(HashMap::new()))
            } else {
                Ok(None)
            }
        }
        core::MatchPattern::Variant { name, fields } => {
            let (variant_name, variant_fields) = match value {
                Value::Variant { name, fields } => (name, fields),
                _ => return Ok(None),
            };
            if variant_name != name {
                return Ok(None);
            }
            let mut bindings = HashMap::new();
            for field in fields {
                if let Some(value) = variant_fields.get(&field.field) {
                    if let Some(bind) = &field.bind {
                        bindings.insert(bind.clone(), value.clone());
                    }
                } else {
                    return Ok(None);
                }
            }
            Ok(Some(bindings))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{eval_program, eval_program_with_builtins, Value};
    use crate::lexer::Lexer;
    use crate::lower::lower_program;
    use crate::parser::parse_program;
    use crate::validate::validate_program;
    use std::collections::HashMap;

    #[test]
    fn evals_match_with_variant_and_fields() {
        let source = r#"
            data Tree = Empty | Node { value };
            fn value_of(t) =
              match t {
                Node { value } => value;
                _ => 0;
              };
            let t = Node { value: 7 };
            value_of(t)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let core = lower_program(program);
        let value = eval_program(&core).expect("eval").expect("value");
        assert_eq!(value, Value::Int(7));
    }

    #[test]
    fn evals_match_with_compare_pattern() {
        let source = r#"
            fn grade(x) =
              match x {
                >= 3 => 1;
                _ => 0;
              };
            grade(5)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let core = lower_program(program);
        let value = eval_program(&core).expect("eval").expect("value");
        assert_eq!(value, Value::Int(1));
    }

    #[test]
    fn eval_errors_on_missing_constructor_fields() {
        let source = r#"
            data Tree = Empty | Node { value, left };
            Node { value: 1 }
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let core = lower_program(program);
        let err = eval_program(&core).expect_err("should fail");
        assert!(err.message.contains("missing fields"));
    }

    #[test]
    fn eval_errors_on_unregistered_extern() {
        let source = r#"
            extern fn foo(x);
            foo(1)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let core = lower_program(program);
        let err = eval_program_with_builtins(&core, &HashMap::new())
            .expect_err("should fail");
        assert!(err.message.contains("extern function"));
    }
}

fn expect_int(value: Value) -> Result<i64, EvalError> {
    match value {
        Value::Int(v) => Ok(v),
        _ => Err(EvalError::new("expected Int")),
    }
}

fn expect_bool(value: Value) -> Result<bool, EvalError> {
    match value {
        Value::Bool(v) => Ok(v),
        _ => Err(EvalError::new("expected Bool")),
    }
}

fn value_to_key(value: &Value) -> Result<ValueKey, EvalError> {
    match value {
        Value::Int(v) => Ok(ValueKey::Int(*v)),
        Value::Bool(v) => Ok(ValueKey::Bool(*v)),
        _ => Err(EvalError::new("map keys must be Int or Bool")),
    }
}
