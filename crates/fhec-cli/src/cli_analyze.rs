use std::error::Error;
use std::fs;
use std::path::Path;

use fhec_analyzer::v4::V4CostModel;
use fhec_compiler::{Compiler, render_compiler_v4_report};
use fhec_frontend::{Lexer, Parser};

pub fn analyze(path: &str) -> Result<(), Box<dyn Error>> {
    if !Path::new(path).is_file() {
        return Err(format!("Source file not found: {path}").into());
    }

    let source = fs::read_to_string(path)?;

    let tokens = Lexer::new(&source)
        .tokenize()
        .map_err(|e| format!("Lexing failed: {e:?}"))?;

    let program = Parser::new(tokens)
        .parse()
        .map_err(|e| format!("Parsing failed: {e:?}"))?;

    if program.functions.is_empty() {
        return Err("No functions found".into());
    }

    let compiler = Compiler::new();

    let module = compiler
        .lower(&program)
        .map_err(|e| format!("Lowering failed: {e:?}"))?;

    let model = V4CostModel::default();

    println!("FHEC Circuit Analysis");
    println!("Source: {path}");
    println!("Functions: {}", module.functions.len());
    println!("Cost model: default (not hardware-calibrated)");

    for function in &module.functions {
        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .map_err(|e| format!("Analysis failed for {}: {e:?}", function.name))?;

        selection
            .circuit_function
            .verify(function)
            .map_err(|e| format!("Verification failed: {e:?}"))?;

        println!();
        println!("Function: {}", function.name);
        println!(
            "Circuit inputs: {} bits",
            crate::circuit_input_bit_count(&selection.circuit_function)
        );
        println!(
            "Circuit outputs: {} bits",
            selection.circuit_function.outputs.len()
        );

        println!("{}", render_compiler_v4_report(&selection));
    }

    println!();
    println!("Analysis complete.");

    Ok(())
}
