use std::error::Error;
use std::fs;

use fhec_analyzer::v4::PredictionTracker;
use fhec_backend::FheBackend;
use fhec_backend_tfhe::TfheBackend;
use fhec_compiler::Compiler;
use fhec_frontend::{lexer::Lexer, parser::Parser};
use fhec_optimizer::optimize_function;
use fhec_runtime::v4::execute_auto_v4_profiled;

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn bits(value: u8) -> Vec<bool> {
    (0..8).map(|i| ((value >> i) & 1) != 0).collect()
}

fn decode(bits: &[bool]) -> u16 {
    bits.iter().enumerate().fold(
        0u16,
        |acc, (index, bit)| {
            if *bit { acc | (1u16 << index) } else { acc }
        },
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut iterations = 3usize;
    let mut output = String::from("target/fhec-benchmarks");

    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--mode" => {
                index += 1;
                if args.get(index).map(String::as_str) != Some("optimizer") {
                    return Err("expected --mode optimizer".into());
                }
            }
            "--iterations" => {
                index += 1;
                iterations = args.get(index).ok_or("missing iterations")?.parse()?;
            }
            "--output" => {
                index += 1;
                output = args.get(index).ok_or("missing output")?.clone();
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
        index += 1;
    }

    if !(1..=20).contains(&iterations) {
        return Err("iterations must be between 1 and 20".into());
    }

    let cases = [
        (
            "addition_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }",
            42u16,
        ),
        (
            "repeated_addition_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return (a + b) + (a + b); }",
            84u16,
        ),
        (
            "nested_calls_u8",
            "fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; } fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return add(add(a, b), b); }",
            47u16,
        ),
        (
            "mux_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { a } else { b }; }",
            37u16,
        ),
    ];

    let compiler = Compiler::new();
    let backend = TfheBackend::new();

    let (_, model) = backend
        .calibrate_v4_cost_model(1)
        .map_err(|e| format!("calibration failed: {e:?}"))?;

    let inputs = bits(37).into_iter().chain(bits(5)).collect::<Vec<_>>();

    let encrypted_inputs = inputs
        .into_iter()
        .map(|value| backend.encrypt_bool(value))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("encryption failed: {e:?}"))?;

    let mut csv = String::from("program,variant,iterations,median_tfhe_ms,output_bits\n");

    for (name, source, expected) in cases {
        let tokens = Lexer::new(source)
            .tokenize()
            .map_err(|e| format!("lexing: {e:?}"))?;

        let program = Parser::new(tokens)
            .parse()
            .map_err(|e| format!("parsing: {e:?}"))?;

        let module = compiler
            .lower(&program)
            .map_err(|e| format!("lowering: {e:?}"))?;

        let baseline = module.functions.last().ok_or("missing function")?.clone();

        let mut optimized = baseline.clone();
        optimize_function(&mut optimized);

        let variants = [("baseline", &baseline), ("optimized", &optimized)];

        let mut measured = Vec::new();

        for (variant_name, function) in variants {
            let candidates = compiler.lower_to_circuit_candidates(function);

            let circuit = &candidates
                .first()
                .ok_or("no circuit candidates")?
                .circuit_function;

            circuit
                .verify(function)
                .map_err(|e| format!("circuit verification: {e:?}"))?;

            let mut samples = Vec::with_capacity(iterations);

            for _ in 0..iterations {
                let mut tracker = PredictionTracker::default();

                let (encrypted_outputs, profile) = execute_auto_v4_profiled(
                    &backend,
                    &circuit.circuit,
                    &encrypted_inputs,
                    &circuit.outputs,
                    &model,
                    &mut tracker,
                )
                .map_err(|e| format!("TFHE execution: {e:?}"))?;

                let decrypted = encrypted_outputs
                    .iter()
                    .map(|ct| backend.decrypt_bool(ct))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("decryption: {e:?}"))?;

                let actual = decode(&decrypted);

                if actual != expected {
                    return Err(format!(
                        "{name}/{variant_name}: expected {expected}, got {actual}"
                    )
                    .into());
                }

                samples.push(profile.elapsed_ns / 1_000_000.0);
            }

            let value = median(&mut samples);

            println!("{name} / {variant_name}: {value:.3} ms");

            csv.push_str(&format!(
                "{name},{variant_name},{iterations},{value:.6},{}\n",
                circuit.outputs.len()
            ));

            measured.push(value);
        }

        let ratio = measured[0] / measured[1];

        println!("{name}: optimized speedup = {ratio:.3}x");
    }

    fs::create_dir_all(&output)?;
    fs::write(format!("{output}/optimizer_results.csv"), csv)?;

    println!("Optimizer report saved to {output}/optimizer_results.csv");
    Ok(())
}
