use std::collections::HashMap;

use fhec_frontend::ast::{
    Block, Expression, Function as AstFunction, Program, Statement, Type as AstType,
};

use fhec_ir::{Function, Module, Operation, Type, Value, ValueId};

use crate::lowering_error::LoweringError;

pub fn lower_program(program: &Program) -> Result<Module, LoweringError> {
    let expanded = crate::inlining::expand_program(program)?;
    validate_program(&expanded)?;
    lower_program_checked(&expanded)
}

fn lower_program_checked(program: &Program) -> Result<Module, LoweringError> {
    let mut module = Module::new();

    for function in &program.functions {
        module.add_function(lower_function(function)?);
    }

    Ok(module)
}

fn validate_program(program: &Program) -> Result<(), LoweringError> {
    for function in &program.functions {
        validate_function(function)?;
    }

    Ok(())
}

fn validate_function(function: &AstFunction) -> Result<(), LoweringError> {
    let mut variables = HashMap::<String, Type>::new();

    for parameter in &function.parameters {
        variables.insert(parameter.name.clone(), lower_type(&parameter.ty));
    }

    for statement in &function.body.statements {
        validate_statement(statement, &mut variables)?;
    }

    Ok(())
}

fn validate_statement(
    statement: &Statement,
    variables: &mut HashMap<String, Type>,
) -> Result<(), LoweringError> {
    match statement {
        Statement::Return(Some(expression)) | Statement::Expression(expression) => {
            validate_expression(expression, variables)?;
        }

        Statement::Return(None) => {}

        Statement::Let { name, ty, value } => {
            let inferred = validate_expression(value, variables)?;

            let value_type = match ty {
                Some(declared) => lower_type(declared),
                None => inferred,
            };

            variables.insert(name.clone(), value_type);
        }
    }

    Ok(())
}

fn validate_expression(
    expression: &Expression,
    variables: &HashMap<String, Type>,
) -> Result<Type, LoweringError> {
    match expression {
        Expression::Integer(_) => Ok(Type::uint(64)),

        Expression::Boolean(_) => Ok(Type::Bool),

        Expression::Variable(name) => variables
            .get(name)
            .cloned()
            .ok_or_else(|| LoweringError::UnresolvedVariable(name.clone())),

        Expression::Call { function, .. } => Err(LoweringError::UnknownFunction(function.clone())),

        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let left_type = validate_expression(left, variables)?;

            validate_expression(right, variables)?;

            match operator {
                fhec_frontend::ast::BinaryOperator::Equal
                | fhec_frontend::ast::BinaryOperator::NotEqual
                | fhec_frontend::ast::BinaryOperator::Greater
                | fhec_frontend::ast::BinaryOperator::GreaterEqual
                | fhec_frontend::ast::BinaryOperator::Less
                | fhec_frontend::ast::BinaryOperator::LessEqual => Ok(Type::Bool),

                _ => Ok(left_type),
            }
        }

        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            validate_expression(condition, variables)?;

            let then_expression = block_value_expression(then_branch, "then")?;

            let else_block = else_branch.as_ref().ok_or(LoweringError::IfWithoutElse)?;

            let else_expression = block_value_expression(else_block, "else")?;

            let then_type = validate_expression(then_expression, variables)?;

            let else_type = validate_expression(else_expression, variables)?;

            if then_type != else_type {
                return Err(LoweringError::IfBranchTypeMismatch {
                    then_type,
                    else_type,
                });
            }

            Ok(then_type)
        }
    }
}

fn block_value_expression<'a>(
    block: &'a Block,
    branch: &'static str,
) -> Result<&'a Expression, LoweringError> {
    let statement = block
        .statements
        .last()
        .ok_or(LoweringError::EmptyValueBlock { branch })?;

    match statement {
        Statement::Expression(expression) | Statement::Return(Some(expression)) => Ok(expression),

        _ => Err(LoweringError::BlockDoesNotProduceValue { branch }),
    }
}

fn lower_function(function: &AstFunction) -> Result<Function, LoweringError> {
    let mut parameters = Vec::new();
    let mut variables = HashMap::new();

    for (index, parameter) in function.parameters.iter().enumerate() {
        let value = Value::new(ValueId(index as u32), lower_type(&parameter.ty));

        variables.insert(parameter.name.clone(), value.clone());

        parameters.push(value);
    }

    let mut ir_function = Function::new(
        function.name.clone(),
        parameters,
        lower_type(&function.return_type),
    );

    for statement in &function.body.statements {
        lower_statement(statement, &mut ir_function, &mut variables)?;
    }

    Ok(ir_function)
}

fn lower_statement(
    statement: &Statement,
    function: &mut Function,
    variables: &mut HashMap<String, Value>,
) -> Result<(), LoweringError> {
    match statement {
        Statement::Return(Some(expression)) => {
            let value = lower_expression(expression, function, variables)?;

            function.push_operation(Operation::Return { value: Some(value) });
        }

        Statement::Return(None) => {
            function.push_operation(Operation::Return { value: None });
        }

        Statement::Let { name, ty, value } => {
            let value_id = lower_expression(value, function, variables)?;

            let value_type = match ty {
                Some(declared_type) => lower_type(declared_type),

                None => expression_type(value, variables)?,
            };

            variables.insert(name.clone(), Value::new(value_id, value_type));
        }

        Statement::Expression(expression) => {
            lower_expression(expression, function, variables)?;
        }
    }

    Ok(())
}

fn lower_expression(
    expression: &Expression,
    function: &mut Function,
    variables: &HashMap<String, Value>,
) -> Result<ValueId, LoweringError> {
    match expression {
        Expression::Integer(value) => {
            let result = function.create_value(Type::uint(64));

            function.push_operation(Operation::Constant {
                result: result.id,
                value: fhec_ir::Constant::UInt {
                    value: *value,
                    width: 64,
                },
            });

            Ok(result.id)
        }

        Expression::Boolean(value) => {
            let result = function.create_value(Type::Bool);

            function.push_operation(Operation::Constant {
                result: result.id,
                value: fhec_ir::Constant::Bool(*value),
            });

            Ok(result.id)
        }

        Expression::Variable(name) => variables
            .get(name)
            .map(|value| value.id)
            .ok_or_else(|| LoweringError::UnresolvedVariable(name.clone())),

        Expression::Call { function, .. } => Err(LoweringError::UnknownFunction(function.clone())),

        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let lhs = lower_expression(left, function, variables)?;

            let rhs = lower_expression(right, function, variables)?;

            let result_type = match operator {
                fhec_frontend::ast::BinaryOperator::Equal
                | fhec_frontend::ast::BinaryOperator::NotEqual
                | fhec_frontend::ast::BinaryOperator::Greater
                | fhec_frontend::ast::BinaryOperator::GreaterEqual
                | fhec_frontend::ast::BinaryOperator::Less
                | fhec_frontend::ast::BinaryOperator::LessEqual => Type::Bool,

                _ => expression_type(left, variables)?,
            };

            let result = function.create_value(result_type);

            let operation = match operator {
                fhec_frontend::ast::BinaryOperator::Add => Operation::Add {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::Subtract => Operation::Sub {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::Multiply => Operation::Mul {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::Equal => Operation::Eq {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::NotEqual => Operation::NotEqual {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::Greater => Operation::GreaterThan {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::GreaterEqual => Operation::GreaterEqual {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::Less => Operation::LessThan {
                    result: result.id,
                    lhs,
                    rhs,
                },

                fhec_frontend::ast::BinaryOperator::LessEqual => Operation::LessEqual {
                    result: result.id,
                    lhs,
                    rhs,
                },
            };

            function.push_operation(operation);

            Ok(result.id)
        }

        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_id = lower_expression(condition, function, variables)?;

            let then_expression = block_value_expression(then_branch, "then")?;

            let else_block = else_branch.as_ref().ok_or(LoweringError::IfWithoutElse)?;

            let else_expression = block_value_expression(else_block, "else")?;

            let then_type = expression_type(then_expression, variables)?;

            let else_type = expression_type(else_expression, variables)?;

            if then_type != else_type {
                return Err(LoweringError::IfBranchTypeMismatch {
                    then_type,
                    else_type,
                });
            }

            let then_value = lower_expression(then_expression, function, variables)?;

            let else_value = lower_expression(else_expression, function, variables)?;

            let result = function.create_value(then_type);

            function.push_operation(Operation::Mux {
                result: result.id,
                condition: condition_id,
                if_true: then_value,
                if_false: else_value,
            });

            Ok(result.id)
        }
    }
}

fn expression_type(
    expression: &Expression,
    variables: &HashMap<String, Value>,
) -> Result<Type, LoweringError> {
    match expression {
        Expression::Integer(_) => Ok(Type::uint(64)),

        Expression::Boolean(_) => Ok(Type::Bool),

        Expression::Variable(name) => variables
            .get(name)
            .map(|value| value.ty.clone())
            .ok_or_else(|| LoweringError::UnresolvedVariable(name.clone())),

        Expression::Call { function, .. } => Err(LoweringError::UnknownFunction(function.clone())),

        Expression::Binary { left, operator, .. } => match operator {
            fhec_frontend::ast::BinaryOperator::Equal
            | fhec_frontend::ast::BinaryOperator::NotEqual
            | fhec_frontend::ast::BinaryOperator::Greater
            | fhec_frontend::ast::BinaryOperator::GreaterEqual
            | fhec_frontend::ast::BinaryOperator::Less
            | fhec_frontend::ast::BinaryOperator::LessEqual => Ok(Type::Bool),

            _ => expression_type(left, variables),
        },

        Expression::If {
            then_branch,
            else_branch,
            ..
        } => {
            let then_expression = block_value_expression(then_branch, "then")?;

            let else_block = else_branch.as_ref().ok_or(LoweringError::IfWithoutElse)?;

            let else_expression = block_value_expression(else_block, "else")?;

            let then_type = expression_type(then_expression, variables)?;

            let else_type = expression_type(else_expression, variables)?;

            if then_type != else_type {
                return Err(LoweringError::IfBranchTypeMismatch {
                    then_type,
                    else_type,
                });
            }

            Ok(then_type)
        }
    }
}

fn lower_type(ty: &AstType) -> Type {
    match ty {
        AstType::Unit => Type::Unit,

        AstType::Bool => Type::Bool,

        AstType::UInt(width) => Type::uint(*width),

        AstType::Int(width) => Type::int(*width),

        AstType::Encrypted(inner) => Type::encrypted(lower_type(inner)),

        AstType::Plaintext(inner) => Type::plaintext(lower_type(inner)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use fhec_frontend::lexer::Lexer;
    use fhec_frontend::parser::Parser;

    #[test]
    fn unresolved_variable_returns_error() {
        let source = r#"
            fn bad() -> u8 {
                return missing;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let error = lower_program(&program).expect_err("expected lowering error");

        assert!(matches!(
            error,
            LoweringError::UnresolvedVariable(
                ref name
            ) if name == "missing"
        ));
    }

    #[test]
    fn if_without_else_returns_error() {
        let source = r#"
            fn bad(
                condition: bool,
                a: u8
            ) -> u8 {
                return if condition {
                    a
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let error = lower_program(&program).expect_err("expected lowering error");

        assert_eq!(error, LoweringError::IfWithoutElse);
    }

    #[test]
    fn mismatched_if_branch_types_return_error() {
        let source = r#"
            fn bad(
                condition: bool,
                a: u8
            ) -> u8 {
                return if condition {
                    a
                } else {
                    a == a
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let error = lower_program(&program).expect_err("expected lowering error");

        assert!(
            matches!(error, LoweringError::IfBranchTypeMismatch { .. }),
            "unexpected lowering error: {error:?}"
        );
    }
    #[test]
    fn lowers_encrypted_addition() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        assert_eq!(module.function_count(), 1);

        let function = &module.functions[0];

        assert_eq!(function.name, "add");
        assert_eq!(function.parameters.len(), 2);

        assert!(function.operations.iter().any(|operation| {
            matches!(
                operation,
                Operation::Add {
                    lhs: ValueId(0),
                    rhs: ValueId(1),
                    ..
                }
            )
        }));
    }

    #[test]
    fn encrypted_arithmetic_preserves_encrypted_type() {
        let source = r#"
            fn add_constant(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a + 5;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let add_result = function
            .operations
            .iter()
            .find_map(|operation| match operation {
                Operation::Add { result, .. } => Some(*result),
                _ => None,
            })
            .expect("expected Add operation");

        assert_eq!(
            function.value_type(add_result),
            Some(&Type::encrypted(Type::uint(8)))
        );
    }

    #[test]
    fn encrypted_subtraction_preserves_encrypted_type() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let sub_result = function
            .operations
            .iter()
            .find_map(|operation| match operation {
                Operation::Sub { result, .. } => Some(*result),

                _ => None,
            })
            .expect("expected Sub operation");

        assert_eq!(
            function.value_type(sub_result),
            Some(&Type::encrypted(Type::uint(8)))
        );
    }
    #[test]
    fn encrypted_multiplication_preserves_encrypted_type() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let mul_result = function
            .operations
            .iter()
            .find_map(|operation| match operation {
                Operation::Mul { result, .. } => Some(*result),

                _ => None,
            })
            .expect("expected Mul operation");

        assert_eq!(
            function.value_type(mul_result),
            Some(&Type::encrypted(Type::uint(8)))
        );
    }
    #[test]
    fn lowers_multiple_functions_independently() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn subtract(
                x: encrypted<u8>,
                y: encrypted<u8>
            ) -> encrypted<u8> {
                return x - y;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        assert_eq!(module.functions.len(), 2);

        assert_eq!(module.functions[0].name, "add");
        assert_eq!(module.functions[1].name, "subtract");

        assert!(
            module.functions[0]
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Add { .. }))
        );

        assert!(
            module.functions[1]
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Sub { .. }))
        );
    }

    #[test]
    fn function_parameter_scopes_are_independent() {
        let source = r#"
            fn first(
                value: encrypted<u8>
            ) -> encrypted<u8> {
                return value + 1;
            }

            fn second(
                value: encrypted<u8>
            ) -> encrypted<u8> {
                return value * 2;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        assert_eq!(module.functions.len(), 2);

        for function in &module.functions {
            assert_eq!(function.parameters.len(), 1);
            assert_eq!(function.parameters[0].id, ValueId(0));
        }

        assert!(
            module.functions[0]
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Add { .. }))
        );

        assert!(
            module.functions[1]
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Mul { .. }))
        );
    }

    #[test]
    fn function_return_types_remain_independent() {
        let source = r#"
            fn arithmetic(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn compare(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<bool> {
                return a > b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        assert_eq!(
            module.functions[0].return_type,
            Type::encrypted(Type::uint(8))
        );

        assert_eq!(module.functions[1].return_type, Type::encrypted(Type::Bool));
    }
    #[test]
    fn empty_if_branch_returns_error() {
        let source = r#"
            fn invalid(
                condition: encrypted<bool>,
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return if condition {
                } else {
                    a
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let error = lower_program(&program).expect_err("lowering should fail");

        assert!(matches!(
            error,
            LoweringError::EmptyValueBlock { branch: "then" }
        ));
    }

    #[test]
    fn value_less_if_branch_returns_error() {
        let source = r#"
            fn invalid(
                condition: encrypted<bool>,
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return if condition {
                    let temporary = a + 1;
                } else {
                    a
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let error = lower_program(&program).expect_err("lowering should fail");

        assert!(matches!(
            error,
            LoweringError::BlockDoesNotProduceValue { branch: "then" }
        ));
    }

    #[test]
    fn nested_unresolved_variable_returns_error() {
        let source = r#"
            fn invalid(
                condition: encrypted<bool>,
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return if condition {
                    a + missing
                } else {
                    a
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let error = lower_program(&program).expect_err("lowering should fail");

        assert!(matches!(
            error,
            LoweringError::UnresolvedVariable(name)
                if name == "missing"
        ));
    }
    #[test]
    fn lowers_all_comparison_operations() {
        let cases = [
            ("==", "Eq"),
            ("!=", "NotEqual"),
            (">", "GreaterThan"),
            (">=", "GreaterEqual"),
            ("<", "LessThan"),
            ("<=", "LessEqual"),
        ];

        for (operator, expected_operation) in cases {
            let source = format!(
                r#"
                    fn compare(
                        a: encrypted<u8>,
                        b: encrypted<u8>
                    ) -> bool {{
                        return a {operator} b;
                    }}
                "#
            );

            let tokens = Lexer::new(&source).tokenize().expect("lexing failed");

            let program = Parser::new(tokens).parse().expect("parsing failed");

            let module = lower_program(&program).expect("lowering failed");

            let function = &module.functions[0];

            let comparison = function
                .operations
                .iter()
                .find(|operation| {
                    matches!(
                        operation,
                        Operation::Eq { .. }
                            | Operation::NotEqual { .. }
                            | Operation::GreaterThan { .. }
                            | Operation::GreaterEqual { .. }
                            | Operation::LessThan { .. }
                            | Operation::LessEqual { .. }
                    )
                })
                .expect("expected comparison operation");

            match expected_operation {
                "Eq" => assert!(matches!(comparison, Operation::Eq { .. })),
                "NotEqual" => assert!(matches!(comparison, Operation::NotEqual { .. })),
                "GreaterThan" => assert!(matches!(comparison, Operation::GreaterThan { .. })),
                "GreaterEqual" => assert!(matches!(comparison, Operation::GreaterEqual { .. })),
                "LessThan" => assert!(matches!(comparison, Operation::LessThan { .. })),
                "LessEqual" => assert!(matches!(comparison, Operation::LessEqual { .. })),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn lowers_if_into_mux() {
        let source = r#"
            fn select(
                condition: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return if condition {
                    a
                } else {
                    b
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        assert!(function.operations.iter().any(|operation| {
            matches!(
                operation,
                Operation::Mux {
                    condition: ValueId(0),
                    if_true: ValueId(1),
                    if_false: ValueId(2),
                    ..
                }
            )
        }));
    }

    #[test]
    fn mux_result_preserves_branch_type() {
        let source = r#"
            fn select(
                condition: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return if condition {
                    a
                } else {
                    b
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let mux_result = function
            .operations
            .iter()
            .find_map(|operation| match operation {
                Operation::Mux { result, .. } => Some(*result),

                _ => None,
            })
            .expect("expected Mux operation");

        assert_eq!(
            function.value_type(mux_result),
            Some(&Type::encrypted(Type::uint(8)))
        );
    }
}
