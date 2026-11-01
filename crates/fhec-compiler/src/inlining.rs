use std::collections::HashMap;

use fhec_frontend::ast::{Block, Expression, Function, Program, Statement};

use crate::LoweringError;

const MAX_INLINE_DEPTH: usize = 32;

pub fn expand_program(program: &Program) -> Result<Program, LoweringError> {
    let mut functions = Vec::with_capacity(program.functions.len());

    for function in &program.functions {
        let mut stack = vec![function.name.clone()];
        let body = expand_root_block(&function.body, program, &mut stack)?;
        let mut expanded = function.clone();
        expanded.body = body;
        functions.push(expanded);
    }

    Ok(Program { functions })
}

fn expand_root_block(
    block: &Block,
    program: &Program,
    stack: &mut Vec<String>,
) -> Result<Block, LoweringError> {
    let mut statements = Vec::with_capacity(block.statements.len());
    let environment = HashMap::new();

    for statement in &block.statements {
        let expanded = match statement {
            Statement::Let { name, ty, value } => Statement::Let {
                name: name.clone(),
                ty: ty.clone(),
                value: expand_expression(value, &environment, program, stack)?,
            },
            Statement::Return(Some(value)) => Statement::Return(Some(expand_expression(
                value,
                &environment,
                program,
                stack,
            )?)),
            Statement::Return(None) => Statement::Return(None),
            Statement::Expression(value) => {
                Statement::Expression(expand_expression(value, &environment, program, stack)?)
            }
        };
        statements.push(expanded);
    }

    Ok(Block { statements })
}

fn expand_expression(
    expression: &Expression,
    environment: &HashMap<String, Expression>,
    program: &Program,
    stack: &mut Vec<String>,
) -> Result<Expression, LoweringError> {
    match expression {
        Expression::Integer(_) | Expression::Boolean(_) => Ok(expression.clone()),
        Expression::Variable(name) => Ok(environment
            .get(name)
            .cloned()
            .unwrap_or_else(|| expression.clone())),
        Expression::Binary {
            left,
            operator,
            right,
        } => Ok(Expression::Binary {
            left: Box::new(expand_expression(left, environment, program, stack)?),
            operator: *operator,
            right: Box::new(expand_expression(right, environment, program, stack)?),
        }),
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = Box::new(expand_expression(condition, environment, program, stack)?);
            let then_branch = expand_value_branch(then_branch, environment, program, stack)?;
            let else_branch = match else_branch {
                Some(branch) => Some(expand_value_branch(branch, environment, program, stack)?),
                None => None,
            };
            Ok(Expression::If {
                condition,
                then_branch,
                else_branch,
            })
        }
        Expression::Call {
            function,
            arguments,
        } => {
            let mut resolved = Vec::with_capacity(arguments.len());
            for argument in arguments {
                resolved.push(expand_expression(argument, environment, program, stack)?);
            }
            inline_call(function, &resolved, program, stack)
        }
    }
}

fn expand_value_branch(
    block: &Block,
    environment: &HashMap<String, Expression>,
    program: &Program,
    stack: &mut Vec<String>,
) -> Result<Block, LoweringError> {
    if block.statements.is_empty() {
        return Ok(block.clone());
    }

    let mut scope = environment.clone();
    for (index, statement) in block.statements.iter().enumerate() {
        let last = index + 1 == block.statements.len();
        match statement {
            Statement::Let { name, value, .. } => {
                let resolved = expand_expression(value, &scope, program, stack)?;
                scope.insert(name.clone(), resolved);
            }
            Statement::Return(Some(value)) | Statement::Expression(value) if last => {
                let resolved = expand_expression(value, &scope, program, stack)?;
                return Ok(Block {
                    statements: vec![Statement::Expression(resolved)],
                });
            }
            Statement::Return(None) if last => {
                return Ok(Block {
                    statements: vec![Statement::Return(None)],
                });
            }
            Statement::Expression(value) => {
                expand_expression(value, &scope, program, stack)?;
            }
            Statement::Return(_) => {
                return Err(LoweringError::UnsupportedCallBody(
                    "early return inside a value branch".to_owned(),
                ));
            }
        }
    }

    Ok(block.clone())
}

fn inline_call(
    name: &str,
    arguments: &[Expression],
    program: &Program,
    stack: &mut Vec<String>,
) -> Result<Expression, LoweringError> {
    let callee = program
        .functions
        .iter()
        .find(|function| function.name == name)
        .ok_or_else(|| LoweringError::UnknownFunction(name.to_owned()))?;

    if callee.parameters.len() != arguments.len() {
        return Err(LoweringError::CallArgumentCount {
            function: name.to_owned(),
            expected: callee.parameters.len(),
            actual: arguments.len(),
        });
    }

    if stack.iter().any(|entry| entry == name) {
        let mut chain = stack.clone();
        chain.push(name.to_owned());
        return Err(LoweringError::RecursiveCall(chain.join(" -> ")));
    }
    if stack.len() >= MAX_INLINE_DEPTH {
        return Err(LoweringError::CallDepthExceeded(name.to_owned()));
    }

    let mut scope: HashMap<String, Expression> = callee
        .parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
        .collect();

    stack.push(name.to_owned());
    let result = inline_body(callee, &mut scope, program, stack);
    stack.pop();
    result
}

fn inline_body(
    callee: &Function,
    scope: &mut HashMap<String, Expression>,
    program: &Program,
    stack: &mut Vec<String>,
) -> Result<Expression, LoweringError> {
    for (index, statement) in callee.body.statements.iter().enumerate() {
        let last = index + 1 == callee.body.statements.len();
        match statement {
            Statement::Let { name, value, .. } => {
                let value = expand_expression(value, scope, program, stack)?;
                scope.insert(name.clone(), value);
            }
            Statement::Return(Some(value)) if last => {
                return expand_expression(value, scope, program, stack);
            }
            Statement::Expression(value) if last => {
                return expand_expression(value, scope, program, stack);
            }
            Statement::Expression(value) => {
                expand_expression(value, scope, program, stack)?;
            }
            Statement::Return(_) => {
                return Err(LoweringError::UnsupportedCallBody(callee.name.clone()));
            }
        }
    }
    Err(LoweringError::UnsupportedCallBody(callee.name.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fhec_frontend::{lexer::Lexer, parser::Parser};

    fn compile(source: &str) -> Result<fhec_ir::Module, LoweringError> {
        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        crate::lowering::lower_program(&program)
    }

    #[test]
    fn calls_inline_through_two_functions() {
        let source = r#"
            fn twice(a: encrypted<u8>) -> encrypted<u8> { return a + a; }
            fn calc(x: encrypted<u8>, y: encrypted<u8>) -> encrypted<u8> {
                return twice(x + y);
            }
        "#;
        let module = compile(source).expect("inlining failed");
        assert_eq!(module.functions.len(), 2);
        let function = &module.functions[1];
        let circuit = crate::lower_function_to_circuit(function);
        assert!(circuit.verify(function).is_ok());
        let input = [3u8, 4u8]
            .iter()
            .flat_map(|value| (0..8).map(move |index| (((*value) >> index) & 1) == 1))
            .collect::<Vec<_>>();
        let result = circuit
            .circuit
            .evaluate(&input, &circuit.outputs)
            .expect("eval failed");
        let decoded = result
            .iter()
            .enumerate()
            .fold(0u8, |number, (index, bit)| number | ((*bit as u8) << index));
        assert_eq!(decoded, 14);
    }

    #[test]
    fn calls_support_forward_reference_and_let() {
        let source = r#"
            fn main(x: encrypted<u8>, y: encrypted<u8>) -> encrypted<u8> {
                return add(x, y);
            }
            fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> {
                let sum = a + b;
                return sum;
            }
        "#;
        assert!(compile(source).is_ok());
    }

    #[test]
    fn rejects_recursive_calls() {
        let source = r#"
            fn first(a: encrypted<u8>) -> encrypted<u8> { return second(a); }
            fn second(a: encrypted<u8>) -> encrypted<u8> { return first(a); }
        "#;
        assert!(matches!(
            compile(source),
            Err(LoweringError::RecursiveCall(_))
        ));
    }

    #[test]
    fn rejects_unknown_call() {
        let source = r#"
            fn main(a: encrypted<u8>) -> encrypted<u8> { return missing(a); }
        "#;
        assert!(matches!(
            compile(source),
            Err(LoweringError::UnknownFunction(_))
        ));
    }
}
