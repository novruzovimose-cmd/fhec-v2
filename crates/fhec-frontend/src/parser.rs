use crate::ast::{
    BinaryOperator, Block, Expression, Function, Parameter, Program, Statement, Type,
};
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    pub message: String,
    pub position: usize,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut functions = Vec::new();

        while !self.check(TokenKind::Eof) {
            functions.push(self.parse_function()?);
        }

        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, ParserError> {
        self.consume(TokenKind::Fn, "expected 'fn'")?;

        let name = self.consume_identifier("expected function name")?;

        self.consume(TokenKind::LeftParen, "expected '(' after function name")?;

        let parameters = self.parse_parameters()?;

        self.consume(TokenKind::RightParen, "expected ')' after parameters")?;

        self.consume(TokenKind::Arrow, "expected '->' after parameters")?;

        let return_type = self.parse_type()?;

        let body = self.parse_block()?;

        Ok(Function {
            name,
            parameters,
            return_type,
            body,
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, ParserError> {
        let mut parameters = Vec::new();

        if self.check(TokenKind::RightParen) {
            return Ok(parameters);
        }

        loop {
            let name = self.consume_identifier("expected parameter name")?;

            self.consume(TokenKind::Colon, "expected ':' after parameter name")?;

            let ty = self.parse_type()?;

            parameters.push(Parameter { name, ty });

            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        Ok(parameters)
    }

    fn parse_type(&mut self) -> Result<Type, ParserError> {
        if self.match_token(TokenKind::Bool) {
            return Ok(Type::Bool);
        }

        if self.match_token(TokenKind::Encrypted) {
            self.consume(TokenKind::Less, "expected '<' after 'encrypted'")?;

            let inner = self.parse_type()?;

            self.consume(TokenKind::Greater, "expected '>' after encrypted type")?;

            return Ok(Type::Encrypted(Box::new(inner)));
        }

        if self.match_token(TokenKind::Plaintext) {
            self.consume(TokenKind::Less, "expected '<' after 'plaintext'")?;

            let inner = self.parse_type()?;

            self.consume(TokenKind::Greater, "expected '>' after plaintext type")?;

            return Ok(Type::Plaintext(Box::new(inner)));
        }

        if self.check(TokenKind::Identifier) {
            let token = self.advance();

            if let Some(width) = token.lexeme.strip_prefix('u') {
                if let Ok(width) = width.parse::<u32>() {
                    return Ok(Type::UInt(width));
                }
            }

            if let Some(width) = token.lexeme.strip_prefix('i') {
                if let Ok(width) = width.parse::<u32>() {
                    return Ok(Type::Int(width));
                }
            }
        }

        Err(self.error("expected type"))
    }

    fn parse_block(&mut self) -> Result<Block, ParserError> {
        self.consume(TokenKind::LeftBrace, "expected '{'")?;

        let mut statements = Vec::new();

        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            statements.push(self.parse_statement()?);
        }

        self.consume(TokenKind::RightBrace, "expected '}'")?;

        Ok(Block { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParserError> {
        if self.match_token(TokenKind::Let) {
            return self.parse_let_statement();
        }

        if self.match_token(TokenKind::Return) {
            return self.parse_return_statement();
        }

        let expression = self.parse_expression()?;

        Ok(Statement::Expression(expression))
    }

    fn parse_let_statement(&mut self) -> Result<Statement, ParserError> {
        let name = self.consume_identifier("expected variable name")?;

        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(TokenKind::Equal, "expected '=' in let statement")?;

        let value = self.parse_expression()?;

        self.match_token(TokenKind::Semicolon);

        Ok(Statement::Let { name, ty, value })
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParserError> {
        if self.check(TokenKind::RightBrace) {
            return Ok(Statement::Return(None));
        }

        let expression = self.parse_expression()?;

        self.match_token(TokenKind::Semicolon);

        Ok(Statement::Return(Some(expression)))
    }
}
impl Parser {
    fn parse_expression(&mut self) -> Result<Expression, ParserError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParserError> {
        let mut expression = self.parse_term()?;

        loop {
            let operator = match self.peek().kind {
                TokenKind::EqualEqual => BinaryOperator::Equal,
                TokenKind::NotEqual => BinaryOperator::NotEqual,
                TokenKind::Greater => BinaryOperator::Greater,
                TokenKind::GreaterEqual => BinaryOperator::GreaterEqual,
                TokenKind::Less => BinaryOperator::Less,
                TokenKind::LessEqual => BinaryOperator::LessEqual,
                _ => break,
            };

            self.advance();

            let right = self.parse_term()?;

            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_term(&mut self) -> Result<Expression, ParserError> {
        let mut expression = self.parse_factor()?;

        loop {
            let operator = match self.peek().kind {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Subtract,
                _ => break,
            };

            self.advance();

            let right = self.parse_factor()?;

            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_factor(&mut self) -> Result<Expression, ParserError> {
        let mut expression = self.parse_primary()?;

        while let TokenKind::Star = self.peek().kind {
            let operator = BinaryOperator::Multiply;

            self.advance();

            let right = self.parse_primary()?;

            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParserError> {
        if self.match_token(TokenKind::True) {
            return Ok(Expression::Boolean(true));
        }

        if self.match_token(TokenKind::False) {
            return Ok(Expression::Boolean(false));
        }

        if self.match_token(TokenKind::Integer) {
            let token = self.previous();

            let value = token.lexeme.parse::<u64>().map_err(|_| ParserError {
                message: "invalid integer literal".to_string(),
                position: token.position,
            })?;

            return Ok(Expression::Integer(value));
        }

        if self.match_token(TokenKind::If) {
            return self.parse_if_expression();
        }

        if self.match_token(TokenKind::LeftParen) {
            let expression = self.parse_expression()?;

            self.consume(TokenKind::RightParen, "expected ')' after expression")?;

            return Ok(expression);
        }

        if self.check(TokenKind::Identifier) {
            let name = self.advance().lexeme;

            if self.match_token(TokenKind::LeftParen) {
                let mut arguments = Vec::new();

                if !self.check(TokenKind::RightParen) {
                    loop {
                        arguments.push(self.parse_expression()?);

                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.consume(
                    TokenKind::RightParen,
                    "expected ')' after function arguments",
                )?;

                return Ok(Expression::Call {
                    function: name,
                    arguments,
                });
            }

            return Ok(Expression::Variable(name));
        }

        Err(self.error("expected expression"))
    }

    fn parse_if_expression(&mut self) -> Result<Expression, ParserError> {
        let condition = self.parse_expression()?;

        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(TokenKind::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Expression::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }
}
impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> Token {
        if !self.check(TokenKind::Eof) {
            self.current += 1;
        }

        self.tokens[self.current - 1].clone()
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<Token, ParserError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String, ParserError> {
        if self.check(TokenKind::Identifier) {
            return Ok(self.advance().lexeme);
        }

        Err(self.error(message))
    }

    fn error(&self, message: &str) -> ParserError {
        ParserError {
            message: message.to_string(),
            position: self.peek().position,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOperator, Expression, Type};

    fn parse(source: &str) -> Program {
        let tokens = crate::Lexer::new(source).tokenize().expect("lexer failed");

        Parser::new(tokens).parse().expect("parser failed")
    }

    #[test]
    fn parses_empty_function() {
        let program = parse(
            r#"
            fn main() -> bool {
            }
            "#,
        );

        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn parses_encrypted_parameters() {
        let program = parse(
            r#"
            fn max(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                a
            }
            "#,
        );

        let function = &program.functions[0];

        assert_eq!(function.parameters.len(), 2);

        assert_eq!(
            function.parameters[0].ty,
            Type::Encrypted(Box::new(Type::UInt(8)))
        );

        assert_eq!(
            function.return_type,
            Type::Encrypted(Box::new(Type::UInt(8)))
        );
    }

    #[test]
    fn parses_comparison() {
        let program = parse(
            r#"
            fn compare(a: encrypted<u8>, b: encrypted<u8>) -> bool {
                a > b
            }
            "#,
        );

        let expression = &program.functions[0].body.statements[0];

        match expression {
            Statement::Expression(Expression::Binary {
                operator: BinaryOperator::Greater,
                ..
            }) => {}

            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parses_max_function() {
        let program = parse(
            r#"
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
            "#,
        );

        let function = &program.functions[0];

        assert_eq!(function.name, "max");
        assert_eq!(function.parameters.len(), 2);

        assert_eq!(function.body.statements.len(), 1);

        match &function.body.statements[0] {
            Statement::Expression(Expression::If {
                condition,
                then_branch,
                else_branch,
            }) => {
                assert!(matches!(
                    condition.as_ref(),
                    Expression::Binary {
                        operator: BinaryOperator::Greater,
                        ..
                    }
                ));

                assert_eq!(then_branch.statements.len(), 1);
                assert!(else_branch.is_some());
            }

            other => panic!("expected if expression, got {other:?}"),
        }
    }

    #[test]
    fn parses_true_boolean_literal() {
        let program = parse(
            r#"
            fn value() -> bool {
                return true;
            }
            "#,
        );

        assert!(matches!(
            program.functions[0].body.statements[0],
            Statement::Return(Some(Expression::Boolean(true)))
        ));
    }

    #[test]
    fn parses_false_boolean_literal() {
        let program = parse(
            r#"
            fn value() -> bool {
                return false;
            }
            "#,
        );

        assert!(matches!(
            program.functions[0].body.statements[0],
            Statement::Return(Some(Expression::Boolean(false)))
        ));
    }

    #[test]
    fn parses_boolean_literal_inside_if() {
        let program = parse(
            r#"
            fn choose(
                condition: encrypted<bool>,
                a: encrypted<bool>
            ) -> encrypted<bool> {
                return if condition {
                    a
                } else {
                    true
                };
            }
            "#,
        );

        let Statement::Return(Some(Expression::If {
            else_branch: Some(else_branch),
            ..
        })) = &program.functions[0].body.statements[0]
        else {
            panic!("expected returned if expression");
        };

        assert!(matches!(
            else_branch.statements[0],
            Statement::Expression(Expression::Boolean(true))
        ));
    }
    #[test]
    fn parses_function_call() {
        let program = parse(
            r#"
            fn add(a: u8, b: u8) -> u8 {
                return a + b;
            }

            fn main(a: u8, b: u8) -> u8 {
                return add(a, b);
            }
            "#,
        );

        let statement = &program.functions[1].body.statements[0];

        match statement {
            Statement::Return(Some(Expression::Call {
                function,
                arguments,
            })) => {
                assert_eq!(function, "add");
                assert_eq!(arguments.len(), 2);
            }

            other => {
                panic!("expected function call, got {other:?}");
            }
        }
    }

    #[test]
    fn parses_zero_argument_function_call() {
        let program = parse(
            r#"
            fn value() -> u8 {
                return 5;
            }

            fn main() -> u8 {
                return value();
            }
            "#,
        );

        match &program.functions[1].body.statements[0] {
            Statement::Return(Some(Expression::Call {
                function,
                arguments,
            })) => {
                assert_eq!(function, "value");
                assert!(arguments.is_empty());
            }

            other => {
                panic!("expected zero-argument call, got {other:?}");
            }
        }
    }

    #[test]
    fn parses_nested_function_call_arguments() {
        let program = parse(
            r#"
            fn add(a: u8, b: u8) -> u8 {
                return a + b;
            }

            fn main(
                a: u8,
                b: u8,
                c: u8
            ) -> u8 {
                return add(a + b, c * 2);
            }
            "#,
        );

        match &program.functions[1].body.statements[0] {
            Statement::Return(Some(Expression::Call {
                function,
                arguments,
            })) => {
                assert_eq!(function, "add");
                assert_eq!(arguments.len(), 2);

                assert!(matches!(arguments[0], Expression::Binary { .. }));

                assert!(matches!(arguments[1], Expression::Binary { .. }));
            }

            other => {
                panic!("expected nested call, got {other:?}");
            }
        }
    }
    #[test]
    fn respects_arithmetic_precedence() {
        let program = parse(
            r#"
            fn calculate(a: u8, b: u8, c: u8) -> u8 {
                a + b * c
            }
            "#,
        );

        let expression = &program.functions[0].body.statements[0];

        match expression {
            Statement::Expression(Expression::Binary {
                operator: BinaryOperator::Add,
                right,
                ..
            }) => {
                assert!(matches!(
                    right.as_ref(),
                    Expression::Binary {
                        operator: BinaryOperator::Multiply,
                        ..
                    }
                ));
            }

            other => panic!("unexpected AST: {other:?}"),
        }
    }
}
