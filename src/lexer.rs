#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int(i64),
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
        "#;
        let tokens = Lexer::new(source).lex_all();
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwData)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwMatch)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::FatArrow)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Bar)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::LBracket)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Hash)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Pipe)));
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

    fn lex_ident_or_keyword(&mut self, position: usize) -> Token {
        let start = self.pos;
        while self.peek_char_opt().is_some_and(|c| {
            c.is_ascii_alphanumeric() || c == b'_'
        }) {
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
            while self.peek_char_opt().is_some_and(|c| c.is_ascii_whitespace()) {
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
