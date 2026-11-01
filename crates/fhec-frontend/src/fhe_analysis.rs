use std::collections::HashMap;

use crate::ast::{BinaryOperator, Block, Expression, Function, Program, Statement, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FheExpressionKind {
    Plaintext,
    Encrypted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FheOperation {
    pub operator: BinaryOperator,
    pub result_kind: FheExpressionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FheBranch {
    pub condition_type: Type,
    pub requires_mux: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FheAnalysis {
    pub encrypted_operations: usize,
    pub operations: Vec<FheOperation>,
    pub encrypted_branches: Vec<FheBranch>,
}

pub struct FheAnalyzer {
    function_returns: HashMap<String, FheExpressionKind>,
}

impl Default for FheAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl FheAnalyzer {
    pub fn new() -> Self {
        Self {
            function_returns: HashMap::new(),
        }
    }

    pub fn analyze(mut self, program: &Program) -> FheAnalysis {
        self.function_returns.clear();

        for function in &program.functions {
            self.function_returns
                .insert(function.name.clone(), self.type_kind(&function.return_type));
        }
        let mut analysis = FheAnalysis {
            encrypted_operations: 0,
            operations: Vec::new(),
            encrypted_branches: Vec::new(),
        };

        for function in &program.functions {
            self.analyze_function(function, &mut analysis);
        }

        analysis
    }

    fn analyze_function(&self, function: &Function, analysis: &mut FheAnalysis) {
        let mut variables = HashMap::new();

        for parameter in &function.parameters {
            let kind = self.type_kind(&parameter.ty);

            variables.insert(parameter.name.clone(), kind);
        }

        self.analyze_block(&function.body, &mut variables, analysis);
    }

    fn analyze_block(
        &self,
        block: &Block,
        variables: &mut HashMap<String, FheExpressionKind>,
        analysis: &mut FheAnalysis,
    ) {
        for statement in &block.statements {
            match statement {
                Statement::Let { name, ty, value } => {
                    self.analyze_expression(value, variables, analysis);

                    let kind = match ty {
                        Some(declared_type) => self.type_kind(declared_type),

                        None => self.expression_kind(value, variables),
                    };

                    variables.insert(name.clone(), kind);
                }

                Statement::Return(Some(expression)) => {
                    self.analyze_expression(expression, variables, analysis);
                }

                Statement::Return(None) => {}

                Statement::Expression(expression) => {
                    self.analyze_expression(expression, variables, analysis);
                }
            }
        }
    }

    fn analyze_expression(
        &self,
        expression: &Expression,
        variables: &mut HashMap<String, FheExpressionKind>,
        analysis: &mut FheAnalysis,
    ) {
        match expression {
            Expression::Integer(_) | Expression::Boolean(_) | Expression::Variable(_) => {}

            Expression::Call { arguments, .. } => {
                for argument in arguments {
                    self.analyze_expression(argument, variables, analysis);
                }
            }

            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let is_encrypted = self.expression_kind(left, variables)
                    == FheExpressionKind::Encrypted
                    || self.expression_kind(right, variables) == FheExpressionKind::Encrypted;

                if is_encrypted && self.is_fhe_operation(*operator) {
                    analysis.encrypted_operations += 1;

                    analysis.operations.push(FheOperation {
                        operator: *operator,
                        result_kind: FheExpressionKind::Encrypted,
                    });
                }

                self.analyze_expression(left, variables, analysis);

                self.analyze_expression(right, variables, analysis);
            }

            Expression::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_kind = self.expression_kind(condition, variables);

                if condition_kind == FheExpressionKind::Encrypted {
                    analysis.encrypted_branches.push(FheBranch {
                        condition_type: Type::Encrypted(Box::new(Type::Bool)),
                        requires_mux: true,
                    });
                }

                self.analyze_expression(condition, variables, analysis);

                let mut then_variables = variables.clone();

                self.analyze_block(then_branch, &mut then_variables, analysis);

                if let Some(else_branch) = else_branch {
                    let mut else_variables = variables.clone();

                    self.analyze_block(else_branch, &mut else_variables, analysis);
                }
            }
        }
    }

    fn expression_kind(
        &self,
        expression: &Expression,
        variables: &HashMap<String, FheExpressionKind>,
    ) -> FheExpressionKind {
        match expression {
            Expression::Integer(_) | Expression::Boolean(_) => FheExpressionKind::Plaintext,

            Expression::Variable(name) => variables
                .get(name)
                .copied()
                .unwrap_or(FheExpressionKind::Plaintext),

            Expression::Call { function, .. } => self
                .function_returns
                .get(function)
                .copied()
                .unwrap_or(FheExpressionKind::Plaintext),

            Expression::Binary { left, right, .. } => {
                if self.expression_kind(left, variables) == FheExpressionKind::Encrypted
                    || self.expression_kind(right, variables) == FheExpressionKind::Encrypted
                {
                    FheExpressionKind::Encrypted
                } else {
                    FheExpressionKind::Plaintext
                }
            }

            Expression::If { .. } => FheExpressionKind::Plaintext,
        }
    }

    fn type_kind(&self, ty: &Type) -> FheExpressionKind {
        match ty {
            Type::Encrypted(_) => FheExpressionKind::Encrypted,

            Type::Plaintext(_) | Type::Bool | Type::UInt(_) | Type::Int(_) | Type::Unit => {
                FheExpressionKind::Plaintext
            }
        }
    }

    fn is_fhe_operation(&self, operator: BinaryOperator) -> bool {
        matches!(
            operator,
            BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::Greater
                | BinaryOperator::GreaterEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::type_checker::TypeChecker;

    fn analyze(source: &str) -> FheAnalysis {
        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        TypeChecker::new()
            .check(&program)
            .expect("type checking failed");

        FheAnalyzer::new().analyze(&program)
    }

    #[test]
    fn detects_encrypted_addition() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 1);
    }

    #[test]
    fn detects_encrypted_comparison() {
        let source = r#"
            fn compare(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<bool> {
                return a > b;
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 1);
    }

    #[test]
    fn detects_encrypted_branch() {
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

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_branches.len(), 1);
        assert!(analysis.encrypted_branches[0].requires_mux);
    }

    #[test]
    fn plaintext_branch_does_not_require_mux() {
        let source = r#"
            fn max(
                a: u8,
                b: u8
            ) -> u8 {
                if a > b {
                    a
                } else {
                    b
                }
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_branches.len(), 0);
    }

    #[test]
    fn tracks_encrypted_let_binding() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                let x = a + b;
                return x;
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 1);
    }

    #[test]
    fn tracks_plaintext_let_binding() {
        let source = r#"
            fn add(
                a: u8,
                b: u8
            ) -> u8 {
                let x = a + b;
                return x;
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 0);
    }

    #[test]
    fn encrypted_let_requires_mux() {
        let source = r#"
            fn max(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                let x = a + b;

                if x > a {
                    x
                } else {
                    a
                }
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 2);
        assert_eq!(analysis.encrypted_branches.len(), 1);
        assert!(analysis.encrypted_branches[0].requires_mux);
    }

    #[test]
    fn records_encrypted_operation() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.operations.len(), 1);
        assert_eq!(analysis.operations[0].operator, BinaryOperator::Add);
        assert_eq!(
            analysis.operations[0].result_kind,
            FheExpressionKind::Encrypted
        );
    }
    #[test]
    fn encrypted_function_call_is_supported_by_analysis() {
        let source = r#"
            fn identity(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a;
            }

            fn main(
                x: encrypted<u8>
            ) -> encrypted<u8> {
                return identity(x);
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 0);
    }

    #[test]
    fn function_call_arguments_are_analyzed() {
        let source = r#"
            fn passthrough(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a;
            }

            fn main(
                x: encrypted<u8>,
                y: encrypted<u8>
            ) -> encrypted<u8> {
                return passthrough(x + y);
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 1);

        assert_eq!(analysis.operations.len(), 1);

        assert_eq!(analysis.operations[0].operator, BinaryOperator::Add);
    }
    #[test]
    fn records_multiple_encrypted_operations() {
        let source = r#"
            fn compute(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                let x = a + b;
                let y = x * a;
                return y;
            }
        "#;

        let analysis = analyze(source);

        assert_eq!(analysis.encrypted_operations, 2);
        assert_eq!(analysis.operations.len(), 2);

        assert_eq!(analysis.operations[0].operator, BinaryOperator::Add);

        assert_eq!(analysis.operations[1].operator, BinaryOperator::Multiply);

        assert_eq!(
            analysis.operations[0].result_kind,
            FheExpressionKind::Encrypted
        );

        assert_eq!(
            analysis.operations[1].result_kind,
            FheExpressionKind::Encrypted
        );
    }
}
