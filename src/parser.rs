use crate::ast::surface::{
    DataDef, DataVariant, ExternFnDef, Expr, FieldPattern, FnDef, Item, LetDef, MatchArm,
    MatchPattern, PipeTarget, Program,
};
use crate::ast::{BinaryOp, UnaryOp};
use crate::error::Error;
use crate::lexer::{Token, TokenKind};

pub fn parse_program(tokens: &[Token]) -> Result<Program, Error> {
    Parser::new(tokens).parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    match_scrutinee_depth: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            match_scrutinee_depth: 0,
        }
    }

    fn parse_program(mut self) -> Result<Program, Error> {
        let mut items = Vec::new();
        while matches!(
            self.peek_kind(),
            TokenKind::KwData | TokenKind::KwExtern | TokenKind::KwFn | TokenKind::KwLet
        ) {
            let item = self.parse_item()?;
            self.expect(TokenKind::Semicolon)?;
            items.push(item);
        }
        let expr = if !matches!(self.peek_kind(), TokenKind::Eof) {
            let expr = self.parse_expr()?;
            if matches!(self.peek_kind(), TokenKind::Semicolon) {
                self.advance();
                None
            } else {
                Some(expr)
            }
        } else {
            None
        };
        self.expect(TokenKind::Eof)?;
        Ok(Program { items, expr })
    }

    fn parse_item(&mut self) -> Result<Item, Error> {
        match self.peek_kind() {
            TokenKind::KwData => self.parse_data_def().map(Item::Data),
            TokenKind::KwExtern => self.parse_extern_fn().map(Item::ExternFn),
            TokenKind::KwFn => self.parse_fn().map(Item::Fn),
            TokenKind::KwLet => self.parse_let().map(Item::Let),
            _ => Err(self.error_here("expected 'data', 'extern', 'fn', or 'let'")),
        }
    }

    fn parse_data_def(&mut self) -> Result<DataDef, Error> {
        self.expect(TokenKind::KwData)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let mut variants = Vec::new();
        loop {
            let variant = self.parse_data_variant()?;
            variants.push(variant);
            if matches!(self.peek_kind(), TokenKind::Bar) {
                self.advance();
                continue;
            }
            break;
        }
        if variants.is_empty() {
            return Err(self.error_here("data must define at least one variant"));
        }
        Ok(DataDef { name, variants })
    }

    fn parse_data_variant(&mut self) -> Result<DataVariant, Error> {
        let name = self.expect_ident()?;
        let mut fields = Vec::new();
        if matches!(self.peek_kind(), TokenKind::LBrace) {
            self.advance();
            if matches!(self.peek_kind(), TokenKind::RBrace) {
                self.advance();
                return Ok(DataVariant { name, fields });
            }
            loop {
                fields.push(self.expect_ident()?);
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.advance();
                    if matches!(self.peek_kind(), TokenKind::RBrace) {
                        break;
                    }
                    continue;
                }
                break;
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(DataVariant { name, fields })
    }

    fn parse_extern_fn(&mut self) -> Result<ExternFnDef, Error> {
        self.expect(TokenKind::KwExtern)?;
        self.expect(TokenKind::KwFn)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        Ok(ExternFnDef { name, params })
    }

    fn parse_fn(&mut self) -> Result<FnDef, Error> {
        self.expect(TokenKind::KwFn)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Eq)?;
        let body = self.parse_expr()?;
        Ok(FnDef { name, params, body })
    }

    fn parse_let(&mut self) -> Result<LetDef, Error> {
        self.expect(TokenKind::KwLet)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let expr = self.parse_expr()?;
        Ok(LetDef { name, expr })
    }

    fn parse_params(&mut self) -> Result<Vec<String>, Error> {
        let mut params = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RParen) {
            return Ok(params);
        }
        loop {
            params.push(self.expect_ident()?);
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(params)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr()?);
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(args)
    }

    fn parse_expr(&mut self) -> Result<Expr, Error> {
        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_logic_or()?;
        while matches!(self.peek_kind(), TokenKind::Pipe) {
            self.advance();
            let target = self.parse_pipe_target()?;
            expr = Expr::Pipe {
                input: Box::new(expr),
                target,
            };
        }
        Ok(expr)
    }

    fn parse_pipe_target(&mut self) -> Result<PipeTarget, Error> {
        let name = self.expect_ident()?;
        if matches!(self.peek_kind(), TokenKind::LParen) {
            self.advance();
            let args = self.parse_args()?;
            self.expect(TokenKind::RParen)?;
            Ok(PipeTarget::Call { name, args })
        } else {
            Ok(PipeTarget::Ident(name))
        }
    }

    fn parse_logic_or(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_logic_and()?;
        while matches!(self.peek_kind(), TokenKind::OrOr) {
            self.advance();
            let rhs = self.parse_logic_and()?;
            expr = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_logic_and(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_equality()?;
        while matches!(self.peek_kind(), TokenKind::AndAnd) {
            self.advance();
            let rhs = self.parse_equality()?;
            expr = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::BangEq => BinaryOp::Neq,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_comparison()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_term()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::Lte,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::Gte,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_term()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_factor()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, Error> {
        match self.peek_kind() {
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, Error> {
        let expr = self.parse_primary()?;
        if matches!(self.peek_kind(), TokenKind::LParen) {
            self.advance();
            let args = self.parse_args()?;
            self.expect(TokenKind::RParen)?;
            match expr {
                Expr::Var(name) => Ok(Expr::Call { callee: name, args }),
                _ => Err(self.error_here("callee must be an identifier")),
            }
        } else {
            Ok(expr)
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, Error> {
        match self.peek_kind() {
            TokenKind::Int(value) => {
                let value = *value;
                self.advance();
                Ok(Expr::Int(value))
            }
            TokenKind::KwTrue => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::KwFalse => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::LBracket => self.parse_list_literal(),
            TokenKind::Hash => self.parse_map_literal(),
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                let is_ctor = name
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase());
                if is_ctor
                    && self.match_scrutinee_depth == 0
                    && matches!(self.peek_kind(), TokenKind::LBrace)
                {
                    self.advance();
                    let fields = self.parse_construct_fields()?;
                    self.expect(TokenKind::RBrace)?;
                    Ok(Expr::Construct { name, fields })
                } else {
                    Ok(Expr::Var(name))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::KwIf => self.parse_if_expr(),
            TokenKind::KwMatch => self.parse_match_expr(),
            TokenKind::Invalid(ch) => Err(self.error_here(&format!(
                "invalid character '{}'",
                ch
            ))),
            _ => Err(self.error_here("unexpected token")),
        }
    }

    fn parse_list_literal(&mut self) -> Result<Expr, Error> {
        self.expect(TokenKind::LBracket)?;
        let mut items = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RBracket) {
            self.advance();
            return Ok(Expr::List(items));
        }
        loop {
            items.push(self.parse_expr()?);
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                if matches!(self.peek_kind(), TokenKind::RBracket) {
                    break;
                }
                continue;
            }
            break;
        }
        self.expect(TokenKind::RBracket)?;
        Ok(Expr::List(items))
    }

    fn parse_map_literal(&mut self) -> Result<Expr, Error> {
        self.expect(TokenKind::Hash)?;
        self.expect(TokenKind::LBrace)?;
        let mut entries = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RBrace) {
            self.advance();
            return Ok(Expr::Map(entries));
        }
        loop {
            let key = self.parse_expr()?;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_expr()?;
            entries.push((key, value));
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                if matches!(self.peek_kind(), TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::Map(entries))
    }

    fn parse_if_expr(&mut self) -> Result<Expr, Error> {
        self.expect(TokenKind::KwIf)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let then_branch = self.parse_expr()?;
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::KwElse)?;
        self.expect(TokenKind::LBrace)?;
        let else_branch = self.parse_expr()?;
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, Error> {
        self.expect(TokenKind::KwMatch)?;
        self.match_scrutinee_depth += 1;
        let scrutinee = self.parse_expr()?;
        self.match_scrutinee_depth -= 1;
        self.expect(TokenKind::LBrace)?;
        let mut arms = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RBrace) {
            return Err(self.error_here("match requires at least one arm"));
        }
        loop {
            let pattern = self.parse_match_pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pattern, body });
            if matches!(
                self.peek_kind(),
                TokenKind::Comma | TokenKind::Semicolon
            ) {
                self.advance();
                if matches!(self.peek_kind(), TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, Error> {
        match self.peek_kind() {
            TokenKind::Ident(name) if name == "_" => {
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
            TokenKind::Ident(name)
                if matches!(self.peek_kind_at(1), Some(TokenKind::LBrace)) =>
            {
                let name = name.clone();
                self.advance();
                self.expect(TokenKind::LBrace)?;
                let fields = self.parse_pattern_fields()?;
                self.expect(TokenKind::RBrace)?;
                Ok(MatchPattern::Variant { name, fields })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(MatchPattern::Variant {
                    name,
                    fields: Vec::new(),
                })
            }
            TokenKind::Lt => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(MatchPattern::Compare {
                    op: BinaryOp::Lt,
                    expr,
                })
            }
            TokenKind::LtEq => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(MatchPattern::Compare {
                    op: BinaryOp::Lte,
                    expr,
                })
            }
            TokenKind::Gt => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(MatchPattern::Compare {
                    op: BinaryOp::Gt,
                    expr,
                })
            }
            TokenKind::GtEq => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(MatchPattern::Compare {
                    op: BinaryOp::Gte,
                    expr,
                })
            }
            _ => {
                let expr = self.parse_expr()?;
                Ok(MatchPattern::Expr(expr))
            }
        }
    }

    fn parse_construct_fields(&mut self) -> Result<Vec<(String, Expr)>, Error> {
        let mut fields = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RBrace) {
            return Ok(fields);
        }
        loop {
            let field_name = self.expect_ident()?;
            let expr = if matches!(self.peek_kind(), TokenKind::Colon) {
                self.advance();
                self.parse_expr()?
            } else {
                Expr::Var(field_name.clone())
            };
            fields.push((field_name, expr));
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                if matches!(self.peek_kind(), TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(fields)
    }

    fn parse_pattern_fields(&mut self) -> Result<Vec<FieldPattern>, Error> {
        let mut fields = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RBrace) {
            return Ok(fields);
        }
        loop {
            let field_name = self.expect_ident()?;
            let bind = if matches!(self.peek_kind(), TokenKind::Colon) {
                self.advance();
                match self.peek_kind() {
                    TokenKind::Ident(name) if name == "_" => {
                        self.advance();
                        None
                    }
                    TokenKind::Ident(name) => {
                        let name = name.clone();
                        self.advance();
                        Some(name)
                    }
                    _ => return Err(self.error_here("expected identifier after ':'")),
                }
            } else {
                Some(field_name.clone())
            };
            fields.push(FieldPattern {
                field: field_name,
                bind,
            });
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                if matches!(self.peek_kind(), TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(fields)
    }

    fn expect(&mut self, expected: TokenKind) -> Result<(), Error> {
        let token = self.peek();
        if token.kind == expected {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error_at(
                token.position,
                &format!("expected {:?}, found {:?}", expected, token.kind),
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, Error> {
        match self.peek_kind() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.pos += 1;
                Ok(name)
            }
            _ => Err(self.error_here("expected identifier")),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_kind_at(&self, offset: usize) -> Option<TokenKind> {
        let idx = self.pos + offset;
        self.tokens.get(idx).map(|token| token.kind.clone())
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn error_here(&self, message: &str) -> Error {
        self.error_at(self.peek().position, message)
    }

    fn error_at(&self, position: usize, message: &str) -> Error {
        Error::new(message, position)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_program;
    use crate::lexer::Lexer;
    use crate::validate::validate_program;

    #[test]
    fn parses_pipe_style() {
        let source = r#"
            extern fn add1(x);
            fn abs(x) = if x < 0 { 0 - x } else { x };
            fn grade(x) = match x { _ => x };
            let x = 10;
            let xs = [1, 2, 3];
            let ys = #{ 1: 2, 3: 4 };
            x |> abs |> add1
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        assert!(program.expr.is_some());
        assert_eq!(program.items.len(), 6);
    }

    #[test]
    fn parses_data_construct_and_match() {
        let source = r#"
            data Tree = Empty | Node { value, left, right };
            fn head(t) =
              match t {
                Node { value, left, right } => value;
                _ => 0;
              };
            let t = Node { value: 3, left: Empty, right: Empty };
            head(t)
        "#;
        let tokens = Lexer::new(source).lex_all();
        let program = parse_program(&tokens).expect("parse");
        validate_program(&program).expect("validate");
        assert!(program.items.iter().any(|item| matches!(item, crate::ast::surface::Item::Data(_))));
        assert!(program.items.iter().any(|item| matches!(item, crate::ast::surface::Item::Fn(_))));
    }
}
