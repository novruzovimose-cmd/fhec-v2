use fhec_analyzer::v4::V4CostModel;
use fhec_frontend::{lexer::Lexer, parser::Parser};

use crate::Compiler;

fn expected(operation: &str, a: u8, b: u8) -> Vec<bool> {
    let (number, width) = match operation {
        "+" => (a.wrapping_add(b), 8),
        "-" => (a.wrapping_sub(b), 8),
        "*" => (a.wrapping_mul(b), 8),
        "==" => (u8::from(a == b), 1),
        ">" => (u8::from(a > b), 1),
        _ => panic!("unsupported test operation"),
    };
    (0..width).map(|bit| (number >> bit) & 1 == 1).collect()
}

fn inputs(a: u8, b: u8) -> Vec<bool> {
    (0..8)
        .map(|bit| ((a >> bit) & 1) != 0)
        .chain((0..8).map(|bit| ((b >> bit) & 1) != 0))
        .collect()
}

#[test]
fn varied_hardware_models_never_select_worse_predicted_candidate() {
    let profiles = [
        (1.0, 1.0, 1.0, 1.5, 1usize),
        (4.0, 8.0, 2.0, 25.0, 4usize),
        (1.5, 1.0, 6.0, 12.0, 12usize),
    ];

    for operation in ["+", "-", "*", "==", ">"] {
        let result_type = if operation == "==" || operation == ">" {
            "bool"
        } else {
            "u8"
        };
        let source = format!(
            "fn check(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<{result_type}> {{ return a {operation} b; }}"
        );
        let tokens = Lexer::new(&source).tokenize().expect("lexing");
        let program = Parser::new(tokens).parse().expect("parsing");
        let compiler = Compiler::new();
        let module = compiler.lower(&program).expect("IR lowering");
        let function = &module.functions[0];

        for (and_ns, xor_ns, or_ns, mux_ns, workers) in profiles {
            let model = V4CostModel {
                and_ns,
                xor_ns,
                or_ns,
                mux_ns,
                hardware: fhec_analyzer::v4::HardwareProfile {
                    worker_threads: workers,
                    ..Default::default()
                },
                ..Default::default()
            };

            let selection = compiler
                .lower_to_circuit_v4(function, &model)
                .expect("candidate selection");
            let comparison = &selection.comparison;
            assert_eq!(comparison.candidates.len(), 4);
            assert!(comparison.selected_index < comparison.candidates.len());

            let selected = &comparison.candidates[comparison.selected_index];
            let selected_cost = selected.analysis.selected_ns();
            assert!(selected_cost.is_finite() && selected_cost >= 0.0);
            for candidate in &comparison.candidates {
                let cost = candidate.analysis.selected_ns();
                assert!(cost.is_finite() && cost >= 0.0);
                assert!(
                    selected_cost <= cost + 1e-9,
                    "op={operation}, workers={workers}, selected={} ({selected_cost}) vs {} ({cost})",
                    selected.name,
                    candidate.name,
                );
            }

            let best_baseline = comparison
                .candidates
                .iter()
                .filter(|candidate| !candidate.name.ends_with("-optimized"))
                .map(|candidate| candidate.analysis.selected_ns())
                .min_by(f64::total_cmp)
                .unwrap();
            assert!(selected_cost <= best_baseline + 1e-9);
            assert_eq!(selection.optimized, selected.name.ends_with("-optimized"));
            selection
                .circuit_function
                .verify(function)
                .expect("circuit verification");

            for (a, b) in [(0u8, 1u8), (37, 5), (255, 255)] {
                let result = selection
                    .circuit_function
                    .circuit
                    .evaluate(&inputs(a, b), &selection.circuit_function.outputs)
                    .expect("circuit evaluation");
                assert_eq!(
                    result,
                    expected(operation, a, b),
                    "operation={operation}, a={a}, b={b}, workers={workers}"
                );
            }
        }
    }
}
