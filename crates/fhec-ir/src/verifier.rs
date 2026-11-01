use std::collections::HashSet;

use crate::{Function, Operation, Type, ValueId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationError {
    pub message: String,
}

impl VerificationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Verifies the structural and type invariants of an IR function.
pub fn verify_function(function: &Function) -> Result<(), VerificationError> {
    let mut defined = HashSet::new();

    for parameter in &function.parameters {
        if !defined.insert(parameter.id) {
            return Err(VerificationError::new(format!(
                "duplicate parameter value {:?}",
                parameter.id
            )));
        }
    }

    for operation in &function.operations {
        verify_operation(function, operation, &mut defined)?;
    }

    Ok(())
}

fn verify_operation(
    function: &Function,
    operation: &Operation,
    defined: &mut HashSet<ValueId>,
) -> Result<(), VerificationError> {
    match operation {
        Operation::Argument { result, ty } => {
            define_result(result, ty, function, defined)?;
        }

        Operation::Constant { result, value } => {
            let ty = match value {
                crate::Constant::Bool(_) => Type::Bool,

                crate::Constant::UInt { width, .. } => Type::uint(*width),

                crate::Constant::Int { width, .. } => Type::int(*width),
            };

            define_result(result, &ty, function, defined)?;
        }

        Operation::Copy { result, operand } => {
            let operand_type = require_value(function, *operand, defined)?;

            define_result(result, operand_type, function, defined)?;
        }

        Operation::Add { result, lhs, rhs }
        | Operation::Sub { result, lhs, rhs }
        | Operation::Mul { result, lhs, rhs } => {
            let lhs_type = require_value(function, *lhs, defined)?;

            let rhs_type = require_value(function, *rhs, defined)?;

            if lhs_type != rhs_type {
                return Err(VerificationError::new(format!(
                    "arithmetic operands have different types: {} and {}",
                    lhs_type, rhs_type
                )));
            }

            define_result(result, lhs_type, function, defined)?;
        }

        Operation::Eq { result, lhs, rhs }
        | Operation::NotEqual { result, lhs, rhs }
        | Operation::GreaterThan { result, lhs, rhs }
        | Operation::GreaterEqual { result, lhs, rhs }
        | Operation::LessThan { result, lhs, rhs }
        | Operation::LessEqual { result, lhs, rhs } => {
            let lhs_type = require_value(function, *lhs, defined)?;

            let rhs_type = require_value(function, *rhs, defined)?;

            if lhs_type != rhs_type {
                return Err(VerificationError::new(format!(
                    "comparison operands have different types: {} and {}",
                    lhs_type, rhs_type
                )));
            }

            let result_type = comparison_result_type(lhs_type);

            define_result(result, &result_type, function, defined)?;
        }

        Operation::Not { result, operand } => {
            let operand_type = require_value(function, *operand, defined)?;

            if !is_boolean_type(operand_type) {
                return Err(VerificationError::new(format!(
                    "Not requires a boolean operand, got {}",
                    operand_type
                )));
            }

            define_result(result, operand_type, function, defined)?;
        }

        Operation::And { result, lhs, rhs }
        | Operation::Or { result, lhs, rhs }
        | Operation::Xor { result, lhs, rhs } => {
            let lhs_type = require_value(function, *lhs, defined)?;

            let rhs_type = require_value(function, *rhs, defined)?;

            if lhs_type != rhs_type {
                return Err(VerificationError::new(format!(
                    "logical operands have different types: {} and {}",
                    lhs_type, rhs_type
                )));
            }

            if !is_boolean_type(lhs_type) {
                return Err(VerificationError::new(format!(
                    "logical operation requires boolean operands, got {}",
                    lhs_type
                )));
            }

            define_result(result, lhs_type, function, defined)?;
        }

        Operation::Mux {
            result,
            condition,
            if_true,
            if_false,
        } => {
            let condition_type = require_value(function, *condition, defined)?;

            if !is_boolean_type(condition_type) {
                return Err(VerificationError::new(format!(
                    "Mux condition must be boolean, got {}",
                    condition_type
                )));
            }

            let true_type = require_value(function, *if_true, defined)?;

            let false_type = require_value(function, *if_false, defined)?;

            if true_type != false_type {
                return Err(VerificationError::new(format!(
                    "Mux branches have different types: {} and {}",
                    true_type, false_type
                )));
            }

            define_result(result, true_type, function, defined)?;
        }

        Operation::Return { value } => match value {
            Some(value_id) => {
                let value_type = require_value(function, *value_id, defined)?;

                if value_type != &function.return_type {
                    return Err(VerificationError::new(format!(
                        "return type mismatch: expected {}, got {}",
                        function.return_type, value_type
                    )));
                }
            }

            None => {
                if function.return_type != Type::Unit {
                    return Err(VerificationError::new(format!(
                        "function returns {}, but return has no value",
                        function.return_type
                    )));
                }
            }
        },
    }

    Ok(())
}

fn require_value<'a>(
    function: &'a Function,
    id: ValueId,
    defined: &HashSet<ValueId>,
) -> Result<&'a Type, VerificationError> {
    if !defined.contains(&id) {
        return Err(VerificationError::new(format!(
            "use of undefined value {:?}",
            id
        )));
    }

    function
        .value_type(id)
        .ok_or_else(|| VerificationError::new(format!("value {:?} has no registered type", id)))
}

fn define_result(
    id: &ValueId,
    ty: &Type,
    function: &Function,
    defined: &mut HashSet<ValueId>,
) -> Result<(), VerificationError> {
    if !defined.insert(*id) {
        return Err(VerificationError::new(format!(
            "value {:?} is defined more than once",
            id
        )));
    }

    let registered_type = function
        .value_type(*id)
        .ok_or_else(|| VerificationError::new(format!("value {:?} has no registered type", id)))?;

    if registered_type != ty {
        return Err(VerificationError::new(format!(
            "value {:?} has type {}, but operation defines it as {}",
            id, registered_type, ty
        )));
    }

    Ok(())
}

fn is_boolean_type(ty: &Type) -> bool {
    match ty {
        Type::Bool => true,
        Type::Encrypted(inner) => **inner == Type::Bool,
        _ => false,
    }
}

fn comparison_result_type(ty: &Type) -> Type {
    match ty {
        Type::Encrypted(_) => Type::encrypted(Type::Bool),
        _ => Type::Bool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Constant, Function, Operation, Value};

    fn function_with_parameters(parameters: Vec<Value>, return_type: Type) -> Function {
        Function::new("test", parameters, return_type)
    }

    #[test]
    fn copy_preserves_type() {
        let mut function =
            function_with_parameters(vec![Value::new(ValueId(0), Type::Bool)], Type::Bool);

        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Copy {
            result: result.id,
            operand: ValueId(0),
        });

        function.push_operation(Operation::Return {
            value: Some(result.id),
        });

        assert!(verify_function(&function).is_ok());
    }

    #[test]
    fn valid_encrypted_mux_is_accepted() {
        let mut function = function_with_parameters(
            vec![
                Value::new(ValueId(0), Type::encrypted(Type::Bool)),
                Value::new(ValueId(1), Type::encrypted(Type::uint(8))),
                Value::new(ValueId(2), Type::encrypted(Type::uint(8))),
            ],
            Type::encrypted(Type::uint(8)),
        );

        let result = function.create_value(Type::encrypted(Type::uint(8)));

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: ValueId(0),
            if_true: ValueId(1),
            if_false: ValueId(2),
        });

        function.push_operation(Operation::Return {
            value: Some(result.id),
        });

        assert!(verify_function(&function).is_ok());
    }

    #[test]
    fn mux_rejects_non_boolean_condition() {
        let mut function = function_with_parameters(
            vec![
                Value::new(ValueId(0), Type::encrypted(Type::uint(8))),
                Value::new(ValueId(1), Type::encrypted(Type::uint(8))),
                Value::new(ValueId(2), Type::encrypted(Type::uint(8))),
            ],
            Type::encrypted(Type::uint(8)),
        );

        let result = function.create_value(Type::encrypted(Type::uint(8)));

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: ValueId(0),
            if_true: ValueId(1),
            if_false: ValueId(2),
        });

        let error =
            verify_function(&function).expect_err("Mux should reject non-boolean condition");

        assert!(error.message.contains("boolean"));
    }

    #[test]
    fn mux_rejects_different_branch_types() {
        let mut function = function_with_parameters(
            vec![
                Value::new(ValueId(0), Type::encrypted(Type::Bool)),
                Value::new(ValueId(1), Type::encrypted(Type::uint(8))),
                Value::new(ValueId(2), Type::encrypted(Type::uint(16))),
            ],
            Type::encrypted(Type::uint(8)),
        );

        let result = function.create_value(Type::encrypted(Type::uint(8)));

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: ValueId(0),
            if_true: ValueId(1),
            if_false: ValueId(2),
        });

        let error = verify_function(&function).expect_err("Mux should reject mismatched branches");

        assert!(error.message.contains("different types"));
    }

    #[test]
    fn encrypted_comparison_produces_encrypted_boolean() {
        let mut function = function_with_parameters(
            vec![
                Value::new(ValueId(0), Type::encrypted(Type::uint(8))),
                Value::new(ValueId(1), Type::encrypted(Type::uint(8))),
            ],
            Type::encrypted(Type::Bool),
        );

        let result = function.create_value(Type::encrypted(Type::Bool));

        function.push_operation(Operation::Eq {
            result: result.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::Return {
            value: Some(result.id),
        });

        assert!(verify_function(&function).is_ok());
    }

    #[test]
    fn plaintext_comparison_produces_plain_boolean() {
        let mut function = function_with_parameters(
            vec![
                Value::new(ValueId(0), Type::uint(8)),
                Value::new(ValueId(1), Type::uint(8)),
            ],
            Type::Bool,
        );

        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Eq {
            result: result.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::Return {
            value: Some(result.id),
        });

        assert!(verify_function(&function).is_ok());
    }

    #[test]
    fn return_type_is_checked() {
        let mut function = function_with_parameters(
            vec![Value::new(ValueId(0), Type::encrypted(Type::uint(8)))],
            Type::Bool,
        );

        function.push_operation(Operation::Return {
            value: Some(ValueId(0)),
        });

        let error =
            verify_function(&function).expect_err("return type mismatch should be rejected");

        assert!(error.message.contains("return type mismatch"));
    }

    #[test]
    fn constants_are_verified() {
        let mut function = function_with_parameters(Vec::new(), Type::uint(8));

        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Constant {
            result: result.id,
            value: Constant::UInt {
                value: 42,
                width: 8,
            },
        });

        function.push_operation(Operation::Return {
            value: Some(result.id),
        });

        assert!(verify_function(&function).is_ok());
    }
}
