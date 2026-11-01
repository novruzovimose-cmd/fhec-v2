use std::error::Error;
use std::fs;

use fhec_analyzer::v4::PredictionTracker;
use fhec_backend::FheBackend;
use fhec_backend_tfhe::TfheBackend;
use fhec_compiler::Compiler;
use fhec_frontend::{lexer::Lexer, parser::Parser};
use fhec_runtime::v4::execute_auto_v4_profiled;

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.total_cmp(b));
    samples[samples.len() / 2]
}

fn bits(value: u8) -> Vec<bool> {
    (0..8).map(|bit| ((value >> bit) & 1) == 1).collect()
}

fn decoded(bits: &[bool]) -> u16 {
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
                if args.get(index).map(String::as_str) != Some("tfhe") {
                    return Err("supported mode: tfhe".into());
                }
            }
            "--iterations" => {
                index += 1;
                iterations = args.get(index).ok_or("missing iterations")?.parse()?;
            }
            "--output" => {
                index += 1;
                output = args.get(index).ok_or("missing output path")?.clone();
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
        index += 1;
    }
    if !(1..=50).contains(&iterations) {
        return Err("TFHE iterations must be between 1 and 50".into());
    }

    let backend = TfheBackend::new();
    let (_, model) = backend
        .calibrate_v4_cost_model(1)
        .map_err(|e| format!("calibration failed: {e:?}"))?;
    let compiler = Compiler::new();
    let cases = [
        (
            "add_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }",
            42u16,
        ),
        (
            "eq_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return a == b; }",
            0u16,
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
        (
            "comparison_mux_bool",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return if a > b { a == b } else { a < b }; }",
            0u16,
        ),
    ];
    let inputs = bits(37).into_iter().chain(bits(5)).collect::<Vec<_>>();
    let encrypted_inputs = inputs
        .into_iter()
        .map(|value| backend.encrypt_bool(value))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("encryption failed: {e:?}"))?;

    let mut csv = String::from(
        "name,iterations,actual_median_ms,predicted_median_ms,prediction_error_median_percent,output_bits\n",
    );
    let mut json = String::from("{\"benchmark_kind\":\"real_tfhe_execution\",\"cases\":[");
    for (case_index, (name, source, expected)) in cases.iter().enumerate() {
        let tokens = Lexer::new(source)
            .tokenize()
            .map_err(|e| format!("lexer: {e:?}"))?;
        let program = Parser::new(tokens)
            .parse()
            .map_err(|e| format!("parser: {e:?}"))?;
        let module = compiler
            .lower(&program)
            .map_err(|e| format!("lowering: {e:?}"))?;
        let function = module.functions.last().ok_or("no function generated")?;
        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .map_err(|e| format!("circuit lowering: {e:?}"))?;
        let circuit = &selection.circuit_function;
        circuit
            .verify(function)
            .map_err(|e| format!("verification: {e:?}"))?;
        let mut actual = Vec::with_capacity(iterations);
        let mut predicted = Vec::with_capacity(iterations);
        let mut errors = Vec::with_capacity(iterations);
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
            let value = decoded(&decrypted);
            if value != *expected {
                return Err(format!("{name}: expected {expected}, got {value}").into());
            }
            actual.push(profile.elapsed_ns / 1_000_000.0);
            predicted.push(profile.prediction.predicted_ns / 1_000_000.0);
            errors.push(profile.prediction.error_percent);
        }
        let actual_ms = median(actual);
        let predicted_ms = median(predicted);
        let error_percent = median(errors);
        println!(
            "{name}: TFHE median {actual_ms:.3} ms, predicted {predicted_ms:.3} ms, error {error_percent:.2}%"
        );
        csv.push_str(&format!(
            "{name},{iterations},{actual_ms:.6},{predicted_ms:.6},{error_percent:.4},{}\n",
            circuit.outputs.len()
        ));
        if case_index > 0 {
            json.push(',');
        }
        json.push_str(&format!("{{\"name\":\"{name}\",\"iterations\":{iterations},\"actual_median_ms\":{actual_ms:.6},\"predicted_median_ms\":{predicted_ms:.6},\"prediction_error_median_percent\":{error_percent:.4},\"output_bits\":{}}}", circuit.outputs.len()));
    }
    json.push_str("]}\n");
    fs::create_dir_all(&output)?;
    fs::write(format!("{output}/tfhe_results.csv"), csv)?;
    fs::write(format!("{output}/tfhe_results.json"), json)?;
    println!("TFHE reports written to {output}");
    Ok(())
}
