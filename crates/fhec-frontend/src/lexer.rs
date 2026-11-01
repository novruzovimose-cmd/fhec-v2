use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    pub message: String,
    pub position: usize,
}

pub struct Lexer<'a> {
    #[allow(dead_code)]
    source: &'a str,
    chars: Vec<char>,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            position: 0,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();

            if self.is_at_end() {
                break;
            }

            tokens.push(self.next_token()?);
        }

        tokens.push(Token::new(TokenKind::Eof, "", self.position));

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexerError> {
        let start = self.position;
        let ch = self.advance();

        let token = match ch {
            '(' => Token::new(TokenKind::LeftParen, "(", start),
            ')' => Token::new(TokenKind::RightParen, ")", start),

            '{' => Token::new(TokenKind::LeftBrace, "{", start),
            '}' => Token::new(TokenKind::RightBrace, "}", start),

            ',' => Token::new(TokenKind::Comma, ",", start),
            ':' => Token::new(TokenKind::Colon, ":", start),
            ';' => Token::new(TokenKind::Semicolon, ";", start),

            '+' => Token::new(TokenKind::Plus, "+", start),
            '*' => Token::new(TokenKind::Star, "*", start),
            '/' => Token::new(TokenKind::Slash, "/", start),

            '-' => {
                if self.match_char('>') {
                    Token::new(TokenKind::Arrow, "->", start)
                } else {
                    Token::new(TokenKind::Minus, "-", start)
                }
            }

            '=' => {
                if self.match_char('=') {
                    Token::new(TokenKind::EqualEqual, "==", start)
                } else {
                    Token::new(TokenKind::Equal, "=", start)
                }
            }

            '!' => {
                if self.match_char('=') {
                    Token::new(TokenKind::NotEqual, "!=", start)
                } else {
                    return Err(self.error("unexpected '!'; expected '!='"));
                }
            }

            '>' => {
                if self.match_char('=') {
                    Token::new(TokenKind::GreaterEqual, ">=", start)
                } else {
                    Token::new(TokenKind::Greater, ">", start)
                }
            }

            '<' => {
                if self.match_char('=') {
                    Token::new(TokenKind::LessEqual, "<=", start)
                } else {
                    Token::new(TokenKind::Less, "<", start)
                }
            }

            c if c.is_ascii_digit() => {
                return Ok(self.number(start, c));
            }

            c if is_identifier_start(c) => {
                return Ok(self.identifier(start));
            }

            c => {
                return Err(self.error(&format!("unexpected character '{c}'")));
            }
        };

        Ok(token)
    }

    fn identifier(&mut self, start: usize) -> Token {
        while !self.is_at_end() && is_identifier_continue(self.peek()) {
            self.advance();
        }

        let lexeme: String = self.chars[start..self.position].iter().collect();

        let kind = match lexeme.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "return" => TokenKind::Return,

            "encrypted" => TokenKind::Encrypted,
            "plaintext" => TokenKind::Plaintext,
            "bool" => TokenKind::Bool,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "u" => TokenKind::UInt,
            "i" => TokenKind::Int,

            _ => TokenKind::Identifier,
        };

        Token::new(kind, lexeme, start)
    }

    fn number(&mut self, start: usize, first: char) -> Token {
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }

        let mut lexeme = String::new();
        lexeme.push(first);

        lexeme.extend(self.chars[start + 1..self.position].iter());

        Token::new(TokenKind::Integer, lexeme, start)
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.peek().is_whitespace() {
            self.advance();
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            return false;
        }

        self.advance();
        true
    }

    fn advance(&mut self) -> char {
        let ch = self.chars[self.position];
        self.position += 1;
        ch
    }

    fn peek(&self) -> char {
        self.chars[self.position]
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.chars.len()
    }

    fn error(&self, message: &str) -> LexerError {
        LexerError {
            message: message.to_string(),
            position: self.position,
        }
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod boolean_literal_tests {
    use super::*;

    #[test]
    fn lexes_true_keyword() {
        let tokens = Lexer::new("true").tokenize().expect("lexing failed");

        assert_eq!(tokens[0].kind, TokenKind::True);
    }

    #[test]
    fn lexes_false_keyword() {
        let tokens = Lexer::new("false").tokenize().expect("lexing failed");

        assert_eq!(tokens[0].kind, TokenKind::False);
    }
}
