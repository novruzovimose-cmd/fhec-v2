use std::collections::{BTreeMap, HashMap};

use fhec_frontend::ast::{Block, Expression, Function, Program, Statement};
use serde::Serialize;

use super::emitter::CoFheError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionInvocationCost {
    pub direct_encrypted_operations: u64,
    pub expanded_encrypted_operations: u64,
    pub direct_helper_calls: u64,
    pub expanded_helper_calls: u64,
    pub max_helper_call_depth: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionCallGraphReport {
    pub name: String,
    pub baseline: FunctionInvocationCost,
    pub optimized: FunctionInvocationCost,
    pub eliminated_expanded_operations: u64,
}

pub(crate) fn analyze_call_graph(
    source: &Program,
    optimized: &Program,
) -> Result<Vec<FunctionCallGraphReport>, CoFheError> {
    let before = analyze_program(source)?;
    let after = analyze_program(optimized)?;
    source
        .functions
        .iter()
        .map(|function| {
            let baseline = before.get(&function.name).ok_or_else(|| {
                CoFheError(format!(
                    "Missing baseline call-graph entry: {}",
                    function.name
                ))
            })?;
            let optimized = after.get(&function.name).ok_or_else(|| {
                CoFheError(format!(
                    "Missing optimized call-graph entry: {}",
                    function.name
                ))
            })?;
            Ok(FunctionCallGraphReport {
                name: function.name.clone(),
                eliminated_expanded_operations: baseline
                    .expanded_encrypted_operations
                    .saturating_sub(optimized.expanded_encrypted_operations),
                baseline: baseline.clone(),
                optimized: optimized.clone(),
            })
        })
        .collect()
}

pub(crate) fn validate_call_graph(program: &Program) -> Result<(), CoFheError> {
    analyze_program(program).map(|_| ())
}

fn analyze_program(
    program: &Program,
) -> Result<HashMap<String, FunctionInvocationCost>, CoFheError> {
    let functions: HashMap<&str, &Function> = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect();
    let mut cache = HashMap::new();
    let mut stack = Vec::new();
    for function in &program.functions {
        visit_function(&function.name, &functions, &mut cache, &mut stack)?;
    }
    Ok(cache)
}

fn visit_function(
    name: &str,
    functions: &HashMap<&str, &Function>,
    cache: &mut HashMap<String, FunctionInvocationCost>,
    stack: &mut Vec<String>,
) -> Result<FunctionInvocationCost, CoFheError> {
    if let Some(known) = cache.get(name) {
        return Ok(known.clone());
    }
    if let Some(position) = stack.iter().position(|item| item == name) {
        let mut cycle = stack[position..].to_vec();
        cycle.push(name.to_owned());
        return Err(CoFheError(format!(
            "Recursive CoFHE helper call is unsupported: {}",
            cycle.join(" -> ")
        )));
    }
    let function = functions
        .get(name)
        .ok_or_else(|| CoFheError(format!("Unknown CoFHE helper function: {name}")))?;
    stack.push(name.to_owned());
    let mut direct_operations = 0_u64;
    let mut calls = BTreeMap::new();
    for statement in &function.body.statements {
        collect_statement(statement, &mut direct_operations, &mut calls)?;
    }
    let direct_helper_calls = calls
        .values()
        .copied()
        .try_fold(0_u64, |accumulator, count| {
            accumulator
                .checked_add(count)
                .ok_or_else(|| CoFheError("CoFHE direct helper count overflow".into()))
        })?;
    let mut result = FunctionInvocationCost {
        direct_encrypted_operations: direct_operations,
        expanded_encrypted_operations: direct_operations,
        direct_helper_calls,
        expanded_helper_calls: 0,
        max_helper_call_depth: 0,
    };
    result.expanded_helper_calls = result.direct_helper_calls;
    for (callee, multiplicity) in calls {
        let nested = visit_function(&callee, functions, cache, stack)?;
        let expanded_ops = nested
            .expanded_encrypted_operations
            .checked_mul(multiplicity)
            .ok_or_else(|| CoFheError("CoFHE call-graph operation count overflow".into()))?;
        result.expanded_encrypted_operations = result
            .expanded_encrypted_operations
            .checked_add(expanded_ops)
            .ok_or_else(|| CoFheError("CoFHE call-graph operation count overflow".into()))?;
        let expanded_calls = nested
            .expanded_helper_calls
            .checked_mul(multiplicity)
            .ok_or_else(|| CoFheError("CoFHE call-graph helper count overflow".into()))?;
        result.expanded_helper_calls = result
            .expanded_helper_calls
            .checked_add(expanded_calls)
            .ok_or_else(|| CoFheError("CoFHE call-graph helper count overflow".into()))?;
        result.max_helper_call_depth = result.max_helper_call_depth.max(
            nested
                .max_helper_call_depth
                .checked_add(1)
                .ok_or_else(|| CoFheError("CoFHE call-graph depth overflow".into()))?,
        );
    }
    stack.pop();
    cache.insert(name.to_owned(), result.clone());
    Ok(result)
}

fn collect_statement(
    statement: &Statement,
    operations: &mut u64,
    calls: &mut BTreeMap<String, u64>,
) -> Result<(), CoFheError> {
    match statement {
        Statement::Let { value, .. }
        | Statement::Expression(value)
        | Statement::Return(Some(value)) => collect_expression(value, operations, calls),
        Statement::Return(None) => Ok(()),
    }
}

fn collect_block(
    block: &Block,
    operations: &mut u64,
    calls: &mut BTreeMap<String, u64>,
) -> Result<(), CoFheError> {
    for statement in &block.statements {
        collect_statement(statement, operations, calls)?;
    }
    Ok(())
}

fn collect_expression(
    expression: &Expression,
    operations: &mut u64,
    calls: &mut BTreeMap<String, u64>,
) -> Result<(), CoFheError> {
    match expression {
        Expression::Integer(_) | Expression::Boolean(_) | Expression::Variable(_) => Ok(()),
        Expression::Binary { left, right, .. } => {
            *operations = operations
                .checked_add(1)
                .ok_or_else(|| CoFheError("CoFHE direct operation count overflow".into()))?;
            collect_expression(left, operations, calls)?;
            collect_expression(right, operations, calls)
        }
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            *operations = operations
                .checked_add(1)
                .ok_or_else(|| CoFheError("CoFHE direct operation count overflow".into()))?;
            collect_expression(condition, operations, calls)?;
            collect_block(then_branch, operations, calls)?;
            if let Some(otherwise) = else_branch {
                collect_block(otherwise, operations, calls)?;
            }
            Ok(())
        }
        Expression::Call {
            function,
            arguments,
        } => {
            let count = calls.entry(function.clone()).or_default();
            *count = count
                .checked_add(1)
                .ok_or_else(|| CoFheError("CoFHE direct helper count overflow".into()))?;
            for argument in arguments {
                collect_expression(argument, operations, calls)?;
            }
            Ok(())
        }
    }
}
