use fhec_analyzer::v4::V4CostModel;
use fhec_frontend::{lexer::Lexer, parser::Parser};

use crate::Compiler;

fn selection_for(operation: &str) -> crate::v4::CircuitSelection {
    let return_type = if matches!(operation, "==" | "!=" | "<" | "<=" | ">" | ">=") {
        "bool"
    } else {
        "u8"
    };

    let source = format!(
        "fn check(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<{return_type}> {{ return a {operation} b; }}"
    );

    let tokens = Lexer::new(&source).tokenize().expect("tokenization failed");

    let program = Parser::new(tokens).parse().expect("parsing failed");

    let compiler = Compiler::new();

    let module = compiler.lower(&program).expect("IR lowering failed");

    compiler
        .lower_to_circuit_v4(&module.functions[0], &V4CostModel::default())
        .expect("candidate selection failed")
}

#[test]
fn selection_chooses_minimum_predicted_latency() {
    for operation in ["+", "-", "*", "==", "!=", "<", "<=", ">", ">="] {
        let selection = selection_for(operation);
        let comparison = &selection.comparison;

        assert_eq!(comparison.candidates.len(), 4);

        let selected = &comparison.candidates[comparison.selected_index];

        let selected_ns = selected.analysis.selected_ns();

        assert!(
            selected_ns.is_finite() && selected_ns >= 0.0,
            "invalid selected latency for {operation}"
        );

        for candidate in &comparison.candidates {
            let candidate_ns = candidate.analysis.selected_ns();

            assert!(
                candidate_ns.is_finite() && candidate_ns >= 0.0,
                "invalid candidate latency: {}",
                candidate.name
            );

            assert!(
                selected_ns <= candidate_ns,
                "operation={operation}: selected={} ({selected_ns} ns), faster={} ({candidate_ns} ns)",
                selected.name,
                candidate.name,
            );
        }
    }
}

#[test]
fn selection_preserves_baseline_fallback() {
    for operation in ["+", "-", "*", "==", "!=", "<", "<=", ">", ">="] {
        let selection = selection_for(operation);
        let comparison = &selection.comparison;

        let baseline_best = comparison
            .candidates
            .iter()
            .filter(|candidate| !candidate.name.ends_with("-optimized"))
            .map(|candidate| candidate.analysis.selected_ns())
            .min_by(f64::total_cmp)
            .expect("baseline candidates missing");

        let selected = &comparison.candidates[comparison.selected_index];

        let selected_ns = selected.analysis.selected_ns();

        assert!(
            selected_ns <= baseline_best,
            "operation={operation}: selected candidate is slower than the best baseline"
        );

        assert_eq!(
            selection.optimized,
            selected.name.ends_with("-optimized"),
            "optimization flag disagrees with selected candidate"
        );

        assert!(
            selected.name.starts_with(selection.implementation.name()),
            "implementation disagrees with candidate name"
        );

        let summary = selection.decision_summary();

        assert_eq!(summary.selected_name, selected.name);
        assert_eq!(summary.selected_predicted_ns, selected_ns);
    }
}
