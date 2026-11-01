pub mod hints;

#[cfg(test)]
mod release_optimizer_tests;

use std::collections::HashMap;

use fhec_ir::{Constant, Function, Operation, ValueId};

/// Runs all optimizer passes over an IR function.
///
/// The optimizer currently performs:
///
/// ```text
/// 1. Constant folding
/// 2. Boolean simplification
/// 3. Constant folding again
/// ```
///
/// Example constant folding:
///
/// ```text
/// Constant(5)
/// Constant(3)
/// Add
///    ↓
/// Constant(8)
/// ```
///
/// Example boolean simplification:
///
/// ```text
/// x AND true
///     ↓
/// x
/// ```
pub fn optimize_function(function: &mut Function) {
    constant_fold(function);
    boolean_optimize(function);
    constant_fold(function);
}

pub fn optimize_function_with_plan(function: &mut Function, plan: &hints::OptimizationPlan) {
    if plan.has_action(hints::OptimizationAction::ConstantFold) {
        constant_fold(function);
    }

    if plan.has_action(hints::OptimizationAction::BooleanSimplify) {
        boolean_optimize(function);
    }

    if plan.actions.iter().any(|planned| {
        planned.action == hints::OptimizationAction::ParallelismExposure
            && planned.currently_supported
    }) {
        parallelism_expose(function);
    }
    if plan.actions.iter().any(|planned| {
        planned.action == hints::OptimizationAction::DepthReduction && planned.currently_supported
    }) {
        depth_reduce(function);
    }
    if plan.actions.iter().any(|planned| {
        planned.action == hints::OptimizationAction::CryptoOperationReduction
            && planned.currently_supported
    }) {
        crypto_operation_reduce(function);
    }

    if plan.actions.iter().any(|planned| {
        planned.action == hints::OptimizationAction::ExpensiveGateLowering
            && planned.currently_supported
    }) {
        mux_optimize(function);
    }

    constant_fold(function);
}

/// Performs constant folding over the function's operations.
///
/// Constant arithmetic and boolean expressions are evaluated at
/// compile time. Encrypted runtime values are never evaluated by
/// this pass.
fn constant_fold(function: &mut Function) {
    let mut constants = HashMap::<ValueId, Constant>::new();

    for index in 0..function.operations.len() {
        let replacement = {
            let operation = &function.operations[index];

            match operation {
                Operation::Constant { result, value } => {
                    constants.insert(*result, value.clone());
                    None
                }

                Operation::Add { result, lhs, rhs } => {
                    fold_add(constants.get(lhs), constants.get(rhs)).map(|value| (*result, value))
                }

                Operation::Sub { result, lhs, rhs } => {
                    fold_sub(constants.get(lhs), constants.get(rhs)).map(|value| (*result, value))
                }

                Operation::Mul { result, lhs, rhs } => {
                    fold_mul(constants.get(lhs), constants.get(rhs)).map(|value| (*result, value))
                }

                Operation::Not { result, operand } => {
                    fold_not(constants.get(operand)).map(|value| (*result, value))
                }

                Operation::And { result, lhs, rhs } => {
                    fold_and(constants.get(lhs), constants.get(rhs)).map(|value| (*result, value))
                }

                Operation::Or { result, lhs, rhs } => {
                    fold_or(constants.get(lhs), constants.get(rhs)).map(|value| (*result, value))
                }

                Operation::Xor { result, lhs, rhs } => {
                    fold_xor(constants.get(lhs), constants.get(rhs)).map(|value| (*result, value))
                }

                _ => None,
            }
        };

        if let Some((result, value)) = replacement {
            constants.insert(result, value.clone());

            function.operations[index] = Operation::Constant { result, value };
        }
    }
}

fn truncate_fold_unsigned(value: u64, width: u32) -> u64 {
    match width {
        0 => 0,
        1..=63 => value & ((1u64 << width) - 1),
        _ => value,
    }
}

fn truncate_fold_signed(value: i64, width: u32) -> i64 {
    match width {
        0 => 0,
        1..=63 => {
            let shift = 64 - width;
            (value << shift) >> shift
        }
        _ => value,
    }
}

fn fold_add(lhs: Option<&Constant>, rhs: Option<&Constant>) -> Option<Constant> {
    match (lhs, rhs) {
        (
            Some(Constant::UInt {
                value: lhs_value,
                width: lhs_width,
            }),
            Some(Constant::UInt {
                value: rhs_value,
                width: rhs_width,
            }),
        ) if lhs_width == rhs_width => Some(Constant::UInt {
            value: truncate_fold_unsigned(lhs_value.wrapping_add(*rhs_value), *lhs_width),
            width: *lhs_width,
        }),

        (
            Some(Constant::Int {
                value: lhs_value,
                width: lhs_width,
            }),
            Some(Constant::Int {
                value: rhs_value,
                width: rhs_width,
            }),
        ) if lhs_width == rhs_width => Some(Constant::Int {
            value: truncate_fold_signed(lhs_value.wrapping_add(*rhs_value), *lhs_width),
            width: *lhs_width,
        }),

        _ => None,
    }
}

fn fold_sub(lhs: Option<&Constant>, rhs: Option<&Constant>) -> Option<Constant> {
    match (lhs, rhs) {
        (
            Some(Constant::UInt {
                value: lhs_value,
                width: lhs_width,
            }),
            Some(Constant::UInt {
                value: rhs_value,
                width: rhs_width,
            }),
        ) if lhs_width == rhs_width => Some(Constant::UInt {
            value: truncate_fold_unsigned(lhs_value.wrapping_sub(*rhs_value), *lhs_width),
            width: *lhs_width,
        }),

        (
            Some(Constant::Int {
                value: lhs_value,
                width: lhs_width,
            }),
            Some(Constant::Int {
                value: rhs_value,
                width: rhs_width,
            }),
        ) if lhs_width == rhs_width => Some(Constant::Int {
            value: truncate_fold_signed(lhs_value.wrapping_sub(*rhs_value), *lhs_width),
            width: *lhs_width,
        }),

        _ => None,
    }
}

fn fold_mul(lhs: Option<&Constant>, rhs: Option<&Constant>) -> Option<Constant> {
    match (lhs, rhs) {
        (
            Some(Constant::UInt {
                value: lhs_value,
                width: lhs_width,
            }),
            Some(Constant::UInt {
                value: rhs_value,
                width: rhs_width,
            }),
        ) if lhs_width == rhs_width => Some(Constant::UInt {
            value: truncate_fold_unsigned(lhs_value.wrapping_mul(*rhs_value), *lhs_width),
            width: *lhs_width,
        }),

        (
            Some(Constant::Int {
                value: lhs_value,
                width: lhs_width,
            }),
            Some(Constant::Int {
                value: rhs_value,
                width: rhs_width,
            }),
        ) if lhs_width == rhs_width => Some(Constant::Int {
            value: truncate_fold_signed(lhs_value.wrapping_mul(*rhs_value), *lhs_width),
            width: *lhs_width,
        }),

        _ => None,
    }
}

fn fold_not(operand: Option<&Constant>) -> Option<Constant> {
    match operand {
        Some(Constant::Bool(value)) => Some(Constant::Bool(!value)),
        _ => None,
    }
}

fn fold_and(lhs: Option<&Constant>, rhs: Option<&Constant>) -> Option<Constant> {
    match (lhs, rhs) {
        (Some(Constant::Bool(lhs_value)), Some(Constant::Bool(rhs_value))) => {
            Some(Constant::Bool(*lhs_value && *rhs_value))
        }
        _ => None,
    }
}

fn fold_or(lhs: Option<&Constant>, rhs: Option<&Constant>) -> Option<Constant> {
    match (lhs, rhs) {
        (Some(Constant::Bool(lhs_value)), Some(Constant::Bool(rhs_value))) => {
            Some(Constant::Bool(*lhs_value || *rhs_value))
        }
        _ => None,
    }
}

fn fold_xor(lhs: Option<&Constant>, rhs: Option<&Constant>) -> Option<Constant> {
    match (lhs, rhs) {
        (Some(Constant::Bool(lhs_value)), Some(Constant::Bool(rhs_value))) => {
            Some(Constant::Bool(*lhs_value ^ *rhs_value))
        }
        _ => None,
    }
}

/// Performs boolean simplification without inspecting encrypted
/// runtime values.
///
/// These transformations are structural and therefore safe:
///
/// ```text
/// x AND true  -> x
/// x AND false -> false
/// x OR false  -> x
/// x OR true   -> true
/// x XOR false -> x
/// x XOR true  -> NOT x
/// ```
fn boolean_optimize(function: &mut Function) {
    let mut replacements = Vec::new();

    for index in 0..function.operations.len() {
        let replacement = match &function.operations[index] {
            Operation::Not { result, operand } => {
                simplify_not(*result, *operand, &function.operations[..index])
            }

            Operation::And { result, lhs, rhs } => {
                simplify_and(*result, *lhs, *rhs, &function.operations[..index])
            }

            Operation::Or { result, lhs, rhs } => {
                simplify_or(*result, *lhs, *rhs, &function.operations[..index])
            }

            Operation::Xor { result, lhs, rhs } => {
                simplify_xor(*result, *lhs, *rhs, &function.operations[..index])
            }

            _ => None,
        };

        if let Some(operation) = replacement {
            replacements.push((index, operation));
        }
    }

    for (index, operation) in replacements {
        function.operations[index] = operation;
    }
}

fn simplify_not(result: ValueId, operand: ValueId, operations: &[Operation]) -> Option<Operation> {
    operations.iter().find_map(|operation| match operation {
        Operation::Not {
            result: inner_result,
            operand: inner_operand,
        } if *inner_result == operand => Some(Operation::Copy {
            result,
            operand: *inner_operand,
        }),

        _ => None,
    })
}
fn simplify_and(
    result: ValueId,
    lhs: ValueId,
    rhs: ValueId,
    operations: &[Operation],
) -> Option<Operation> {
    if lhs == rhs {
        return Some(Operation::Copy {
            result,
            operand: lhs,
        });
    }
    if is_constant_bool(operations, lhs, true) {
        return Some(Operation::Copy {
            result,
            operand: rhs,
        });
    }

    if is_constant_bool(operations, rhs, true) {
        return Some(Operation::Copy {
            result,
            operand: lhs,
        });
    }

    if is_constant_bool(operations, lhs, false) {
        return Some(Operation::Constant {
            result,
            value: Constant::Bool(false),
        });
    }

    if is_constant_bool(operations, rhs, false) {
        return Some(Operation::Constant {
            result,
            value: Constant::Bool(false),
        });
    }

    None
}

fn simplify_or(
    result: ValueId,
    lhs: ValueId,
    rhs: ValueId,
    operations: &[Operation],
) -> Option<Operation> {
    if lhs == rhs {
        return Some(Operation::Copy {
            result,
            operand: lhs,
        });
    }
    if is_constant_bool(operations, lhs, false) {
        return Some(Operation::Copy {
            result,
            operand: rhs,
        });
    }

    if is_constant_bool(operations, rhs, false) {
        return Some(Operation::Copy {
            result,
            operand: lhs,
        });
    }

    if is_constant_bool(operations, lhs, true) {
        return Some(Operation::Constant {
            result,
            value: Constant::Bool(true),
        });
    }

    if is_constant_bool(operations, rhs, true) {
        return Some(Operation::Constant {
            result,
            value: Constant::Bool(true),
        });
    }

    None
}

fn simplify_xor(
    result: ValueId,
    lhs: ValueId,
    rhs: ValueId,
    operations: &[Operation],
) -> Option<Operation> {
    if lhs == rhs {
        return Some(Operation::Constant {
            result,
            value: Constant::Bool(false),
        });
    }
    if is_constant_bool(operations, lhs, false) {
        return Some(Operation::Copy {
            result,
            operand: rhs,
        });
    }

    if is_constant_bool(operations, rhs, false) {
        return Some(Operation::Copy {
            result,
            operand: lhs,
        });
    }

    if is_constant_bool(operations, lhs, true) {
        return Some(Operation::Not {
            result,
            operand: rhs,
        });
    }

    if is_constant_bool(operations, rhs, true) {
        return Some(Operation::Not {
            result,
            operand: lhs,
        });
    }

    None
}

fn is_constant_bool(operations: &[Operation], value_id: ValueId, expected: bool) -> bool {
    operations.iter().any(|operation| {
        matches!(
            operation,
            Operation::Constant {
                result,
                value: Constant::Bool(value),
            } if *result == value_id && *value == expected
        )
    })
}

fn parallelism_expose(function: &mut Function) {
    depth_reduce(function);
}
fn depth_reduce(function: &mut Function) {
    let original = function.operations.clone();
    let mut replacements = Vec::new();
    let mut rewritten = std::collections::HashSet::new();

    for final_index in 0..original.len() {
        let Some((kind, final_result, chain_result, final_other)) =
            associative_binary(&original[final_index])
        else {
            continue;
        };

        let Some(chain_index) = find_operation_result(&original, chain_result) else {
            continue;
        };

        if chain_index >= final_index {
            continue;
        }

        let Some((chain_kind, _, first_result, chain_other)) =
            associative_binary(&original[chain_index])
        else {
            continue;
        };

        if chain_kind != kind {
            continue;
        }

        let Some(first_index) = find_operation_result(&original, first_result) else {
            continue;
        };

        if first_index >= chain_index
            || rewritten.contains(&first_index)
            || rewritten.contains(&chain_index)
            || rewritten.contains(&final_index)
        {
            continue;
        }

        let Some((first_kind, _, first_lhs, first_rhs)) =
            associative_binary(&original[first_index])
        else {
            continue;
        };

        if first_kind != kind {
            continue;
        }

        if value_use_count(&original, chain_result) != 1 {
            continue;
        }

        let available_before_chain = function
            .parameters
            .iter()
            .any(|parameter| parameter.id == final_other)
            || find_operation_result(&original, final_other)
                .is_some_and(|index| index < chain_index);

        if !available_before_chain {
            continue;
        }

        let new_middle = make_binary_operation(kind, chain_result, chain_other, final_other);

        let new_final = make_binary_operation(kind, final_result, first_result, chain_result);

        replacements.push((chain_index, new_middle));
        replacements.push((final_index, new_final));
        rewritten.insert(chain_index);
        rewritten.insert(final_index);

        let _ = (first_lhs, first_rhs);
    }

    for (index, operation) in replacements {
        function.operations[index] = operation;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssociativeBinaryKind {
    And,
    Or,
    Xor,
}

fn associative_binary(
    operation: &Operation,
) -> Option<(AssociativeBinaryKind, ValueId, ValueId, ValueId)> {
    match operation {
        Operation::And { result, lhs, rhs } => {
            Some((AssociativeBinaryKind::And, *result, *lhs, *rhs))
        }

        Operation::Or { result, lhs, rhs } => {
            Some((AssociativeBinaryKind::Or, *result, *lhs, *rhs))
        }

        Operation::Xor { result, lhs, rhs } => {
            Some((AssociativeBinaryKind::Xor, *result, *lhs, *rhs))
        }

        _ => None,
    }
}

fn make_binary_operation(
    kind: AssociativeBinaryKind,
    result: ValueId,
    lhs: ValueId,
    rhs: ValueId,
) -> Operation {
    match kind {
        AssociativeBinaryKind::And => Operation::And { result, lhs, rhs },

        AssociativeBinaryKind::Or => Operation::Or { result, lhs, rhs },

        AssociativeBinaryKind::Xor => Operation::Xor { result, lhs, rhs },
    }
}

fn find_operation_result(operations: &[Operation], value: ValueId) -> Option<usize> {
    operations
        .iter()
        .position(|operation| operation_result(operation) == Some(value))
}

fn operation_result(operation: &Operation) -> Option<ValueId> {
    match operation {
        Operation::Copy { result, .. }
        | Operation::Argument { result, .. }
        | Operation::Constant { result, .. }
        | Operation::Add { result, .. }
        | Operation::Sub { result, .. }
        | Operation::Mul { result, .. }
        | Operation::Eq { result, .. }
        | Operation::NotEqual { result, .. }
        | Operation::GreaterThan { result, .. }
        | Operation::GreaterEqual { result, .. }
        | Operation::LessThan { result, .. }
        | Operation::LessEqual { result, .. }
        | Operation::Not { result, .. }
        | Operation::And { result, .. }
        | Operation::Or { result, .. }
        | Operation::Xor { result, .. }
        | Operation::Mux { result, .. } => Some(*result),

        Operation::Return { .. } => None,
    }
}

fn value_use_count(operations: &[Operation], value: ValueId) -> usize {
    operations
        .iter()
        .map(|operation| operation_use_count(operation, value))
        .sum()
}

fn operation_use_count(operation: &Operation, value: ValueId) -> usize {
    match operation {
        Operation::Copy { operand, .. } | Operation::Not { operand, .. } => {
            usize::from(*operand == value)
        }

        Operation::Add { lhs, rhs, .. }
        | Operation::Sub { lhs, rhs, .. }
        | Operation::Mul { lhs, rhs, .. }
        | Operation::Eq { lhs, rhs, .. }
        | Operation::NotEqual { lhs, rhs, .. }
        | Operation::GreaterThan { lhs, rhs, .. }
        | Operation::GreaterEqual { lhs, rhs, .. }
        | Operation::LessThan { lhs, rhs, .. }
        | Operation::LessEqual { lhs, rhs, .. }
        | Operation::And { lhs, rhs, .. }
        | Operation::Or { lhs, rhs, .. }
        | Operation::Xor { lhs, rhs, .. } => {
            usize::from(*lhs == value) + usize::from(*rhs == value)
        }

        Operation::Mux {
            condition,
            if_true,
            if_false,
            ..
        } => {
            usize::from(*condition == value)
                + usize::from(*if_true == value)
                + usize::from(*if_false == value)
        }

        Operation::Return {
            value: Some(returned),
        } => usize::from(*returned == value),

        Operation::Argument { .. }
        | Operation::Constant { .. }
        | Operation::Return { value: None } => 0,
    }
}
fn crypto_operation_reduce(function: &mut Function) {
    let mut replacements = Vec::new();

    for index in 0..function.operations.len() {
        let replacement = match &function.operations[index] {
            Operation::Not { result, operand } => {
                find_previous_not(&function.operations[..index], *operand).map(|previous| {
                    Operation::Copy {
                        result: *result,
                        operand: previous,
                    }
                })
            }

            Operation::And { result, lhs, rhs } => find_previous_binary(
                &function.operations[..index],
                *lhs,
                *rhs,
                BinaryGateKind::And,
            )
            .map(|previous| Operation::Copy {
                result: *result,
                operand: previous,
            }),

            Operation::Or { result, lhs, rhs } => find_previous_binary(
                &function.operations[..index],
                *lhs,
                *rhs,
                BinaryGateKind::Or,
            )
            .map(|previous| Operation::Copy {
                result: *result,
                operand: previous,
            }),

            Operation::Xor { result, lhs, rhs } => find_previous_binary(
                &function.operations[..index],
                *lhs,
                *rhs,
                BinaryGateKind::Xor,
            )
            .map(|previous| Operation::Copy {
                result: *result,
                operand: previous,
            }),

            Operation::Mux {
                result,
                condition,
                if_true,
                if_false,
            } => find_previous_mux(
                &function.operations[..index],
                *condition,
                *if_true,
                *if_false,
            )
            .map(|previous| Operation::Copy {
                result: *result,
                operand: previous,
            }),

            _ => None,
        };

        if let Some(operation) = replacement {
            replacements.push((index, operation));
        }
    }

    for (index, operation) in replacements {
        function.operations[index] = operation;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryGateKind {
    And,
    Or,
    Xor,
}

fn same_commutative_operands(
    left_lhs: ValueId,
    left_rhs: ValueId,
    right_lhs: ValueId,
    right_rhs: ValueId,
) -> bool {
    (left_lhs == right_lhs && left_rhs == right_rhs)
        || (left_lhs == right_rhs && left_rhs == right_lhs)
}

fn find_previous_not(operations: &[Operation], operand: ValueId) -> Option<ValueId> {
    operations
        .iter()
        .rev()
        .find_map(|operation| match operation {
            Operation::Not {
                result,
                operand: previous_operand,
            } if *previous_operand == operand => Some(*result),

            _ => None,
        })
}

fn find_previous_binary(
    operations: &[Operation],
    lhs: ValueId,
    rhs: ValueId,
    kind: BinaryGateKind,
) -> Option<ValueId> {
    operations
        .iter()
        .rev()
        .find_map(|operation| match (kind, operation) {
            (
                BinaryGateKind::And,
                Operation::And {
                    result,
                    lhs: previous_lhs,
                    rhs: previous_rhs,
                },
            )
            | (
                BinaryGateKind::Or,
                Operation::Or {
                    result,
                    lhs: previous_lhs,
                    rhs: previous_rhs,
                },
            )
            | (
                BinaryGateKind::Xor,
                Operation::Xor {
                    result,
                    lhs: previous_lhs,
                    rhs: previous_rhs,
                },
            ) if same_commutative_operands(lhs, rhs, *previous_lhs, *previous_rhs) => Some(*result),

            _ => None,
        })
}

fn find_previous_mux(
    operations: &[Operation],
    condition: ValueId,
    if_true: ValueId,
    if_false: ValueId,
) -> Option<ValueId> {
    operations
        .iter()
        .rev()
        .find_map(|operation| match operation {
            Operation::Mux {
                result,
                condition: previous_condition,
                if_true: previous_true,
                if_false: previous_false,
            } if *previous_condition == condition
                && *previous_true == if_true
                && *previous_false == if_false =>
            {
                Some(*result)
            }

            _ => None,
        })
}
fn mux_optimize(function: &mut Function) {
    let mut replacements = Vec::new();

    for index in 0..function.operations.len() {
        let replacement = match &function.operations[index] {
            Operation::Mux {
                result,
                condition,
                if_true,
                if_false,
            } => {
                if if_true == if_false
                    || is_constant_bool(&function.operations[..index], *condition, true)
                {
                    Some(Operation::Copy {
                        result: *result,
                        operand: *if_true,
                    })
                } else if is_constant_bool(&function.operations[..index], *condition, false) {
                    Some(Operation::Copy {
                        result: *result,
                        operand: *if_false,
                    })
                } else if is_constant_bool(&function.operations[..index], *if_true, true)
                    && is_constant_bool(&function.operations[..index], *if_false, false)
                {
                    Some(Operation::Copy {
                        result: *result,
                        operand: *condition,
                    })
                } else if is_constant_bool(&function.operations[..index], *if_true, false)
                    && is_constant_bool(&function.operations[..index], *if_false, true)
                {
                    Some(Operation::Not {
                        result: *result,
                        operand: *condition,
                    })
                } else if condition == if_true {
                    Some(Operation::Or {
                        result: *result,
                        lhs: *condition,
                        rhs: *if_false,
                    })
                } else if condition == if_false {
                    Some(Operation::And {
                        result: *result,
                        lhs: *condition,
                        rhs: *if_true,
                    })
                } else {
                    None
                }
            }

            _ => None,
        };

        if let Some(operation) = replacement {
            replacements.push((index, operation));
        }
    }

    for (index, operation) in replacements {
        function.operations[index] = operation;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use fhec_ir::{Type, Value};

    fn make_function(return_type: Type, parameters: Vec<Value>) -> Function {
        Function::new("test", parameters, return_type)
    }

    #[test]
    fn optimization_plan_executes_expensive_gate_lowering() {
        use fhec_analyzer::hints::{
            HintPriority, OptimizerHint, OptimizerHintKind, OptimizerHintSet, TargetMetric,
        };

        let mut function = make_function(Type::uint(8), Vec::new());

        let condition = function.create_value(Type::Bool);
        let if_true = function.create_value(Type::uint(8));
        let if_false = function.create_value(Type::uint(8));
        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Constant {
            result: condition.id,
            value: Constant::Bool(true),
        });

        function.push_operation(Operation::Constant {
            result: if_true.id,
            value: Constant::UInt {
                value: 10,
                width: 8,
            },
        });

        function.push_operation(Operation::Constant {
            result: if_false.id,
            value: Constant::UInt {
                value: 20,
                width: 8,
            },
        });

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: condition.id,
            if_true: if_true.id,
            if_false: if_false.id,
        });

        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReplaceExpensiveGates,
                priority: HintPriority::High,
                target: TargetMetric::GateCost,
                current_value: 100.0,
                reason: "expensive mux".to_string(),
            }],
        };

        let plan = hints::build_optimization_plan(&hints);

        optimize_function_with_plan(&mut function, &plan);

        assert!(matches!(
            function.operations[3],
            Operation::Copy {
                result: candidate_result,
                operand
            } if candidate_result == result.id && operand == if_true.id
        ));
    }
    #[test]
    fn simplifies_mux_with_true_condition() {
        let mut function = make_function(Type::uint(8), Vec::new());

        let condition = function.create_value(Type::Bool);
        let if_true = function.create_value(Type::uint(8));
        let if_false = function.create_value(Type::uint(8));
        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Constant {
            result: condition.id,
            value: Constant::Bool(true),
        });

        function.push_operation(Operation::Constant {
            result: if_true.id,
            value: Constant::UInt {
                value: 10,
                width: 8,
            },
        });

        function.push_operation(Operation::Constant {
            result: if_false.id,
            value: Constant::UInt {
                value: 20,
                width: 8,
            },
        });

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: condition.id,
            if_true: if_true.id,
            if_false: if_false.id,
        });

        mux_optimize(&mut function);

        assert!(matches!(
            function.operations[3],
            Operation::Copy {
                result: candidate_result,
                operand
            } if candidate_result == result.id && operand == if_true.id
        ));
    }

    #[test]
    fn simplifies_mux_with_false_condition() {
        let mut function = make_function(Type::uint(8), Vec::new());

        let condition = function.create_value(Type::Bool);
        let if_true = function.create_value(Type::uint(8));
        let if_false = function.create_value(Type::uint(8));
        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Constant {
            result: condition.id,
            value: Constant::Bool(false),
        });

        function.push_operation(Operation::Constant {
            result: if_true.id,
            value: Constant::UInt {
                value: 10,
                width: 8,
            },
        });

        function.push_operation(Operation::Constant {
            result: if_false.id,
            value: Constant::UInt {
                value: 20,
                width: 8,
            },
        });

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: condition.id,
            if_true: if_true.id,
            if_false: if_false.id,
        });

        mux_optimize(&mut function);

        assert!(matches!(
            function.operations[3],
            Operation::Copy {
                result: candidate_result,
                operand
            } if candidate_result == result.id && operand == if_false.id
        ));
    }

    #[test]
    fn simplifies_mux_with_identical_branches() {
        let mut function = make_function(Type::uint(8), Vec::new());

        let condition = function.create_value(Type::Bool);
        let value = function.create_value(Type::uint(8));
        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: condition.id,
            if_true: value.id,
            if_false: value.id,
        });

        mux_optimize(&mut function);

        assert!(matches!(
            function.operations[0],
            Operation::Copy {
                result: candidate_result,
                operand
            } if candidate_result == result.id && operand == value.id
        ));
    }
    #[test]
    fn optimization_plan_executes_parallelism_exposure() {
        use fhec_analyzer::hints::{
            HintPriority, OptimizerHint, OptimizerHintKind, OptimizerHintSet, TargetMetric,
        };

        let mut function = make_function(
            Type::Bool,
            vec![
                Value::new(ValueId(0), Type::Bool),
                Value::new(ValueId(1), Type::Bool),
                Value::new(ValueId(2), Type::Bool),
                Value::new(ValueId(3), Type::Bool),
            ],
        );

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: first.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::And {
            result: second.id,
            lhs: first.id,
            rhs: ValueId(2),
        });

        function.push_operation(Operation::And {
            result: result.id,
            lhs: second.id,
            rhs: ValueId(3),
        });

        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::IncreaseParallelism,
                priority: HintPriority::High,
                target: TargetMetric::Parallelism,
                current_value: 1.0,
                reason: "expose parallel work".to_string(),
            }],
        };

        let plan = hints::build_optimization_plan(&hints);

        optimize_function_with_plan(&mut function, &plan);

        assert!(matches!(
            function.operations[1],
            Operation::And {
                result: middle_result,
                lhs: ValueId(2),
                rhs: ValueId(3),
            } if middle_result == second.id
        ));

        assert!(matches!(
            function.operations[2],
            Operation::And {
                result: final_result,
                lhs,
                rhs,
            } if final_result == result.id
                && lhs == first.id
                && rhs == second.id
        ));
    }
    #[test]
    fn balances_deep_and_chain() {
        let mut function = make_function(
            Type::Bool,
            vec![
                Value::new(ValueId(0), Type::Bool),
                Value::new(ValueId(1), Type::Bool),
                Value::new(ValueId(2), Type::Bool),
                Value::new(ValueId(3), Type::Bool),
            ],
        );

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: first.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::And {
            result: second.id,
            lhs: first.id,
            rhs: ValueId(2),
        });

        function.push_operation(Operation::And {
            result: result.id,
            lhs: second.id,
            rhs: ValueId(3),
        });

        depth_reduce(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::And {
                result: middle_result,
                lhs: ValueId(2),
                rhs: ValueId(3),
            } if middle_result == second.id
        ));

        assert!(matches!(
            function.operations[2],
            Operation::And {
                result: final_result,
                lhs,
                rhs,
            } if final_result == result.id
                && lhs == first.id
                && rhs == second.id
        ));
    }

    #[test]
    fn depth_reduction_preserves_shared_intermediate() {
        let mut function = make_function(
            Type::Bool,
            vec![
                Value::new(ValueId(0), Type::Bool),
                Value::new(ValueId(1), Type::Bool),
                Value::new(ValueId(2), Type::Bool),
                Value::new(ValueId(3), Type::Bool),
            ],
        );

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);
        let extra = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: first.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::And {
            result: second.id,
            lhs: first.id,
            rhs: ValueId(2),
        });

        function.push_operation(Operation::And {
            result: result.id,
            lhs: second.id,
            rhs: ValueId(3),
        });

        function.push_operation(Operation::Not {
            result: extra.id,
            operand: second.id,
        });

        let before = function.operations.clone();

        depth_reduce(&mut function);

        assert_eq!(function.operations, before);
    }
    #[test]
    fn removes_duplicate_and_operation() {
        let mut function = make_function(
            Type::Bool,
            vec![
                Value::new(ValueId(0), Type::Bool),
                Value::new(ValueId(1), Type::Bool),
            ],
        );

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: first.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::And {
            result: second.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        crypto_operation_reduce(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Copy {
                operand,
                ..
            } if operand == first.id
        ));
    }

    #[test]
    fn removes_duplicate_commutative_and_operation() {
        let mut function = make_function(
            Type::Bool,
            vec![
                Value::new(ValueId(0), Type::Bool),
                Value::new(ValueId(1), Type::Bool),
            ],
        );

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: first.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::And {
            result: second.id,
            lhs: ValueId(1),
            rhs: ValueId(0),
        });

        crypto_operation_reduce(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Copy {
                operand,
                ..
            } if operand == first.id
        ));
    }

    #[test]
    fn removes_duplicate_not_operation() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);

        function.push_operation(Operation::Not {
            result: first.id,
            operand: ValueId(0),
        });

        function.push_operation(Operation::Not {
            result: second.id,
            operand: ValueId(0),
        });

        crypto_operation_reduce(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Copy {
                operand,
                ..
            } if operand == first.id
        ));
    }

    #[test]
    fn removes_duplicate_mux_operation() {
        let mut function = make_function(
            Type::Bool,
            vec![
                Value::new(ValueId(0), Type::Bool),
                Value::new(ValueId(1), Type::Bool),
                Value::new(ValueId(2), Type::Bool),
            ],
        );

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);

        function.push_operation(Operation::Mux {
            result: first.id,
            condition: ValueId(0),
            if_true: ValueId(1),
            if_false: ValueId(2),
        });

        function.push_operation(Operation::Mux {
            result: second.id,
            condition: ValueId(0),
            if_true: ValueId(1),
            if_false: ValueId(2),
        });

        crypto_operation_reduce(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Copy {
                operand,
                ..
            } if operand == first.id
        ));
    }
    #[test]
    fn simplifies_and_with_same_operand() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: result.id,
            lhs: ValueId(0),
            rhs: ValueId(0),
        });

        boolean_optimize(&mut function);

        assert!(matches!(
            function.operations[0],
            Operation::Copy {
                operand: ValueId(0),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_or_with_same_operand() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Or {
            result: result.id,
            lhs: ValueId(0),
            rhs: ValueId(0),
        });

        boolean_optimize(&mut function);

        assert!(matches!(
            function.operations[0],
            Operation::Copy {
                operand: ValueId(0),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_xor_with_same_operand() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Xor {
            result: result.id,
            lhs: ValueId(0),
            rhs: ValueId(0),
        });

        boolean_optimize(&mut function);

        assert!(matches!(
            function.operations[0],
            Operation::Constant {
                value: Constant::Bool(false),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_double_not() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);

        function.push_operation(Operation::Not {
            result: first.id,
            operand: ValueId(0),
        });

        function.push_operation(Operation::Not {
            result: second.id,
            operand: first.id,
        });

        boolean_optimize(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Copy {
                operand: ValueId(0),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_boolean_mux_identity() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let true_value = function.create_value(Type::Bool);
        let false_value = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Constant {
            result: true_value.id,
            value: Constant::Bool(true),
        });

        function.push_operation(Operation::Constant {
            result: false_value.id,
            value: Constant::Bool(false),
        });

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: ValueId(0),
            if_true: true_value.id,
            if_false: false_value.id,
        });

        mux_optimize(&mut function);

        assert!(matches!(
            function.operations[2],
            Operation::Copy {
                operand: ValueId(0),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_boolean_mux_negation() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let true_value = function.create_value(Type::Bool);
        let false_value = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Constant {
            result: true_value.id,
            value: Constant::Bool(true),
        });

        function.push_operation(Operation::Constant {
            result: false_value.id,
            value: Constant::Bool(false),
        });

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: ValueId(0),
            if_true: false_value.id,
            if_false: true_value.id,
        });

        mux_optimize(&mut function);

        assert!(matches!(
            function.operations[2],
            Operation::Not {
                operand: ValueId(0),
                ..
            }
        ));
    }
    #[test]
    fn folds_constant_addition() {
        let mut function = make_function(Type::uint(8), Vec::new());

        let a = function.create_value(Type::uint(8));
        let b = function.create_value(Type::uint(8));
        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Constant {
            result: a.id,
            value: Constant::UInt { value: 5, width: 8 },
        });

        function.push_operation(Operation::Constant {
            result: b.id,
            value: Constant::UInt { value: 3, width: 8 },
        });

        function.push_operation(Operation::Add {
            result: result.id,
            lhs: a.id,
            rhs: b.id,
        });

        constant_fold(&mut function);

        assert!(matches!(
            function.operations[2],
            Operation::Constant {
                result,
                value: Constant::UInt {
                    value: 8,
                    width: 8,
                },
            } if result == result
        ));
    }

    #[test]
    fn folds_constant_subtraction() {
        let mut function = make_function(Type::uint(8), Vec::new());

        let a = function.create_value(Type::uint(8));
        let b = function.create_value(Type::uint(8));
        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Constant {
            result: a.id,
            value: Constant::UInt {
                value: 10,
                width: 8,
            },
        });

        function.push_operation(Operation::Constant {
            result: b.id,
            value: Constant::UInt { value: 3, width: 8 },
        });

        function.push_operation(Operation::Sub {
            result: result.id,
            lhs: a.id,
            rhs: b.id,
        });

        constant_fold(&mut function);

        assert!(matches!(
            function.operations[2],
            Operation::Constant {
                value: Constant::UInt { value: 7, width: 8 },
                ..
            }
        ));
    }

    #[test]
    fn folds_constant_multiplication() {
        let mut function = make_function(Type::uint(8), Vec::new());

        let a = function.create_value(Type::uint(8));
        let b = function.create_value(Type::uint(8));
        let result = function.create_value(Type::uint(8));

        function.push_operation(Operation::Constant {
            result: a.id,
            value: Constant::UInt { value: 6, width: 8 },
        });

        function.push_operation(Operation::Constant {
            result: b.id,
            value: Constant::UInt { value: 7, width: 8 },
        });

        function.push_operation(Operation::Mul {
            result: result.id,
            lhs: a.id,
            rhs: b.id,
        });

        constant_fold(&mut function);

        assert!(matches!(
            function.operations[2],
            Operation::Constant {
                value: Constant::UInt {
                    value: 42,
                    width: 8,
                },
                ..
            }
        ));
    }

    #[test]
    fn does_not_fold_unknown_values() {
        let mut function = make_function(
            Type::encrypted(Type::uint(8)),
            vec![
                Value::new(ValueId(0), Type::encrypted(Type::uint(8))),
                Value::new(ValueId(1), Type::encrypted(Type::uint(8))),
            ],
        );

        let result = function.create_value(Type::encrypted(Type::uint(8)));

        function.push_operation(Operation::Add {
            result: result.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        constant_fold(&mut function);

        assert!(matches!(function.operations[0], Operation::Add { .. }));
    }

    #[test]
    fn folds_constant_not() {
        let mut function = make_function(Type::Bool, Vec::new());

        let value = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Constant {
            result: value.id,
            value: Constant::Bool(true),
        });

        function.push_operation(Operation::Not {
            result: result.id,
            operand: value.id,
        });

        constant_fold(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Constant {
                value: Constant::Bool(false),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_and_true() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let constant = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Constant {
            result: constant.id,
            value: Constant::Bool(true),
        });

        function.push_operation(Operation::And {
            result: result.id,
            lhs: ValueId(0),
            rhs: constant.id,
        });

        boolean_optimize(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Copy {
                operand: ValueId(0),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_or_false() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let constant = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Constant {
            result: constant.id,
            value: Constant::Bool(false),
        });

        function.push_operation(Operation::Or {
            result: result.id,
            lhs: ValueId(0),
            rhs: constant.id,
        });

        boolean_optimize(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Copy {
                operand: ValueId(0),
                ..
            }
        ));
    }

    #[test]
    fn simplifies_xor_true_to_not() {
        let mut function = make_function(Type::Bool, vec![Value::new(ValueId(0), Type::Bool)]);

        let constant = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Constant {
            result: constant.id,
            value: Constant::Bool(true),
        });

        function.push_operation(Operation::Xor {
            result: result.id,
            lhs: ValueId(0),
            rhs: constant.id,
        });

        boolean_optimize(&mut function);

        assert!(matches!(
            function.operations[1],
            Operation::Not {
                operand: ValueId(0),
                ..
            }
        ));
    }
}
