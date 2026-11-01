use std::collections::BTreeMap;

use fhec_frontend::ast::{BinaryOperator, Block, Expression, Function, Program, Statement};
use serde::Serialize;

use super::call_graph::FunctionCallGraphReport;
use super::emitter::CoFheError;
use super::optimizer::OptimizationStats;

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct OperationProfile {
    pub encrypted_operations: u64,
    pub trivial_encryptions: u64,
    pub helper_calls: u64,
    pub max_local_operation_depth: u32,
    pub operations: BTreeMap<String, u64>,
}

impl OperationProfile {
    fn record_operation(&mut self, name: &str) {
        self.encrypted_operations += 1;
        *self.operations.entry(name.to_owned()).or_default() += 1;
    }

    fn absorb(&mut self, other: &Self) {
        self.encrypted_operations += other.encrypted_operations;
        self.trivial_encryptions += other.trivial_encryptions;
        self.helper_calls += other.helper_calls;
        self.max_local_operation_depth = self
            .max_local_operation_depth
            .max(other.max_local_operation_depth);
        for (name, count) in &other.operations {
            *self.operations.entry(name.clone()).or_default() += count;
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionCostReport {
    pub name: String,
    pub baseline: OperationProfile,
    pub optimized: OperationProfile,
    pub eliminated_encrypted_operations: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CoFheCostReport {
    pub schema: &'static str,
    pub emitted_variant: &'static str,
    pub measurement: &'static str,
    pub baseline_totals: OperationProfile,
    pub optimized_totals: OperationProfile,
    pub eliminated_encrypted_operations: u64,
    pub reduction_percent: u64,
    pub rules_applied: BTreeMap<String, u64>,
    pub functions: Vec<FunctionCostReport>,
    pub call_graph: Vec<FunctionCallGraphReport>,
    pub limitations: Vec<&'static str>,
}

impl CoFheCostReport {
    pub fn to_pretty_json(&self) -> Result<String, CoFheError> {
        serde_json::to_string_pretty(self)
            .map_err(|error| CoFheError(format!("Could not serialize cost report: {error}")))
    }
}

pub(crate) fn build_report(
    baseline: &Program,
    optimized: &Program,
    stats: OptimizationStats,
    emit_optimized: bool,
    call_graph: Vec<FunctionCallGraphReport>,
) -> CoFheCostReport {
    let mut baseline_totals = OperationProfile::default();
    let mut optimized_totals = OperationProfile::default();
    let mut functions = Vec::new();

    for (source, result) in baseline.functions.iter().zip(optimized.functions.iter()) {
        let before = profile_function(source);
        let after = profile_function(result);
        baseline_totals.absorb(&before);
        optimized_totals.absorb(&after);
        functions.push(FunctionCostReport {
            name: source.name.clone(),
            eliminated_encrypted_operations: before
                .encrypted_operations
                .saturating_sub(after.encrypted_operations),
            baseline: before,
            optimized: after,
        });
    }

    let eliminated = baseline_totals
        .encrypted_operations
        .saturating_sub(optimized_totals.encrypted_operations);
    let reduction = eliminated
        .saturating_mul(100)
        .checked_div(baseline_totals.encrypted_operations)
        .unwrap_or(0);

    CoFheCostReport {
        schema: "fhec-cofhe-static-optimization-v1",
        emitted_variant: if emit_optimized {
            "optimized"
        } else {
            "baseline"
        },
        measurement: "static source-level operation counts, not gas or FHE execution time",
        baseline_totals,
        optimized_totals,
        eliminated_encrypted_operations: eliminated,
        reduction_percent: reduction,
        rules_applied: stats.rules,
        functions,
        call_graph,
        limitations: vec![
            "Counts are direct per function definition, not weighted by runtime helper call frequency.",
            "Depth is syntactic nesting within one function, not the end-to-end CoFHE task graph.",
            "No gas, latency, or real-network cost is inferred from operation counts.",
            "Optimizations preserve encrypted values, but ciphertext handles and access-control side effects may differ.",
            "Call-graph expanded operations assume one invocation of the named helper and count eager evaluation of both encrypted branches.",
            "Expanded operations are deterministic source-level estimates, not CoFHE task scheduling, gas, or runtime measurements.",
        ],
    }
}

fn profile_function(function: &Function) -> OperationProfile {
    let mut profile = OperationProfile::default();
    for statement in &function.body.statements {
        let depth = match statement {
            Statement::Let { value, .. }
            | Statement::Return(Some(value))
            | Statement::Expression(value) => visit(value, &mut profile),
            Statement::Return(None) => 0,
        };
        profile.max_local_operation_depth = profile.max_local_operation_depth.max(depth);
    }
    profile
}

fn visit_block(block: &Block, profile: &mut OperationProfile) -> u32 {
    block
        .statements
        .iter()
        .map(|statement| match statement {
            Statement::Let { value, .. }
            | Statement::Return(Some(value))
            | Statement::Expression(value) => visit(value, profile),
            Statement::Return(None) => 0,
        })
        .max()
        .unwrap_or(0)
}

fn visit(expression: &Expression, profile: &mut OperationProfile) -> u32 {
    match expression {
        Expression::Integer(_) | Expression::Boolean(_) => {
            profile.trivial_encryptions += 1;
            0
        }
        Expression::Variable(_) => 0,
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let left_depth = visit(left, profile);
            let right_depth = visit(right, profile);
            let op = match operator {
                BinaryOperator::Add => "add",
                BinaryOperator::Subtract => "sub",
                BinaryOperator::Multiply => "mul",
                BinaryOperator::Equal => "eq",
                BinaryOperator::NotEqual => "ne",
                BinaryOperator::Greater => "gt",
                BinaryOperator::GreaterEqual => "ge",
                BinaryOperator::Less => "lt",
                BinaryOperator::LessEqual => "le",
            };
            profile.record_operation(op);
            1 + left_depth.max(right_depth)
        }
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_depth = visit(condition, profile);
            let then_depth = visit_block(then_branch, profile);
            let else_depth = else_branch
                .as_ref()
                .map_or(0, |block| visit_block(block, profile));
            profile.record_operation("select");
            1 + condition_depth.max(then_depth).max(else_depth)
        }
        Expression::Call { arguments, .. } => {
            profile.helper_calls += 1;
            arguments
                .iter()
                .map(|argument| visit(argument, profile))
                .max()
                .unwrap_or(0)
        }
    }
}
