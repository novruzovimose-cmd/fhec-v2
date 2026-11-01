use std::error::Error;
use std::fs;

use fhec_analyzer::v4::PredictionTracker;
use fhec_backend::FheBackend;
use fhec_backend_tfhe::TfheBackend;
use fhec_backend_tfhe::profile::{V4ProfileLoad, load_v4_profile};
use fhec_bench::measurement_quality::summarize_measurements;
use fhec_bench::prediction_accuracy::{CandidateTiming, analyze_prediction_accuracy};
use fhec_compiler::Compiler;
use fhec_frontend::{lexer::Lexer, parser::Parser};
use fhec_runtime::v4::execute_auto_v4_profiled;

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.total_cmp(b));
    let n = samples.len();
    if n % 2 == 0 {
        (samples[n / 2 - 1] + samples[n / 2]) / 2.0
    } else {
        samples[n / 2]
    }
}

fn decode(bits: &[bool]) -> u16 {
    bits.iter().enumerate().fold(
        0u16,
        |value, (index, bit)| {
            if *bit { value | (1u16 << index) } else { value }
        },
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut iterations = 3usize;
    let mut warmup = 1usize;
    let mut output = String::from("target/fhec-benchmarks");
    let mut profile_path = String::from("target/fhec-profiles/tfhe-v4.json");

    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--mode" => {
                index += 1;
                if args.get(index).map(String::as_str) != Some("compare") {
                    return Err("expected --mode compare".into());
                }
            }
            "--iterations" => {
                index += 1;
                iterations = args.get(index).ok_or("missing iterations")?.parse()?;
            }
            "--warmup" => {
                index += 1;
                warmup = args.get(index).ok_or("missing warmup value")?.parse()?;
            }
            "--profile" => {
                index += 1;
                profile_path = args.get(index).ok_or("missing profile path")?.clone();
            }
            "--output" => {
                index += 1;
                output = args.get(index).ok_or("missing output path")?.clone();
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
        index += 1;
    }

    if !(1..=20).contains(&iterations) {
        return Err("iterations must be between 1 and 20".into());
    }

    if warmup > 5 {
        return Err("warmup must be between 0 and 5".into());
    }

    let backend = TfheBackend::new();
    let model = match load_v4_profile(&profile_path)? {
        V4ProfileLoad::Loaded(calibration) => calibration.cost_model(),
        status => {
            return Err(format!(
                "hardware profile is not usable: {status:?} at {profile_path}. Run fhec profile calibrate first."
            )
            .into());
        }
    };

    println!("Benchmark hardware profile: {profile_path}");
    println!("Calibration source: verified persistent profile");

    let compiler = Compiler::new();

    let cases = [
        (
            "addition_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }",
            42u16,
        ),
        (
            "nested_addition_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return (a + b) + b; }",
            47u16,
        ),
        (
            "subtraction_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a - b; }",
            32u16,
        ),
        (
            "multiplication_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a * b; }",
            185u16,
        ),
        (
            "greater_than_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return a > b; }",
            1u16,
        ),
        (
            "equality_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return a == b; }",
            0u16,
        ),
        (
            "nested_subtraction_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return (a - b) - b; }",
            27u16,
        ),
        (
            "nested_addition_three_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return ((a + b) + b) + b; }",
            52u16,
        ),
        (
            "subtraction_reverse_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return b - a; }",
            224u16,
        ),
        (
            "multiplication_plus_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return (a * b) + b; }",
            190u16,
        ),
        (
            "multiplication_nested_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return (a + b) * b; }",
            210u16,
        ),
        (
            "greater_than_reverse_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return b > a; }",
            0u16,
        ),
        (
            "addition_commuted_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return b + a; }",
            42u16,
        ),
    ];

    let plain_inputs = (0..8)
        .map(|i| ((37u8 >> i) & 1) != 0)
        .chain((0..8).map(|i| ((5u8 >> i) & 1) != 0))
        .collect::<Vec<_>>();

    let encrypted_inputs = plain_inputs
        .iter()
        .map(|bit| backend.encrypt_bool(*bit))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("encryption failed: {e:?}"))?;

    let mut csv = String::from("program,implementation,iterations,median_tfhe_ms,output_bits\n");
    let mut quality_csv = String::from(
        "program,implementation,samples,median_ms,min_ms,max_ms,cv_percent,high_variability\n",
    );

    let mut accuracy_csv = String::from(
        "program,predicted_winner,measured_winner,ranking_correct,mean_error_percent,max_error_percent,selection_penalty_percent\n",
    );

    let mut raw_csv = String::from("program,implementation,iteration,measured_ms,predicted_ms\n");

    for (name, source, expected) in cases {
        let tokens = Lexer::new(source)
            .tokenize()
            .map_err(|e| format!("lexer: {e:?}"))?;

        let program = Parser::new(tokens)
            .parse()
            .map_err(|e| format!("parser: {e:?}"))?;

        let module = compiler
            .lower(&program)
            .map_err(|e| format!("lowering: {e:?}"))?;

        let function = module
            .functions
            .last()
            .ok_or("missing benchmark function")?;

        let candidates = compiler.lower_to_circuit_candidates(function);

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .map_err(|e| format!("candidate analysis failed: {e:?}"))?;

        let mut results = Vec::new();
        let mut accuracy_timings = Vec::new();

        for candidate in candidates {
            let circuit = &candidate.circuit_function;

            circuit
                .verify(function)
                .map_err(|e| format!("verification: {e:?}"))?;

            for _ in 0..warmup {
                let mut tracker = PredictionTracker::default();

                let (encrypted_output, _) = execute_auto_v4_profiled(
                    &backend,
                    &circuit.circuit,
                    &encrypted_inputs,
                    &circuit.outputs,
                    &model,
                    &mut tracker,
                )
                .map_err(|e| format!("warmup execution: {e:?}"))?;

                let decrypted = encrypted_output
                    .iter()
                    .map(|ct| backend.decrypt_bool(ct))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("warmup decryption: {e:?}"))?;

                if decode(&decrypted) != expected {
                    return Err(format!("{name}: incorrect warmup result").into());
                }
            }

            let mut timings = Vec::with_capacity(iterations);

            for _ in 0..iterations {
                let mut tracker = PredictionTracker::default();

                let (encrypted_output, profile) = execute_auto_v4_profiled(
                    &backend,
                    &circuit.circuit,
                    &encrypted_inputs,
                    &circuit.outputs,
                    &model,
                    &mut tracker,
                )
                .map_err(|e| format!("execution: {e:?}"))?;

                let decrypted = encrypted_output
                    .iter()
                    .map(|ct| backend.decrypt_bool(ct))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("decryption: {e:?}"))?;

                let result = decode(&decrypted);

                if result != expected {
                    return Err(format!("{name}: expected {expected}, got {result}").into());
                }

                timings.push(profile.elapsed_ns / 1_000_000.0);
            }

            let quality = summarize_measurements(&timings)?;
            let raw_timings = timings.clone();
            let median_ms = median(&mut timings);
            let implementation = candidate.implementation.name();

            quality_csv.push_str(&format!(
                "{name},{implementation},{},{:.6},{:.6},{:.6},{:.6},{}\n",
                quality.samples,
                quality.median_ms,
                quality.minimum_ms,
                quality.maximum_ms,
                quality.cv_percent,
                quality.high_variability,
            ));

            println!(
                "{name} / {implementation}: variability={:.2}% CV, samples={}",
                quality.cv_percent, quality.samples
            );

            if quality.high_variability {
                eprintln!("WARNING: unstable measurement for {name} / {implementation}");
            }

            let predicted_ns = selection
                .comparison
                .candidates
                .iter()
                .find(|entry| entry.name == implementation)
                .ok_or_else(|| format!("missing prediction for {implementation}"))?
                .analysis
                .selected_ns();

            accuracy_timings.push(CandidateTiming {
                name: implementation.to_string(),
                predicted_ns,
                measured_ns: median_ms * 1_000_000.0,
            });

            for (sample_index, measured_ms) in raw_timings.iter().enumerate() {
                raw_csv.push_str(&format!(
                    "{name},{implementation},{},{measured_ms:.6},{:.6}\n",
                    sample_index + 1,
                    predicted_ns / 1_000_000.0
                ));
            }

            println!(
                "{name} / {implementation}: measured={median_ms:.3} ms, predicted={:.3} ms",
                predicted_ns / 1_000_000.0
            );

            csv.push_str(&format!(
                "{name},{implementation},{iterations},{median_ms:.6},{}\n",
                circuit.outputs.len()
            ));

            results.push((implementation.to_string(), median_ms));
        }

        if results.len() == 2 {
            let ratio = results[0].1 / results[1].1;

            println!(
                "{name}: {} / {} latency ratio = {ratio:.3}x",
                results[0].0, results[1].0
            );
        }

        let report = analyze_prediction_accuracy(&accuracy_timings)?;

        println!(
            "{name}: predicted winner={}, measured winner={}, ranking correct={}",
            report.predicted_winner, report.measured_winner, report.ranking_correct
        );

        println!(
            "{name}: mean error={:.2}%, max error={:.2}%, selection penalty={:.2}%",
            report.mean_percentage_error,
            report.max_percentage_error,
            report.selection_penalty_percent
        );

        accuracy_csv.push_str(&format!(
            "{name},{},{},{},{:.6},{:.6},{:.6}\n",
            report.predicted_winner,
            report.measured_winner,
            report.ranking_correct,
            report.mean_percentage_error,
            report.max_percentage_error,
            report.selection_penalty_percent,
        ));
    }

    fs::create_dir_all(&output)?;
    fs::write(format!("{output}/tfhe_comparison.csv"), csv)?;
    fs::write(
        format!("{output}/tfhe_measurement_quality.csv"),
        quality_csv,
    )?;
    fs::write(
        format!("{output}/tfhe_prediction_accuracy.csv"),
        accuracy_csv,
    )?;

    fs::write(format!("{output}/tfhe_raw_measurements.csv"), raw_csv)?;

    println!("Comparison saved to {output}/tfhe_comparison.csv");
    Ok(())
}
