use std::collections::BTreeMap;

use fhec_frontend::ast::{BinaryOperator, Block, Expression, Function, Program, Statement};

#[derive(Clone, Debug, Default)]
pub(crate) struct OptimizationStats {
    pub(crate) rules: BTreeMap<String, u64>,
}

impl OptimizationStats {
    fn record(&mut self, name: &str) {
        *self.rules.entry(name.to_owned()).or_default() += 1;
    }
}

pub(crate) fn optimize_program(source: &Program) -> (Program, OptimizationStats) {
    let mut stats = OptimizationStats::default();
    let functions = source
        .functions
        .iter()
        .map(|function| optimize_function(function, &mut stats))
        .collect();
    (Program { functions }, stats)
}

fn optimize_function(function: &Function, stats: &mut OptimizationStats) -> Function {
    let mut rewritten = function.clone();
    let mut prior: Vec<(Expression, String)> = Vec::new();
    rewritten.body.statements = function
        .body
        .statements
        .iter()
        .map(|statement| match statement {
            Statement::Let { name, ty, value } => {
                let original = optimize_expression(value, stats);
                let replacement = if shareable(&original) {
                    prior
                        .iter()
                        .find(|(expression, _)| *expression == original)
                        .map(|(_, previous)| previous.clone())
                } else {
                    None
                };

                let optimized = if let Some(previous) = replacement {
                    stats.record("common_subexpression_binding");
                    Expression::Variable(previous)
                } else {
                    prior.push((original.clone(), name.clone()));
                    original
                };

                Statement::Let {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: optimized,
                }
            }
            Statement::Return(Some(value)) => {
                Statement::Return(Some(optimize_expression(value, stats)))
            }
            Statement::Expression(value) => {
                Statement::Expression(optimize_expression(value, stats))
            }
            Statement::Return(None) => Statement::Return(None),
        })
        .collect();
    rewritten
}

fn shareable(value: &Expression) -> bool {
    match value {
        Expression::Binary { left, right, .. } => !contains_call(left) && !contains_call(right),
        _ => false,
    }
}

fn contains_call(value: &Expression) -> bool {
    match value {
        Expression::Call { .. } => true,
        Expression::Binary { left, right, .. } => contains_call(left) || contains_call(right),
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            contains_call(condition)
                || then_branch.statements.iter().any(statement_contains_call)
                || else_branch
                    .as_ref()
                    .is_some_and(|block| block.statements.iter().any(statement_contains_call))
        }
        Expression::Integer(_) | Expression::Boolean(_) | Expression::Variable(_) => false,
    }
}

fn statement_contains_call(value: &Statement) -> bool {
    match value {
        Statement::Let { value, .. }
        | Statement::Expression(value)
        | Statement::Return(Some(value)) => contains_call(value),
        Statement::Return(None) => false,
    }
}

fn optimize_expression(value: &Expression, stats: &mut OptimizationStats) -> Expression {
    match value {
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let lhs = optimize_expression(left, stats);
            let rhs = optimize_expression(right, stats);
            match operator {
                BinaryOperator::Add if rhs == Expression::Integer(0) => {
                    stats.record("add_zero_right");
                    lhs
                }
                BinaryOperator::Add if lhs == Expression::Integer(0) => {
                    stats.record("add_zero_left");
                    rhs
                }
                BinaryOperator::Subtract if rhs == Expression::Integer(0) => {
                    stats.record("subtract_zero");
                    lhs
                }
                BinaryOperator::Multiply if rhs == Expression::Integer(1) => {
                    stats.record("multiply_one_right");
                    lhs
                }
                BinaryOperator::Multiply if lhs == Expression::Integer(1) => {
                    stats.record("multiply_one_left");
                    rhs
                }
                _ => Expression::Binary {
                    left: Box::new(lhs),
                    operator: *operator,
                    right: Box::new(rhs),
                },
            }
        }
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = optimize_expression(condition, stats);
            let then_branch = optimize_block(then_branch, stats);
            let else_branch = else_branch
                .as_ref()
                .map(|block| optimize_block(block, stats));

            if matches!(condition, Expression::Variable(_)) {
                if let Some(ref otherwise) = else_branch {
                    let then_value = branch_expression(&then_branch);
                    let else_value = branch_expression(otherwise);
                    if let (Some(first), Some(second)) = (then_value, else_value) {
                        if first == second && !contains_call(first) {
                            stats.record("identical_selection_branches");
                            return first.clone();
                        }
                    }
                }
            }

            Expression::If {
                condition: Box::new(condition),
                then_branch,
                else_branch,
            }
        }
        Expression::Call {
            function,
            arguments,
        } => Expression::Call {
            function: function.clone(),
            arguments: arguments
                .iter()
                .map(|argument| optimize_expression(argument, stats))
                .collect(),
        },
        _ => value.clone(),
    }
}

fn optimize_block(block: &Block, stats: &mut OptimizationStats) -> Block {
    Block {
        statements: block
            .statements
            .iter()
            .map(|statement| match statement {
                Statement::Expression(value) => {
                    Statement::Expression(optimize_expression(value, stats))
                }
                Statement::Return(Some(value)) => {
                    Statement::Return(Some(optimize_expression(value, stats)))
                }
                _ => statement.clone(),
            })
            .collect(),
    }
}

fn branch_expression(block: &Block) -> Option<&Expression> {
    match block.statements.as_slice() {
        [Statement::Return(Some(value))] | [Statement::Expression(value)] => Some(value),
        _ => None,
    }
}
