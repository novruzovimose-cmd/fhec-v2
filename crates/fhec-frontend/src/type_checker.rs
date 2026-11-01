use std::collections::HashMap;

use crate::ast::{
    BinaryOperator, Block, Expression, Function, Parameter, Program, Statement, Type,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub message: String,
}

impl TypeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub struct TypeChecker {
    functions: HashMap<String, FunctionSignature>,
    scopes: Vec<HashMap<String, Type>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionSignature {
    parameters: Vec<Type>,
    return_type: Type,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
impl TypeChecker {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            scopes: Vec::new(),
        }
    }

    pub fn check(&mut self, program: &Program) -> Result<(), TypeError> {
        self.collect_function_signatures(program)?;

        for function in &program.functions {
            self.check_function(function)?;
        }

        Ok(())
    }

    fn collect_function_signatures(&mut self, program: &Program) -> Result<(), TypeError> {
        for function in &program.functions {
            if self.functions.contains_key(&function.name) {
                return Err(TypeError::new(format!(
                    "duplicate function '{}'",
                    function.name
                )));
            }

            let signature = FunctionSignature {
                parameters: function
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                return_type: function.return_type.clone(),
            };

            self.functions.insert(function.name.clone(), signature);
        }

        Ok(())
    }

    fn check_function(&mut self, function: &Function) -> Result<(), TypeError> {
        self.push_scope();

        for parameter in &function.parameters {
            self.define_variable(parameter)?;
        }

        self.check_block(&function.body, &function.return_type)?;

        self.pop_scope();

        Ok(())
    }

    fn check_block(&mut self, block: &Block, expected_return: &Type) -> Result<(), TypeError> {
        for statement in &block.statements {
            self.check_statement(statement, expected_return)?;
        }

        Ok(())
    }

    fn check_statement(
        &mut self,
        statement: &Statement,
        expected_return: &Type,
    ) -> Result<(), TypeError> {
        match statement {
            Statement::Let { name, ty, value } => {
                let value_type = self.check_expression(value)?;

                if let Some(declared_type) = ty {
                    if declared_type != &value_type {
                        return Err(TypeError::new(format!(
                            "variable '{}' declared as {:?}, but value has type {:?}",
                            name, declared_type, value_type
                        )));
                    }
                }

                self.define(name.clone(), ty.clone().unwrap_or(value_type))?;

                Ok(())
            }

            Statement::Return(expression) => {
                let actual_type = match expression {
                    Some(expression) => self.check_expression(expression)?,
                    None => Type::Unit,
                };

                if &actual_type != expected_return {
                    return Err(TypeError::new(format!(
                        "expected return type {:?}, found {:?}",
                        expected_return, actual_type
                    )));
                }

                Ok(())
            }

            Statement::Expression(expression) => {
                self.check_expression(expression)?;
                Ok(())
            }
        }
    }

    fn check_expression(&mut self, expression: &Expression) -> Result<Type, TypeError> {
        match expression {
            Expression::Integer(_) => Ok(Type::UInt(64)),

            Expression::Boolean(_) => Ok(Type::Bool),

            Expression::Variable(name) => self.lookup(name),

            Expression::Call {
                function,
                arguments,
            } => {
                let signature =
                    self.functions.get(function).cloned().ok_or_else(|| {
                        TypeError::new(format!("unknown function '{}'", function))
                    })?;

                if arguments.len() != signature.parameters.len() {
                    return Err(TypeError::new(format!(
                        "function '{}' expects {} arguments, found {}",
                        function,
                        signature.parameters.len(),
                        arguments.len()
                    )));
                }

                for (index, (argument, expected_type)) in arguments
                    .iter()
                    .zip(signature.parameters.iter())
                    .enumerate()
                {
                    let actual_type = self.check_expression(argument)?;

                    let literal_compatible = matches!(argument, Expression::Integer(_))
                        && is_numeric_type(expected_type);

                    if actual_type != *expected_type && !literal_compatible {
                        return Err(TypeError::new(format!(
                            "argument {} of function '{}' expects {:?}, found {:?}",
                            index + 1,
                            function,
                            expected_type,
                            actual_type
                        )));
                    }
                }

                Ok(signature.return_type)
            }

            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.check_expression(left)?;
                let right_type = self.check_expression(right)?;

                if matches!(left.as_ref(), Expression::Integer(_)) && is_numeric_type(&right_type) {
                    return self.check_binary_operation(right_type.clone(), *operator, right_type);
                }

                if matches!(right.as_ref(), Expression::Integer(_)) && is_numeric_type(&left_type) {
                    return self.check_binary_operation(left_type.clone(), *operator, left_type);
                }

                self.check_binary_operation(left_type, *operator, right_type)
            }

            Expression::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_type = self.check_expression(condition)?;

                if !is_boolean_type(&condition_type) {
                    return Err(TypeError::new(format!(
                        "if condition must be boolean, found {:?}",
                        condition_type
                    )));
                }

                let then_type = self.block_result_type(then_branch)?;

                if let Some(else_branch) = else_branch {
                    let else_type = self.block_result_type(else_branch)?;

                    if then_type != else_type {
                        return Err(TypeError::new(format!(
                            "if branches have incompatible types: {:?} and {:?}",
                            then_type, else_type
                        )));
                    }

                    Ok(then_type)
                } else {
                    Ok(Type::Unit)
                }
            }
        }
    }

    fn block_result_type(&mut self, block: &Block) -> Result<Type, TypeError> {
        if let Some(statement) = block.statements.last() {
            match statement {
                Statement::Expression(expression) => self.check_expression(expression),

                Statement::Return(expression) => match expression {
                    Some(expression) => self.check_expression(expression),
                    None => Ok(Type::Unit),
                },

                Statement::Let { .. } => Ok(Type::Unit),
            }
        } else {
            Ok(Type::Unit)
        }
    }

    fn check_binary_operation(
        &self,
        left: Type,
        operator: BinaryOperator,
        right: Type,
    ) -> Result<Type, TypeError> {
        if left != right {
            return Err(TypeError::new(format!(
                "binary operation requires matching types, found {:?} and {:?}",
                left, right
            )));
        }

        match operator {
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply => {
                if is_numeric_type(&left) {
                    Ok(left)
                } else {
                    Err(TypeError::new(format!(
                        "arithmetic is not supported for {:?}",
                        left
                    )))
                }
            }

            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual => {
                if is_comparable_type(&left) {
                    Ok(boolean_result_type(&left))
                } else {
                    Err(TypeError::new(format!(
                        "comparison is not supported for {:?}",
                        left
                    )))
                }
            }
        }
    }

    fn define_variable(&mut self, parameter: &Parameter) -> Result<(), TypeError> {
        self.define(parameter.name.clone(), parameter.ty.clone())
    }

    fn define(&mut self, name: String, ty: Type) -> Result<(), TypeError> {
        let scope = self.scopes.last_mut().unwrap();

        if scope.contains_key(&name) {
            return Err(TypeError::new(format!("duplicate variable '{}'", name)));
        }

        scope.insert(name, ty);

        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<Type, TypeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Ok(ty.clone());
            }
        }

        Err(TypeError::new(format!("undefined variable '{}'", name)))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

fn is_numeric_type(ty: &Type) -> bool {
    match ty {
        Type::UInt(_) | Type::Int(_) => true,

        Type::Encrypted(inner) => is_numeric_type(inner),
        Type::Plaintext(inner) => is_numeric_type(inner),

        _ => false,
    }
}

fn is_comparable_type(ty: &Type) -> bool {
    match ty {
        Type::Bool | Type::UInt(_) | Type::Int(_) => true,

        Type::Encrypted(inner) => is_comparable_type(inner),
        Type::Plaintext(inner) => is_comparable_type(inner),

        _ => false,
    }
}

fn is_boolean_type(ty: &Type) -> bool {
    match ty {
        Type::Bool => true,

        Type::Encrypted(inner) => {
            matches!(inner.as_ref(), Type::Bool)
        }

        _ => false,
    }
}

fn boolean_result_type(ty: &Type) -> Type {
    match ty {
        Type::Encrypted(_) => Type::Encrypted(Box::new(Type::Bool)),

        Type::Plaintext(_) => Type::Plaintext(Box::new(Type::Bool)),

        _ => Type::Bool,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check(source: &str) -> Result<(), TypeError> {
        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        TypeChecker::new().check(&program)
    }

    #[test]
    fn accepts_simple_function() {
        let source = r#"
            fn main(a: u8) -> u8 {
                return a;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn rejects_undefined_variable() {
        let source = r#"
            fn main(a: u8) -> u8 {
                return b;
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn accepts_encrypted_addition() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_encrypted_comparison() {
        let source = r#"
            fn compare(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<bool> {
                return a > b;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn rejects_mismatched_addition() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: u8
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn accepts_encrypted_addition_with_integer_literal() {
        let source = r#"
            fn add(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a + 5;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_integer_literal_plus_encrypted_value() {
        let source = r#"
            fn add(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return 5 + a;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_encrypted_subtraction_with_integer_literal() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a - 5;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_encrypted_multiplication_with_integer_literal() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a * 5;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_encrypted_comparison_with_integer_literal() {
        let source = r#"
            fn compare(
                a: encrypted<u8>
            ) -> encrypted<bool> {
                return a > 5;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_plaintext_arithmetic_with_integer_literal() {
        let source = r#"
            fn add(
                a: plaintext<u16>
            ) -> plaintext<u16> {
                return a + 5;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn rejects_encrypted_plaintext_addition() {
        let source = r#"
            fn invalid(
                a: encrypted<u8>,
                b: plaintext<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn rejects_plaintext_encrypted_addition() {
        let source = r#"
            fn invalid(
                a: plaintext<u8>,
                b: encrypted<u8>
            ) -> plaintext<u8> {
                return a + b;
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn rejects_encrypted_plaintext_comparison() {
        let source = r#"
            fn invalid(
                a: encrypted<u8>,
                b: plaintext<u8>
            ) -> encrypted<bool> {
                return a > b;
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn rejects_encrypted_unwrapped_addition() {
        let source = r#"
            fn invalid(
                a: encrypted<u8>,
                b: u8
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        assert!(check(source).is_err());
    }
    #[test]
    fn accepts_valid_function_call() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn main(
                x: encrypted<u8>,
                y: encrypted<u8>
            ) -> encrypted<u8> {
                return add(x, y);
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_call_to_function_declared_later() {
        let source = r#"
            fn main(
                x: encrypted<u8>,
                y: encrypted<u8>
            ) -> encrypted<u8> {
                return add(x, y);
            }

            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn accepts_function_call_with_numeric_literal() {
        let source = r#"
            fn add_five(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a + 5;
            }

            fn main(
                x: encrypted<u8>
            ) -> encrypted<u8> {
                return add_five(x);
            }
        "#;

        assert!(check(source).is_ok());
    }

    #[test]
    fn rejects_unknown_function_call() {
        let source = r#"
            fn main(
                x: encrypted<u8>
            ) -> encrypted<u8> {
                return missing(x);
            }
        "#;

        let error = check(source).expect_err("unknown function should fail");

        assert!(error.message.contains("unknown function 'missing'"));
    }

    #[test]
    fn rejects_function_call_with_too_few_arguments() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn main(
                x: encrypted<u8>
            ) -> encrypted<u8> {
                return add(x);
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn rejects_function_call_with_too_many_arguments() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn main(
                x: encrypted<u8>,
                y: encrypted<u8>,
                z: encrypted<u8>
            ) -> encrypted<u8> {
                return add(x, y, z);
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn rejects_function_call_argument_type_mismatch() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn main(
                x: encrypted<u8>,
                y: plaintext<u8>
            ) -> encrypted<u8> {
                return add(x, y);
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn function_call_return_type_flows_into_expression() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn main(
                x: encrypted<u8>,
                y: encrypted<u8>
            ) -> encrypted<u8> {
                return add(x, y) * 2;
            }
        "#;

        assert!(check(source).is_ok());
    }
    #[test]
    fn accepts_encrypted_max() {
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

        assert!(check(source).is_ok());
    }

    #[test]
    fn rejects_wrong_return_type() {
        let source = r#"
            fn main(a: u8) -> encrypted<u8> {
                return a;
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn rejects_non_boolean_if_condition() {
        let source = r#"
            fn main(a: u8) -> u8 {
                if a {
                    a
                } else {
                    a
                }
            }
        "#;

        assert!(check(source).is_err());
    }

    #[test]
    fn rejects_duplicate_variable() {
        let source = r#"
            fn main(a: u8) -> u8 {
                let x: u8 = a;
                let x: u8 = a;
                return x;
            }
        "#;

        assert!(check(source).is_err());
    }
}
