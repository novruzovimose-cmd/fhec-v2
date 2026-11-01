mod cli_analyze;
mod cli_cofhe;
mod cli_profile;
mod cli_run;
#[cfg(test)]
mod cli_tests;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use fhec_compiler::Compiler;
use fhec_frontend::{Lexer, Parser};

fn run_bench(args: &[String]) -> Result<(), Box<dyn Error>> {
    use std::process::Command;

    let mut command = Command::new("cargo");

    command.args([
        "run",
        "--release",
        "-p",
        "fhec-bench",
        "--bin",
        "fhec-bench",
        "--locked",
        "--",
    ]);
    command.args(args);

    let status = command.status()?;

    if !status.success() {
        return Err(format!("Benchmark runner failed with status: {status}").into());
    }

    Ok(())
}
fn print_help() {
    println!("FHEC - FHE-Native Circuit Compiler");
    println!();
    println!("USAGE:");
    println!("  fhec build <source.fhe>");
    println!("  fhec analyze <source.fhe>");
    println!(
        "  fhec cofhe <source.fhe> --out <output.sol> [--optimize] [--report <report.json>] [--library <name>]"
    );
    println!("  fhec run <source.fhe> --function <name> --inputs <a,b>");
    println!("  fhec run <source.fhe> --function <name> --inputs-stdin");
    println!("  fhec --help");
    println!();
    println!("COMMANDS:");
    println!("  build       Compile and verify FHE circuits");
    println!("  bench       Run FHEC performance benchmarks");
    println!("  profile     Manage TFHE hardware calibration");
    println!("  analyze     Analyze and compare circuit implementations");
    println!("  cofhe       Generate a Solidity CoFHE library (experimental)");
}

fn circuit_input_bit_count(circuit: &fhec_compiler::CircuitFunction) -> usize {
    circuit.inputs.values().map(|value| value.width()).sum()
}

fn build(path: &str) -> Result<(), Box<dyn Error>> {
    let source_path = Path::new(path);

    if !source_path.is_file() {
        return Err(format!("Source file not found: {path}").into());
    }

    let source = fs::read_to_string(source_path)?;

    let tokens = Lexer::new(&source)
        .tokenize()
        .map_err(|e| format!("Lexing failed: {e:?}"))?;

    let program = Parser::new(tokens)
        .parse()
        .map_err(|e| format!("Parsing failed: {e:?}"))?;

    if program.functions.is_empty() {
        return Err("No functions found in source file".into());
    }

    let compiler = Compiler::new();

    let module = compiler
        .lower(&program)
        .map_err(|e| format!("IR lowering failed: {e:?}"))?;

    println!("FHEC Build");
    println!("Source: {path}");
    println!("Functions: {}", module.functions.len());

    for function in &module.functions {
        let circuit = compiler
            .try_lower_to_circuit(function)
            .map_err(|e| format!("Circuit lowering failed for {}: {e:?}", function.name))?;

        circuit
            .verify(function)
            .map_err(|e| format!("Circuit verification failed for {}: {e:?}", function.name))?;

        println!();
        println!("Function: {}", function.name);
        println!("  Inputs (bits): {}", circuit_input_bit_count(&circuit));
        println!("  Outputs (bits): {}", circuit.outputs.len());
        println!("  Verification: PASS");
    }

    println!();
    println!("Build successful.");

    Ok(())
}

fn dispatch(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };

    match command {
        "--help" | "-h" | "help" if args.len() == 1 => {
            print_help();
            Ok(())
        }
        "build" => {
            if args.len() != 2 {
                return Err("Usage: fhec build <source.fhe>".into());
            }
            build(&args[1])
        }
        "analyze" => {
            if args.len() != 2 {
                return Err("Usage: fhec analyze <source.fhe>".into());
            }
            cli_analyze::analyze(&args[1])
        }
        "cofhe" => cli_cofhe::generate(&args[1..]),
        "run" => {
            if args.len() < 2 || args[1].starts_with("--") {
                return Err("Usage: fhec run <source.fhe> --function <name> --inputs <a,b>".into());
            }
            cli_run::run(args)
        }
        "profile" => cli_profile::run(&args[1..]),
        "bench" => run_bench(&args[1..]),
        other => Err(format!("Unknown command: {other}. Run fhec --help for usage.").into()),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    dispatch(&args)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FHEC error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod input_width_tests {
    use super::*;

    #[test]
    fn encrypted_u8_parameters_account_for_sixteen_bits() {
        let source =
            "fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }";

        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        let compiler = Compiler::new();
        let module = compiler.lower(&program).unwrap();

        let function = &module.functions[0];
        let circuit = compiler.try_lower_to_circuit(function).unwrap();

        circuit.verify(function).unwrap();

        assert_eq!(circuit.inputs.len(), 2);
        assert_eq!(circuit_input_bit_count(&circuit), 16);
        assert_eq!(circuit.outputs.len(), 8);
    }
}
