#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int(i64),
    Str(String),
    Bytes(Vec<u8>),
    KwExtern,
    KwData,
    KwMatch,
    KwFn,
    KwLet,
    KwIf,
    KwElse,
    KwTrue,
    KwFalse,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Eq,
    FatArrow,
    Bar,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Bang,
    Pipe,
    Hash,
    Invalid(char),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub position: usize,
}

pub struct Lexer<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
    len: usize,
}

#[cfg(test)]
mod tests {
    use super::{Lexer, TokenKind};

    #[test]
    fn lexes_keywords_and_symbols() {
        let source = r#"
            data Tree = Empty | Node { value, left, right };
            match x { >= 1 => y; _ => z; }
            [1,2] #{ 1: 2 } |> f(a)
            "hi" b"hi"
        "#;
        let tokens = Lexer::new(source).lex_all();
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwData)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwMatch)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::FatArrow)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Bar)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::LBracket)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Hash)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Pipe)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Str(_))));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Bytes(_))));
    }

    #[test]
    fn lexes_line_comments() {
        let source = r#"
            let x = 1; // comment
            let y = 2; // another comment
        "#;
        let tokens = Lexer::new(source).lex_all();
        let let_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::KwLet))
            .count();
        assert_eq!(let_count, 2);
        assert!(
            !tokens
                .iter()
                .any(|t| matches!(t.kind, TokenKind::Invalid('/')))
        );
    }
}
impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
            len: input.len(),
        }
    }

    pub fn lex_all(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();
        let position = self.pos;
        if self.pos >= self.len {
            return Token {
                kind: TokenKind::Eof,
                position,
            };
        }
        let ch = self.peek_char();
        match ch {
            b'(' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::LParen,
                    position,
                }
            }
            b')' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::RParen,
                    position,
                }
            }
            b'{' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::LBrace,
                    position,
                }
            }
            b'}' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::RBrace,
                    position,
                }
            }
            b'[' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::LBracket,
                    position,
                }
            }
            b']' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::RBracket,
                    position,
                }
            }
            b',' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Comma,
                    position,
                }
            }
            b':' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Colon,
                    position,
                }
            }
            b';' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Semicolon,
                    position,
                }
            }
            b'=' => {
                self.pos += 1;
                if self.peek_char_opt() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::EqEq,
                        position,
                    }
                } else if self.peek_char_opt() == Some(b'>') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::FatArrow,
                        position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Eq,
                        position,
                    }
                }
            }
            b'+' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Plus,
                    position,
                }
            }
            b'-' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Minus,
                    position,
                }
            }
            b'*' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Star,
                    position,
                }
            }
            b'/' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Slash,
                    position,
                }
            }
            b'%' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Percent,
                    position,
                }
            }
            b'#' => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Hash,
                    position,
                }
            }
            b'!' => {
                self.pos += 1;
                if self.peek_char_opt() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::BangEq,
                        position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Bang,
                        position,
                    }
                }
            }
            b'<' => {
                self.pos += 1;
                if self.peek_char_opt() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::LtEq,
                        position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Lt,
                        position,
                    }
                }
            }
            b'>' => {
                self.pos += 1;
                if self.peek_char_opt() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::GtEq,
                        position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Gt,
                        position,
                    }
                }
            }
            b'&' => {
                self.pos += 1;
                if self.peek_char_opt() == Some(b'&') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::AndAnd,
                        position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Invalid('&'),
                        position,
                    }
                }
            }
            b'|' => {
                self.pos += 1;
                if self.peek_char_opt() == Some(b'|') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::OrOr,
                        position,
                    }
                } else if self.peek_char_opt() == Some(b'>') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::Pipe,
                        position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Bar,
                        position,
                    }
                }
            }
            b'0'..=b'9' => self.lex_number(position),
            b'"' => self.lex_string(position),
            b'b' if self.peek_char_opt_at(1) == Some(b'"') => self.lex_bytes(position),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident_or_keyword(position),
            _ => {
                self.pos += 1;
                Token {
                    kind: TokenKind::Invalid(ch as char),
                    position,
                }
            }
        }
    }

    fn lex_number(&mut self, position: usize) -> Token {
        let start = self.pos;
        while self.peek_char_opt().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        let text = &self.input[start..self.pos];
        let value = text.parse::<i64>().unwrap_or(0);
        Token {
            kind: TokenKind::Int(value),
            position,
        }
    }

    fn lex_string(&mut self, position: usize) -> Token {
        self.pos += 1;
        let mut out = String::new();
        while let Some(ch) = self.peek_char_opt() {
            if ch == b'"' {
                self.pos += 1;
                return Token {
                    kind: TokenKind::Str(out),
                    position,
                };
            }
            if ch == b'\\' {
                self.pos += 1;
                match self.peek_char_opt() {
                    Some(b'n') => {
                        out.push('\n');
                        self.pos += 1;
                    }
                    Some(b'r') => {
                        out.push('\r');
                        self.pos += 1;
                    }
                    Some(b't') => {
                        out.push('\t');
                        self.pos += 1;
                    }
                    Some(b'0') => {
                        out.push('\0');
                        self.pos += 1;
                    }
                    Some(b'\\') => {
                        out.push('\\');
                        self.pos += 1;
                    }
                    Some(b'"') => {
                        out.push('"');
                        self.pos += 1;
                    }
                    Some(b'x') => {
                        if let Some(byte) = self.lex_hex_escape() {
                            out.push(byte as char);
                        } else {
                            return Token {
                                kind: TokenKind::Invalid('\\'),
                                position,
                            };
                        }
                    }
                    _ => {
                        return Token {
                            kind: TokenKind::Invalid('\\'),
                            position,
                        };
                    }
                }
                continue;
            }
            out.push(ch as char);
            self.pos += 1;
        }
        Token {
            kind: TokenKind::Invalid('"'),
            position,
        }
    }

    fn lex_bytes(&mut self, position: usize) -> Token {
        self.pos += 2;
        let mut out = Vec::new();
        while let Some(ch) = self.peek_char_opt() {
            if ch == b'"' {
                self.pos += 1;
                return Token {
                    kind: TokenKind::Bytes(out),
                    position,
                };
            }
            if ch == b'\\' {
                self.pos += 1;
                match self.peek_char_opt() {
                    Some(b'n') => {
                        out.push(b'\n');
                        self.pos += 1;
                    }
                    Some(b'r') => {
                        out.push(b'\r');
                        self.pos += 1;
                    }
                    Some(b't') => {
                        out.push(b'\t');
                        self.pos += 1;
                    }
                    Some(b'0') => {
                        out.push(b'\0');
                        self.pos += 1;
                    }
                    Some(b'\\') => {
                        out.push(b'\\');
                        self.pos += 1;
                    }
                    Some(b'"') => {
                        out.push(b'"');
                        self.pos += 1;
                    }
                    Some(b'x') => {
                        if let Some(byte) = self.lex_hex_escape() {
                            out.push(byte);
                        } else {
                            return Token {
                                kind: TokenKind::Invalid('\\'),
                                position,
                            };
                        }
                    }
                    _ => {
                        return Token {
                            kind: TokenKind::Invalid('\\'),
                            position,
                        };
                    }
                }
                continue;
            }
            out.push(ch);
            self.pos += 1;
        }
        Token {
            kind: TokenKind::Invalid('"'),
            position,
        }
    }

    fn lex_hex_escape(&mut self) -> Option<u8> {
        self.pos += 1;
        let hi = self.peek_char_opt()?;
        self.pos += 1;
        let lo = self.peek_char_opt()?;
        self.pos += 1;
        let hi = (hi as char).to_digit(16)?;
        let lo = (lo as char).to_digit(16)?;
        Some(((hi << 4) | lo) as u8)
    }

    fn lex_ident_or_keyword(&mut self, position: usize) -> Token {
        let start = self.pos;
        while self
            .peek_char_opt()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            self.pos += 1;
        }
        let text = &self.input[start..self.pos];
        let kind = match text {
            "extern" => TokenKind::KwExtern,
            "data" => TokenKind::KwData,
            "match" => TokenKind::KwMatch,
            "fn" => TokenKind::KwFn,
            "let" => TokenKind::KwLet,
            "if" => TokenKind::KwIf,
            "else" => TokenKind::KwElse,
            "true" => TokenKind::KwTrue,
            "false" => TokenKind::KwFalse,
            _ => TokenKind::Ident(text.to_string()),
        };
        Token { kind, position }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self
                .peek_char_opt()
                .is_some_and(|c| c.is_ascii_whitespace())
            {
                self.pos += 1;
            }
            if self.peek_char_opt() == Some(b'/') && self.peek_char_opt_at(1) == Some(b'/') {
                self.pos += 2;
                while self.peek_char_opt().is_some_and(|c| c != b'\n') {
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
    }

    fn peek_char(&self) -> u8 {
        self.bytes[self.pos]
    }

    fn peek_char_opt(&self) -> Option<u8> {
        if self.pos >= self.len {
            None
        } else {
            Some(self.bytes[self.pos])
        }
    }

    fn peek_char_opt_at(&self, offset: usize) -> Option<u8> {
        let idx = self.pos + offset;
        if idx >= self.len {
            None
        } else {
            Some(self.bytes[idx])
        }
    }
}
