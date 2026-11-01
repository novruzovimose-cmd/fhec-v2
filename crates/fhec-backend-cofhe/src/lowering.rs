use std::collections::HashMap;

use fhec_frontend::ast::{BinaryOperator, Block, Expression, Function, Statement};

use super::emitter::{CoFheError, check_identifier};
use super::type_map::CoType;

#[derive(Clone)]
pub(crate) struct Signature {
    pub(crate) parameters: Vec<CoType>,
    pub(crate) result: CoType,
}

struct Value {
    code: String,
    ty: CoType,
}

pub(crate) fn lower_function(
    function: &Function,
    signatures: &HashMap<String, Signature>,
) -> Result<String, CoFheError> {
    let signature = signatures
        .get(&function.name)
        .ok_or_else(|| CoFheError(format!("Missing function signature: {}", function.name)))?;
    let mut scope = HashMap::new();
    let mut parameters = Vec::new();

    for (parameter, ty) in function.parameters.iter().zip(signature.parameters.iter()) {
        check_identifier(&parameter.name)?;
        scope.insert(parameter.name.clone(), *ty);
        parameters.push(format!("{} {}", ty.solidity(), parameter.name));
    }

    let mut body = String::new();
    let mut returned = false;
    for statement in &function.body.statements {
        if returned {
            return Err(CoFheError(format!(
                "Unreachable statement after return in '{}'",
                function.name
            )));
        }
        match statement {
            Statement::Let { name, ty, value } => {
                check_identifier(name)?;
                if scope.contains_key(name) {
                    return Err(CoFheError(format!("Duplicate variable '{name}'")));
                }
                let expected = ty.as_ref().map(CoType::from_ast).transpose()?;
                let lowered = lower_expression(value, &scope, signatures, expected)?;
                if let Some(expected) = expected {
                    if lowered.ty != expected {
                        return Err(CoFheError(format!("Type mismatch for variable '{name}'")));
                    }
                }
                body.push_str(&format!(
                    "        {} {name} = {};\n",
                    lowered.ty.solidity(),
                    lowered.code
                ));
                scope.insert(name.clone(), lowered.ty);
            }
            Statement::Return(Some(value)) => {
                let lowered = lower_expression(value, &scope, signatures, Some(signature.result))?;
                if lowered.ty != signature.result {
                    return Err(CoFheError(format!(
                        "Wrong return type in '{}'",
                        function.name
                    )));
                }
                body.push_str(&format!("        return {};\n", lowered.code));
                returned = true;
            }
            Statement::Return(None) => {
                return Err(CoFheError(format!(
                    "Encrypted function '{}' must return a value",
                    function.name
                )));
            }
            Statement::Expression(_) => {
                return Err(CoFheError(format!(
                    "Standalone expression statements are not supported in '{}'",
                    function.name
                )));
            }
        }
    }

    if !returned {
        return Err(CoFheError(format!(
            "Function '{}' requires an explicit return",
            function.name
        )));
    }

    Ok(format!(
        "    function {}({}) internal returns ({}) {{\n{}    }}\n\n",
        function.name,
        parameters.join(", "),
        signature.result.solidity(),
        body
    ))
}

fn lower_expression(
    expression: &Expression,
    scope: &HashMap<String, CoType>,
    signatures: &HashMap<String, Signature>,
    expected: Option<CoType>,
) -> Result<Value, CoFheError> {
    match expression {
        Expression::Variable(name) => {
            check_identifier(name)?;
            let ty = *scope
                .get(name)
                .ok_or_else(|| CoFheError(format!("Unknown variable '{name}'")))?;
            Ok(Value {
                code: name.clone(),
                ty,
            })
        }
        Expression::Integer(value) => {
            let ty = expected.ok_or_else(|| {
                CoFheError("Numeric literal needs an encrypted integer type context".into())
            })?;
            Ok(Value {
                code: ty.literal(*value)?,
                ty,
            })
        }
        Expression::Boolean(value) => {
            if expected != Some(CoType::Bool) {
                return Err(CoFheError(
                    "Boolean literal needs an encrypted<bool> context".into(),
                ));
            }
            Ok(Value {
                code: format!("FHE.asEbool({value})"),
                ty: CoType::Bool,
            })
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let left_expected = if matches!(
                left.as_ref(),
                Expression::Integer(_) | Expression::Boolean(_)
            ) {
                None
            } else {
                expected
            };
            let right_expected = if matches!(
                right.as_ref(),
                Expression::Integer(_) | Expression::Boolean(_)
            ) {
                None
            } else {
                expected
            };

            let (lhs, rhs) = if matches!(
                left.as_ref(),
                Expression::Integer(_) | Expression::Boolean(_)
            ) {
                let rhs = lower_expression(right, scope, signatures, right_expected)?;
                let lhs = lower_expression(left, scope, signatures, Some(rhs.ty))?;
                (lhs, rhs)
            } else {
                let lhs = lower_expression(left, scope, signatures, left_expected)?;
                let rhs = lower_expression(right, scope, signatures, Some(lhs.ty))?;
                (lhs, rhs)
            };

            if lhs.ty != rhs.ty {
                return Err(CoFheError(
                    "CoFHE binary operands must have matching encrypted types".into(),
                ));
            }

            let (op, result) = match operator {
                BinaryOperator::Add => ("add", lhs.ty),
                BinaryOperator::Subtract => ("sub", lhs.ty),
                BinaryOperator::Multiply => ("mul", lhs.ty),
                BinaryOperator::Equal => ("eq", CoType::Bool),
                BinaryOperator::NotEqual => ("ne", CoType::Bool),
                BinaryOperator::Greater => ("gt", CoType::Bool),
                BinaryOperator::GreaterEqual => ("ge", CoType::Bool),
                BinaryOperator::Less => ("lt", CoType::Bool),
                BinaryOperator::LessEqual => ("le", CoType::Bool),
            };

            if !matches!(lhs.ty, CoType::Uint(_)) {
                return Err(CoFheError(format!(
                    "Operator {op} requires encrypted unsigned integers"
                )));
            }

            Ok(Value {
                code: format!("FHE.{op}({}, {})", lhs.code, rhs.code),
                ty: result,
            })
        }
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = lower_expression(condition, scope, signatures, Some(CoType::Bool))?;
            if condition.ty != CoType::Bool {
                return Err(CoFheError(
                    "Encrypted selection condition must have type encrypted<bool>".into(),
                ));
            }
            let else_branch = else_branch
                .as_ref()
                .ok_or_else(|| CoFheError("CoFHE selection requires an else branch".into()))?;
            let then_expr =
                lower_expression(block_result(then_branch)?, scope, signatures, expected)?;
            let else_expr = lower_expression(
                block_result(else_branch)?,
                scope,
                signatures,
                Some(then_expr.ty),
            )?;
            if then_expr.ty != else_expr.ty {
                return Err(CoFheError(
                    "CoFHE selection branches must have matching types".into(),
                ));
            }
            Ok(Value {
                code: format!(
                    "FHE.select({}, {}, {})",
                    condition.code, then_expr.code, else_expr.code
                ),
                ty: then_expr.ty,
            })
        }
        Expression::Call {
            function,
            arguments,
        } => {
            check_identifier(function)?;
            let signature = signatures
                .get(function)
                .ok_or_else(|| CoFheError(format!("Unknown function '{function}'")))?;
            if signature.parameters.len() != arguments.len() {
                return Err(CoFheError(format!(
                    "Wrong argument count calling '{function}'"
                )));
            }
            let mut args = Vec::with_capacity(arguments.len());
            for (argument, expected_type) in arguments.iter().zip(signature.parameters.iter()) {
                let lowered = lower_expression(argument, scope, signatures, Some(*expected_type))?;
                if lowered.ty != *expected_type {
                    return Err(CoFheError(format!(
                        "Wrong argument type calling '{function}'"
                    )));
                }
                args.push(lowered.code);
            }
            Ok(Value {
                code: format!("{function}({})", args.join(", ")),
                ty: signature.result,
            })
        }
    }
}

fn block_result(block: &Block) -> Result<&Expression, CoFheError> {
    match block.statements.as_slice() {
        [Statement::Return(Some(expression))] | [Statement::Expression(expression)] => {
            Ok(expression)
        }
        _ => Err(CoFheError(
            "CoFHE selection branches require one result expression".into(),
        )),
    }
}
