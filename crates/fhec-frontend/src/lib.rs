pub mod ast;
pub mod diagnostic;
pub mod fhe_analysis;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod type_checker;

pub use fhe_analysis::{FheAnalysis, FheAnalyzer, FheBranch};
pub use lexer::{Lexer, LexerError};
pub use parser::{Parser, ParserError};
pub use token::{Token, TokenKind};
pub use type_checker::{TypeChecker, TypeError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_recognizes_function_keyword() {
        let tokens = Lexer::new("fn").tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Fn);
    }

    #[test]
    fn lexer_recognizes_encrypted_keyword() {
        let tokens = Lexer::new("encrypted").tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Encrypted);
    }

    #[test]
    fn lexer_recognizes_comparison() {
        let tokens = Lexer::new("a > b").tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::Greater);
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
    }

    #[test]
    fn lexer_recognizes_arrow() {
        let tokens = Lexer::new("->").tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Arrow);
    }

    #[test]
    fn lexer_recognizes_integer() {
        let tokens = Lexer::new("123").tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Integer);
        assert_eq!(tokens[0].lexeme, "123");
    }

    #[test]
    fn lexer_recognizes_assignment() {
        let tokens = Lexer::new("x = 42").tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::Equal);
        assert_eq!(tokens[2].kind, TokenKind::Integer);
    }

    #[test]
    fn lexer_recognizes_max_function() {
        let source = r#"
            fn max(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                if a > b {
                    a
                } else {
                    b
                }
            }
        "#;

        let tokens = Lexer::new(source).tokenize().unwrap();

        assert_eq!(tokens.first().unwrap().kind, TokenKind::Fn);
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);

        assert!(tokens.iter().any(|t| { t.kind == TokenKind::Encrypted }));

        assert!(tokens.iter().any(|t| { t.kind == TokenKind::Greater }));
    }
}
