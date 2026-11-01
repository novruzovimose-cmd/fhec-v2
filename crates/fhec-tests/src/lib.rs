#[cfg(test)]
mod tests {
    use std::time::Instant;

    use fhec_analyzer::v4::{PredictionTracker, V4CostModel};

    use fhec_backend::FheBackend;

    use fhec_backend_tfhe::TfheBackend;

    use fhec_compiler::{CircuitImplementation, Compiler, render_compiler_v4_report};

    use fhec_frontend::{lexer::Lexer, parser::Parser};

    use fhec_runtime::v4::execute_auto_v4_profiled;

    fn bits_u8(value: u8) -> Vec<bool> {
        (0..8).map(|index| ((value >> index) & 1) == 1).collect()
    }

    fn from_bits_u8(bits: &[bool]) -> u8 {
        bits.iter().enumerate().fold(
            0u8,
            |value, (index, bit)| {
                if *bit { value | (1 << index) } else { value }
            },
        )
    }

    #[test]
    fn real_tfhe_executes_encrypted_subtraction() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("compiler selection failed");

        selection
            .circuit_function
            .verify(function)
            .expect("selected circuit verification failed");

        let plaintext_inputs = bits_u8(37)
            .into_iter()
            .chain(bits_u8(5))
            .collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, _) = execute_auto_v4_profiled(
            &backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            &model,
            &mut tracker,
        )
        .expect("TFHE subtraction execution failed");

        let decrypted_bits = encrypted_outputs
            .iter()
            .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
            .collect::<Vec<_>>();

        let result = from_bits_u8(&decrypted_bits);

        assert_eq!(result, 32);
    }

    #[test]
    fn real_tfhe_executes_subtraction_with_borrow() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("compiler selection failed");

        let plaintext_inputs = bits_u8(16)
            .into_iter()
            .chain(bits_u8(1))
            .collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, _) = execute_auto_v4_profiled(
            &backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            &model,
            &mut tracker,
        )
        .expect("TFHE borrow execution failed");

        let decrypted_bits = encrypted_outputs
            .iter()
            .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
            .collect::<Vec<_>>();

        assert_eq!(from_bits_u8(&decrypted_bits), 15);
    }
    #[test]
    fn real_tfhe_executes_encrypted_multiplication() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("compiler selection failed");

        selection
            .circuit_function
            .verify(function)
            .expect("selected circuit verification failed");

        let plaintext_inputs = bits_u8(6).into_iter().chain(bits_u8(7)).collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, _) = execute_auto_v4_profiled(
            &backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            &model,
            &mut tracker,
        )
        .expect("TFHE multiplication execution failed");

        let decrypted_bits = encrypted_outputs
            .iter()
            .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
            .collect::<Vec<_>>();

        assert_eq!(from_bits_u8(&decrypted_bits), 42);
    }

    #[test]
    fn real_tfhe_multiplication_wraps_at_u8_width() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("compiler selection failed");

        let plaintext_inputs = bits_u8(20)
            .into_iter()
            .chain(bits_u8(20))
            .collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, _) = execute_auto_v4_profiled(
            &backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            &model,
            &mut tracker,
        )
        .expect("TFHE multiplication execution failed");

        let decrypted_bits = encrypted_outputs
            .iter()
            .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
            .collect::<Vec<_>>();

        assert_eq!(from_bits_u8(&decrypted_bits), 144);
    }
    fn execute_real_tfhe_comparison(
        backend: &TfheBackend,
        model: &V4CostModel,
        operator: &str,
        lhs: u8,
        rhs: u8,
    ) -> bool {
        let source = format!(
            r#"
            fn compare(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<bool> {{
                return a {} b;
            }}
        "#,
            operator
        );

        let tokens = Lexer::new(&source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let selection = compiler
            .lower_to_circuit_v4(function, model)
            .expect("compiler selection failed");

        selection
            .circuit_function
            .verify(function)
            .expect("comparison circuit verification failed");

        assert_eq!(selection.circuit_function.outputs.len(), 1);

        let plaintext_inputs = bits_u8(lhs)
            .into_iter()
            .chain(bits_u8(rhs))
            .collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, _) = execute_auto_v4_profiled(
            backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            model,
            &mut tracker,
        )
        .expect("TFHE comparison execution failed");

        assert_eq!(encrypted_outputs.len(), 1);

        backend
            .decrypt_bool(&encrypted_outputs[0])
            .expect("decryption failed")
    }

    #[test]
    fn real_tfhe_executes_all_comparison_operators() {
        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        assert!(execute_real_tfhe_comparison(&backend, &model, "==", 42, 42,));

        assert!(execute_real_tfhe_comparison(&backend, &model, "!=", 42, 41,));

        assert!(execute_real_tfhe_comparison(&backend, &model, ">", 200, 12,));

        assert!(execute_real_tfhe_comparison(&backend, &model, ">=", 42, 42,));

        assert!(execute_real_tfhe_comparison(&backend, &model, "<", 12, 200,));

        assert!(execute_real_tfhe_comparison(&backend, &model, "<=", 42, 42,));
    }

    #[test]
    fn real_tfhe_comparisons_handle_u8_boundaries() {
        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        assert!(execute_real_tfhe_comparison(&backend, &model, "<", 0, 255,));

        assert!(execute_real_tfhe_comparison(&backend, &model, ">", 255, 0,));

        assert!(execute_real_tfhe_comparison(
            &backend, &model, "==", 255, 255,
        ));

        assert!(!execute_real_tfhe_comparison(
            &backend, &model, "==", 0, 255,
        ));
    }
    #[test]
    fn real_tfhe_executes_nested_encrypted_expression() {
        let source = r#"
            fn nested(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<u8> {
                return (a + b) * (c - d);
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("compiler selection failed");

        selection
            .circuit_function
            .verify(function)
            .expect("nested-expression circuit verification failed");

        let plaintext_inputs = bits_u8(3)
            .into_iter()
            .chain(bits_u8(4))
            .chain(bits_u8(9))
            .chain(bits_u8(2))
            .collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, _) = execute_auto_v4_profiled(
            &backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            &model,
            &mut tracker,
        )
        .expect("nested-expression TFHE execution failed");

        let decrypted_bits = encrypted_outputs
            .iter()
            .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
            .collect::<Vec<_>>();

        assert_eq!(from_bits_u8(&decrypted_bits), 49);
    }

    #[test]
    fn real_tfhe_executes_nested_encrypted_conditional() {
        let source = r#"
            fn nested_select(
                first: encrypted<bool>,
                second: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>
            ) -> encrypted<u8> {
                return if first {
                    if second {
                        a
                    } else {
                        b
                    }
                } else {
                    c
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("compiler selection failed");

        selection
            .circuit_function
            .verify(function)
            .expect("nested-conditional circuit verification failed");

        let plaintext_inputs = [true, false]
            .into_iter()
            .chain(bits_u8(10))
            .chain(bits_u8(20))
            .chain(bits_u8(30))
            .collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, _) = execute_auto_v4_profiled(
            &backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            &model,
            &mut tracker,
        )
        .expect("nested-conditional TFHE execution failed");

        let decrypted_bits = encrypted_outputs
            .iter()
            .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
            .collect::<Vec<_>>();

        assert_eq!(from_bits_u8(&decrypted_bits), 20);
    }
    #[test]
    fn v4_default_model_remains_available() {
        let model = V4CostModel::default();

        assert!(!model.is_calibrated());

        assert!(!model.has_empirical_scaling());

        assert_eq!(model.ciphertext_bytes, 0);
    }

    #[test]
    fn v4_compiler_generates_valid_candidates() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let candidates = compiler.lower_to_circuit_candidates(function);

        assert_eq!(candidates.len(), 2);

        for candidate in &candidates {
            candidate
                .circuit_function
                .verify(function)
                .expect("candidate verification failed");
        }
    }

    #[test]
    fn v4_compiler_selects_and_benchmarks_real_tfhe_candidates() {
        let total_start = Instant::now();

        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        let function = &module.functions[0];

        let keygen_start = Instant::now();

        let backend = TfheBackend::new();

        let keygen_time = keygen_start.elapsed();

        let calibration_start = Instant::now();

        let (calibration, model) = backend
            .calibrate_v4_cost_model(2)
            .expect("TFHE calibration failed");

        let calibration_time = calibration_start.elapsed();

        assert!(calibration.ciphertext_bytes > 0);

        assert!(model.ciphertext_bytes > 0);

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("compiler selection failed");

        for candidate in &selection.comparison.candidates {
            assert!(candidate.analysis.memory.produced_ciphertexts > 0);

            assert!(candidate.analysis.memory.peak_live_ciphertexts > 0);

            assert!(candidate.analysis.memory.peak_live_bytes > 0);

            assert!(candidate.analysis.memory.total_ciphertext_bytes_created > 0);
        }

        println!();

        println!("{}", render_compiler_v4_report(&selection));

        println!(
            "TFHE serialized ciphertext size: {} bytes",
            calibration.ciphertext_bytes
        );

        println!();

        let mut candidates = compiler.lower_to_circuit_candidates(function);

        let plaintext_inputs = bits_u8(37)
            .into_iter()
            .chain(bits_u8(5))
            .collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut actual_results = Vec::new();

        for candidate in candidates.drain(..) {
            candidate
                .circuit_function
                .verify(function)
                .expect("candidate circuit verification failed");

            let mut tracker = PredictionTracker::default();

            let (encrypted_outputs, profile) = execute_auto_v4_profiled(
                &backend,
                &candidate.circuit_function.circuit,
                &encrypted_inputs,
                &candidate.circuit_function.outputs,
                &model,
                &mut tracker,
            )
            .expect("candidate execution failed");

            let decrypted_bits = encrypted_outputs
                .iter()
                .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
                .collect::<Vec<_>>();

            let value = from_bits_u8(&decrypted_bits);

            assert_eq!(value, 42);

            let analyzed_candidate = selection
                .comparison
                .candidates
                .iter()
                .find(|analyzed| analyzed.name == candidate.implementation.name())
                .expect("missing analyzed candidate");

            let prediction_interval = analyzed_candidate.analysis.prediction_interval;

            assert!(prediction_interval.lower_ns <= analyzed_candidate.analysis.selected_ns());

            assert!(prediction_interval.upper_ns >= analyzed_candidate.analysis.selected_ns());

            assert!(prediction_interval.relative_margin > 0.0);

            let inside_prediction_interval = prediction_interval.contains(profile.elapsed_ns);

            println!();

            println!("{} REAL EXECUTION", candidate.implementation.name());

            println!("======================");

            println!(
                "Predicted: {:.3} ms",
                profile.prediction.predicted_ns / 1_000_000.0
            );
            println!(
                "Prediction interval: {:.3}-{:.3} ms",
                prediction_interval.lower_milliseconds(),
                prediction_interval.upper_milliseconds()
            );

            println!(
                "Inside prediction interval: {}",
                if inside_prediction_interval {
                    "YES"
                } else {
                    "NO"
                }
            );

            println!("Actual: {:.3} ms", profile.elapsed_ns / 1_000_000.0);

            println!("Prediction error: {:.2}%", profile.prediction.error_percent);

            println!("Runtime strategy: {:?}", profile.decision.strategy);

            println!("Result: {}", value);

            actual_results.push((candidate.implementation, profile.elapsed_ns));
        }

        let actual_winner = actual_results
            .iter()
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|value| value.0)
            .expect("missing actual result");

        let decision_correct = actual_winner == selection.implementation;

        println!();

        println!("V5.6 DECISION");

        println!("=============");

        println!("Predicted winner: {}", selection.implementation.name());

        println!("Actual winner: {}", actual_winner.name());

        println!(
            "Compiler decision correct: {}",
            if decision_correct { "YES" } else { "NO" }
        );

        println!();

        println!("Calibration widths: {:?}", calibration.calibration_widths);
        println!(
            "Calibration variability: {:.2}%",
            calibration.calibration_relative_mad * 100.0
        );

        println!("Ciphertext size: {} bytes", calibration.ciphertext_bytes);

        println!("Key generation: {:?}", keygen_time);

        println!("Calibration: {:?}", calibration_time);

        println!("Total benchmark: {:?}", total_start.elapsed());

        assert_eq!(actual_results.len(), 2);
    }

    #[test]
    fn implementation_names_are_stable() {
        assert_eq!(
            CircuitImplementation::ParallelPrefix.name(),
            "parallel-prefix"
        );

        assert_eq!(CircuitImplementation::RippleCarry.name(), "ripple-carry");
    }
}

#[cfg(test)]
mod function_call_e2e_tests {
    use fhec_analyzer::v4::PredictionTracker;
    use fhec_backend::FheBackend;
    use fhec_backend_tfhe::TfheBackend;
    use fhec_compiler::Compiler;
    use fhec_frontend::{lexer::Lexer, parser::Parser};
    use fhec_runtime::v4::execute_auto_v4_profiled;

    fn bits(value: u8) -> Vec<bool> {
        (0..8).map(|index| ((value >> index) & 1) == 1).collect()
    }

    fn decode(bits: &[bool]) -> u8 {
        bits.iter().enumerate().fold(0u8, |result, (index, bit)| {
            if *bit {
                result | (1u8 << index)
            } else {
                result
            }
        })
    }

    #[test]
    fn real_tfhe_executes_nested_function_calls() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn calculate(
                x: encrypted<u8>,
                y: encrypted<u8>
            ) -> encrypted<u8> {
                return add(add(x, y), y);
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler
            .lower(&program)
            .expect("function-call lowering failed");

        let function = module
            .functions
            .iter()
            .find(|function| function.name == "calculate")
            .expect("calculate function missing");

        let backend = TfheBackend::new();

        let (_, model) = backend
            .calibrate_v4_cost_model(1)
            .expect("TFHE calibration failed");

        let selection = compiler
            .lower_to_circuit_v4(function, &model)
            .expect("circuit selection failed");

        selection
            .circuit_function
            .verify(function)
            .expect("circuit verification failed");

        assert_eq!(selection.circuit_function.inputs.len(), 2);
        assert_eq!(selection.circuit_function.outputs.len(), 8);

        let plaintext_inputs = bits(32).into_iter().chain(bits(5)).collect::<Vec<_>>();

        let encrypted_inputs = plaintext_inputs
            .into_iter()
            .map(|value| backend.encrypt_bool(value).expect("encryption failed"))
            .collect::<Vec<_>>();

        let mut tracker = PredictionTracker::default();

        let (encrypted_outputs, profile) = execute_auto_v4_profiled(
            &backend,
            &selection.circuit_function.circuit,
            &encrypted_inputs,
            &selection.circuit_function.outputs,
            &model,
            &mut tracker,
        )
        .expect("TFHE execution failed");

        let decrypted = encrypted_outputs
            .iter()
            .map(|ciphertext| backend.decrypt_bool(ciphertext).expect("decryption failed"))
            .collect::<Vec<_>>();

        assert_eq!(decode(&decrypted), 42);

        println!("Function calls executed successfully");
        println!("Decrypted result: {}", decode(&decrypted));
        println!("Runtime strategy: {:?}", profile.decision.strategy);
        println!(
            "Predicted latency: {:.3} ms",
            profile.prediction.predicted_ns / 1_000_000.0
        );
        println!("Actual latency: {:.3} ms", profile.elapsed_ns / 1_000_000.0);
    }

    #[test]
    fn nested_calls_compile_to_valid_circuit() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }

            fn calculate(
                x: encrypted<u8>,
                y: encrypted<u8>
            ) -> encrypted<u8> {
                return add(add(x, y), y);
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("inlining failed");

        let function = module
            .functions
            .iter()
            .find(|function| function.name == "calculate")
            .expect("calculate missing");

        let circuit = compiler.lower_to_circuit(function);

        circuit.verify(function).expect("invalid circuit");

        let mut inputs = bits(32);
        inputs.extend(bits(5));

        let outputs = circuit
            .circuit
            .evaluate(&inputs, &circuit.outputs)
            .expect("evaluation failed");

        assert_eq!(decode(&outputs), 42);
    }
}

#[cfg(test)]
mod release_correctness_tests;
