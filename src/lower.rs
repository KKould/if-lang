use crate::ast::{core, surface};

pub fn lower_program(program: surface::Program) -> core::Program {
    core::Program {
        items: program.items.into_iter().map(lower_item).collect(),
        expr: program.expr.map(lower_expr),
    }
}

fn lower_item(item: surface::Item) -> core::Item {
    match item {
        surface::Item::Fn(def) => core::Item::Fn(lower_fn(def)),
        surface::Item::Let(def) => core::Item::Let(lower_let(def)),
        surface::Item::ExternFn(def) => core::Item::ExternFn(lower_extern_fn(def)),
        surface::Item::Data(def) => core::Item::Data(lower_data_def(def)),
    }
}

fn lower_fn(def: surface::FnDef) -> core::FnDef {
    core::FnDef {
        name: def.name,
        params: def.params,
        body: lower_expr(def.body),
    }
}

fn lower_let(def: surface::LetDef) -> core::LetDef {
    core::LetDef {
        name: def.name,
        expr: lower_expr(def.expr),
    }
}

fn lower_extern_fn(def: surface::ExternFnDef) -> core::ExternFnDef {
    core::ExternFnDef {
        name: def.name,
        params: def.params,
    }
}

fn lower_data_def(def: surface::DataDef) -> core::DataDef {
    core::DataDef {
        name: def.name,
        variants: def
            .variants
            .into_iter()
            .map(|variant| core::DataVariant {
                name: variant.name,
                fields: variant.fields,
            })
            .collect(),
    }
}

fn lower_expr(expr: surface::Expr) -> core::Expr {
    match expr {
        surface::Expr::Int(value) => core::Expr::Int(value),
        surface::Expr::Bool(value) => core::Expr::Bool(value),
        surface::Expr::Str(value) => core::Expr::Str(value),
        surface::Expr::Bytes(value) => core::Expr::Bytes(value),
        surface::Expr::List(items) => {
            core::Expr::List(items.into_iter().map(lower_expr).collect())
        }
        surface::Expr::Map(entries) => core::Expr::Map(
            entries
                .into_iter()
                .map(|(k, v)| (lower_expr(k), lower_expr(v)))
                .collect(),
        ),
        surface::Expr::Var(name) => core::Expr::Var(name),
        surface::Expr::Construct { name, fields } => core::Expr::Construct {
            name,
            fields: fields
                .into_iter()
                .map(|(field, expr)| (field, lower_expr(expr)))
                .collect(),
        },
        surface::Expr::Unary { op, expr } => core::Expr::Unary {
            op,
            expr: Box::new(lower_expr(*expr)),
        },
        surface::Expr::Binary { op, left, right } => core::Expr::Binary {
            op,
            left: Box::new(lower_expr(*left)),
            right: Box::new(lower_expr(*right)),
        },
        surface::Expr::If {
            cond,
            then_branch,
            else_branch,
        } => core::Expr::If {
            cond: Box::new(lower_expr(*cond)),
            then_branch: Box::new(lower_expr(*then_branch)),
            else_branch: Box::new(lower_expr(*else_branch)),
        },
        surface::Expr::Call { callee, args } => core::Expr::Call {
            callee,
            args: args.into_iter().map(lower_expr).collect(),
        },
        surface::Expr::Pipe { input, target } => {
            let input = lower_expr(*input);
            match target {
                surface::PipeTarget::Ident(name) => core::Expr::Call {
                    callee: name,
                    args: vec![input],
                },
                surface::PipeTarget::Call { name, args } => {
                    let mut lowered_args: Vec<core::Expr> =
                        args.into_iter().map(lower_expr).collect();
                    lowered_args.insert(0, input);
                    core::Expr::Call {
                        callee: name,
                        args: lowered_args,
                    }
                }
            }
        }
        surface::Expr::Match { scrutinee, arms } => core::Expr::Match {
            scrutinee: Box::new(lower_expr(*scrutinee)),
            arms: arms.into_iter().map(lower_match_arm).collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::lower::lower_program;
    use crate::parser::parse_program;
    use crate::validate::validate_program;

    #[test]
    fn lowers_pipe_with_call_target() {
        let source = r#"
            fn f(x, y) = x + y;
            let x = 3;
            x |> f(4)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        let core = lower_program(program);
        let expr = core.expr.expect("expr");
        match expr {
            crate::ast::core::Expr::Call { callee, args } => {
                assert_eq!(callee, "f");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected call"),
        }
    }
}

fn lower_match_arm(arm: surface::MatchArm) -> core::MatchArm {
    core::MatchArm {
        pattern: lower_match_pattern(arm.pattern),
        body: lower_expr(arm.body),
    }
}

fn lower_match_pattern(pattern: surface::MatchPattern) -> core::MatchPattern {
    match pattern {
        surface::MatchPattern::Wildcard => core::MatchPattern::Wildcard,
        surface::MatchPattern::Expr(expr) => core::MatchPattern::Expr(lower_expr(expr)),
        surface::MatchPattern::Compare { op, expr } => core::MatchPattern::Compare {
            op,
            expr: lower_expr(expr),
        },
        surface::MatchPattern::Variant { name, fields } => core::MatchPattern::Variant {
            name,
            fields: fields
                .into_iter()
                .map(|field| core::FieldPattern {
                    field: field.field,
                    bind: field.bind,
                })
                .collect(),
        },
    }
}
