use std::collections::HashMap;

use crate::ast::surface;
use crate::error::Error;

pub fn validate_program(program: &surface::Program) -> Result<(), Error> {
    for item in &program.items {
        match item {
            surface::Item::Fn(def) => {
                validate_expr(&def.body)?;
                validate_fn_param_order(def)?;
            }
            surface::Item::Let(def) => {
                validate_expr(&def.expr)?;
            }
            _ => {}
        }
    }
    if let Some(expr) = &program.expr {
        validate_expr(expr)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_program;
    use crate::lexer::Lexer;
    use crate::parser::parse_program;

    #[test]
    fn validates_param_order_violation() {
        let source = r#"
            fn f(a, b) = b + a;
            f(1, 2)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        let err = validate_program(&program).expect_err("should fail");
        assert!(err.message.contains("uses parameter"));
    }

    #[test]
    fn validates_match_wildcard_last() {
        let source = r#"
            fn f(x) = match x { _ => 1; 2 => 3; };
            f(0)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        let err = validate_program(&program).expect_err("should fail");
        assert!(err.message.contains("wildcard"));
    }
}
fn validate_fn_param_order(def: &surface::FnDef) -> Result<(), Error> {
    let mut param_index = HashMap::new();
    for (idx, name) in def.params.iter().enumerate() {
        param_index.insert(name.as_str(), idx);
    }

    let mut seen = HashMap::new();
    let mut first_seen: Vec<&str> = Vec::new();
    collect_first_seen_params(&def.body, &param_index, &mut seen, &mut first_seen);

    let mut last_idx: Option<usize> = None;
    let mut last_param: Option<&str> = None;
    for name in first_seen {
        let idx = *param_index
            .get(name)
            .expect("param must exist in map");
        if let Some(prev_idx) = last_idx {
            if idx < prev_idx {
                let prev = last_param.unwrap_or("<unknown>");
                return Err(Error::new(
                    format!(
                        "function '{}' uses parameter '{}' before earlier parameter '{}'",
                        def.name, name, prev
                    ),
                    0,
                ));
            }
        }
        last_idx = Some(idx);
        last_param = Some(name);
    }

    Ok(())
}

fn collect_first_seen_params<'a>(
    expr: &'a surface::Expr,
    param_index: &HashMap<&'a str, usize>,
    seen: &mut HashMap<&'a str, bool>,
    first_seen: &mut Vec<&'a str>,
) {
    match expr {
        surface::Expr::Int(_) | surface::Expr::Bool(_) | surface::Expr::Str(_) | surface::Expr::Bytes(_) => {}
        surface::Expr::Var(name) => {
            if param_index.contains_key(name.as_str()) && !seen.contains_key(name.as_str()) {
                seen.insert(name.as_str(), true);
                first_seen.push(name.as_str());
            }
        }
        surface::Expr::List(items) => {
            for item in items {
                collect_first_seen_params(item, param_index, seen, first_seen);
            }
        }
        surface::Expr::Map(entries) => {
            for (key, value) in entries {
                collect_first_seen_params(key, param_index, seen, first_seen);
                collect_first_seen_params(value, param_index, seen, first_seen);
            }
        }
        surface::Expr::Construct { fields, .. } => {
            for (_, expr) in fields {
                collect_first_seen_params(expr, param_index, seen, first_seen);
            }
        }
        surface::Expr::Unary { expr, .. } => {
            collect_first_seen_params(expr, param_index, seen, first_seen);
        }
        surface::Expr::Binary { left, right, .. } => {
            collect_first_seen_params(left, param_index, seen, first_seen);
            collect_first_seen_params(right, param_index, seen, first_seen);
        }
        surface::Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_first_seen_params(cond, param_index, seen, first_seen);
            collect_first_seen_params(then_branch, param_index, seen, first_seen);
            collect_first_seen_params(else_branch, param_index, seen, first_seen);
        }
        surface::Expr::Call { args, .. } => {
            for arg in args {
                collect_first_seen_params(arg, param_index, seen, first_seen);
            }
        }
        surface::Expr::Pipe { input, target } => {
            collect_first_seen_params(input, param_index, seen, first_seen);
            match target {
                surface::PipeTarget::Ident(_) => {}
                surface::PipeTarget::Call { args, .. } => {
                    for arg in args {
                        collect_first_seen_params(arg, param_index, seen, first_seen);
                    }
                }
            }
        }
        surface::Expr::Match { scrutinee, arms } => {
            collect_first_seen_params(scrutinee, param_index, seen, first_seen);
            for arm in arms {
                match &arm.pattern {
                    surface::MatchPattern::Wildcard => {}
                    surface::MatchPattern::Expr(expr) => {
                        collect_first_seen_params(expr, param_index, seen, first_seen);
                    }
                    surface::MatchPattern::Compare { expr, .. } => {
                        collect_first_seen_params(expr, param_index, seen, first_seen);
                    }
                    surface::MatchPattern::Variant { .. } => {}
                }
                collect_first_seen_params(&arm.body, param_index, seen, first_seen);
            }
        }
    }
}

fn validate_expr(expr: &surface::Expr) -> Result<(), Error> {
    match expr {
        surface::Expr::Int(_)
        | surface::Expr::Bool(_)
        | surface::Expr::Str(_)
        | surface::Expr::Bytes(_)
        | surface::Expr::Var(_) => Ok(()),
        surface::Expr::List(items) => {
            for item in items {
                validate_expr(item)?;
            }
            Ok(())
        }
        surface::Expr::Map(entries) => {
            for (key, value) in entries {
                validate_expr(key)?;
                validate_expr(value)?;
            }
            Ok(())
        }
        surface::Expr::Construct { fields, .. } => {
            for (_, expr) in fields {
                validate_expr(expr)?;
            }
            Ok(())
        }
        surface::Expr::Unary { expr, .. } => validate_expr(expr),
        surface::Expr::Binary { left, right, .. } => {
            validate_expr(left)?;
            validate_expr(right)?;
            Ok(())
        }
        surface::Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            validate_expr(cond)?;
            validate_expr(then_branch)?;
            validate_expr(else_branch)?;
            Ok(())
        }
        surface::Expr::Call { args, .. } => {
            for arg in args {
                validate_expr(arg)?;
            }
            Ok(())
        }
        surface::Expr::Pipe { input, target } => {
            validate_expr(input)?;
            if let surface::PipeTarget::Call { args, .. } = target {
                for arg in args {
                    validate_expr(arg)?;
                }
            }
            Ok(())
        }
        surface::Expr::Match { scrutinee, arms } => {
            validate_expr(scrutinee)?;
            if arms.is_empty() {
                return Err(Error::new("match requires at least one arm", 0));
            }
            let mut wildcard_index: Option<usize> = None;
            for (idx, arm) in arms.iter().enumerate() {
                match &arm.pattern {
                    surface::MatchPattern::Wildcard => {
                        if wildcard_index.is_some() {
                            return Err(Error::new(
                                "match may contain only one wildcard arm",
                                0,
                            ));
                        }
                        wildcard_index = Some(idx);
                    }
                    surface::MatchPattern::Expr(expr) => {
                        validate_expr(expr)?;
                    }
                    surface::MatchPattern::Compare { expr, .. } => {
                        validate_expr(expr)?;
                    }
                    surface::MatchPattern::Variant { .. } => {}
                }
                validate_expr(&arm.body)?;
            }
            if let Some(idx) = wildcard_index {
                if idx != arms.len() - 1 {
                    return Err(Error::new(
                        "match wildcard '_' arm must be last",
                        0,
                    ));
                }
            }
            Ok(())
        }
    }
}
