use fhec_analyzer::v4::V4CostModel;
use fhec_frontend::{lexer::Lexer, parser::Parser};

use crate::Compiler;

fn compile_case(operation: &str) -> (fhec_ir::Module, Compiler) {
    let result_type = if matches!(operation, "==" | "!=" | "<" | "<=" | ">" | ">=") {
        "bool"
    } else {
        "u8"
    };

    let source = format!(
        "fn check(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<{result_type}> {{ return a {operation} b; }}"
    );

    let tokens = Lexer::new(&source).tokenize().expect("tokenization failed");

    let program = Parser::new(tokens).parse().expect("parsing failed");

    let compiler = Compiler::new();

    let module = compiler.lower(&program).expect("IR lowering failed");

    (module, compiler)
}

fn expected_output(operation: &str, a: u8, b: u8) -> Vec<bool> {
    let (value, width) = match operation {
        "+" => (a.wrapping_add(b), 8),
        "-" => (a.wrapping_sub(b), 8),
        "*" => (a.wrapping_mul(b), 8),
        "==" => (u8::from(a == b), 1),
        "<" => (u8::from(a < b), 1),
        ">=" => (u8::from(a >= b), 1),
        _ => panic!("unsupported test operation: {operation}"),
    };

    (0..width).map(|bit| ((value >> bit) & 1) != 0).collect()
}

fn input_bits(a: u8, b: u8) -> Vec<bool> {
    (0..8)
        .map(|bit| ((a >> bit) & 1) != 0)
        .chain((0..8).map(|bit| ((b >> bit) & 1) != 0))
        .collect()
}

#[test]
fn optimizer_effectiveness_report() {
    for operation in ["+", "-", "*", "==", "<", ">="] {
        let (module, compiler) = compile_case(operation);
        let function = &module.functions[0];

        let selection = compiler
            .lower_to_circuit_v4(function, &V4CostModel::default())
            .expect("candidate selection failed");

        let candidates = &selection.comparison.candidates;

        assert_eq!(candidates.len(), 4);

        println!("\nOPERATION: {operation}");

        for candidate in candidates {
            let latency = candidate.analysis.selected_ns();
            let gates = candidate.analysis.structural.gate_count;
            let depth = candidate.analysis.structural.max_depth;

            assert!(
                latency.is_finite() && latency >= 0.0,
                "invalid predicted latency for {}",
                candidate.name
            );

            println!(
                "  {:24} gates={:6} depth={:5} predicted_ns={:12.2}",
                candidate.name, gates, depth, latency
            );
        }

        for index in 0..2 {
            let baseline = &candidates[index];
            let optimized = &candidates[index + 2];

            let gate_delta = optimized.analysis.structural.gate_count as i64
                - baseline.analysis.structural.gate_count as i64;

            let depth_delta = optimized.analysis.structural.max_depth as i64
                - baseline.analysis.structural.max_depth as i64;

            let latency_delta = optimized.analysis.selected_ns() - baseline.analysis.selected_ns();

            println!(
                "  DELTA {:20} gates={:+} depth={:+} latency_ns={:+.2}",
                baseline.name, gate_delta, depth_delta, latency_delta
            );
        }

        println!(
            "  SELECTED: {}",
            candidates[selection.comparison.selected_index].name
        );
    }
}

#[test]
fn all_candidates_preserve_reference_results() {
    let input_pairs = [
        (0u8, 0u8),
        (1, 1),
        (7, 13),
        (37, 5),
        (127, 128),
        (128, 127),
        (254, 255),
        (255, 255),
    ];

    for operation in ["+", "-", "*", "==", "<", ">="] {
        let (module, compiler) = compile_case(operation);
        let function = &module.functions[0];

        let baseline = compiler.lower_to_circuit_candidates(function);

        assert_eq!(baseline.len(), 2);

        let selection = compiler
            .lower_to_circuit_v4(function, &V4CostModel::default())
            .expect("candidate selection failed");

        selection
            .circuit_function
            .verify(function)
            .expect("selected circuit verification failed");

        for (a, b) in input_pairs {
            let inputs = input_bits(a, b);
            let expected = expected_output(operation, a, b);

            for candidate in &baseline {
                let circuit = &candidate.circuit_function;

                circuit
                    .verify(function)
                    .expect("baseline circuit verification failed");

                let actual = circuit
                    .circuit
                    .evaluate(&inputs, &circuit.outputs)
                    .expect("baseline evaluation failed");

                assert_eq!(
                    actual, expected,
                    "baseline mismatch: op={operation}, a={a}, b={b}"
                );
            }

            let circuit = &selection.circuit_function;

            let actual = circuit
                .circuit
                .evaluate(&inputs, &circuit.outputs)
                .expect("selected circuit evaluation failed");

            assert_eq!(
                actual, expected,
                "selected mismatch: op={operation}, a={a}, b={b}"
            );
        }
    }
}
