use std::fmt::Write;

use fhec_analyzer::attribution::{CostAttributionAnalysis, analyze_cost_attribution};

use fhec_analyzer::hints::generate_optimizer_hints;

use fhec_analyzer::v4::{
    CandidateComparison, CircuitCandidate, V4CostModel, V4ExecutionStrategy, compare_candidates,
};
use fhec_ir::Function;

use fhec_optimizer::{hints::build_optimization_plan, optimize_function_with_plan};

use crate::circuit_lowering::{
    AdderStrategy, CircuitFunction, CircuitLoweringError, lower_function_to_circuit_with_adder,
    validate_circuit_operands,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitImplementation {
    ParallelPrefix,
    RippleCarry,
}

impl CircuitImplementation {
    pub fn name(self) -> &'static str {
        match self {
            Self::ParallelPrefix => "parallel-prefix",
            Self::RippleCarry => "ripple-carry",
        }
    }
}

#[derive(Debug)]
pub struct CompilerCandidate {
    pub implementation: CircuitImplementation,
    pub circuit_function: CircuitFunction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerDecisionSummary {
    pub selected_name: String,
    pub rejected_name: String,
    pub selected_predicted_ns: f64,
    pub rejected_predicted_ns: f64,
    pub predicted_speedup: f64,
    pub predicted_time_saved_ns: f64,
    pub selected_gate_count: usize,
    pub rejected_gate_count: usize,
    pub gate_delta: isize,
    pub selected_depth: usize,
    pub rejected_depth: usize,
    pub depth_delta: isize,
    pub selected_strategy: V4ExecutionStrategy,
    pub rejected_strategy: V4ExecutionStrategy,
    pub reason: String,
}

#[derive(Debug)]
pub struct CircuitSelection {
    pub implementation: CircuitImplementation,
    pub optimized: bool,
    pub circuit_function: CircuitFunction,
    pub comparison: CandidateComparison,
    pub attribution: CostAttributionAnalysis,
}

impl CircuitSelection {
    pub fn decision_summary(&self) -> CompilerDecisionSummary {
        let selected_index = self.comparison.selected_index;

        let selected = &self.comparison.candidates[selected_index];

        let rejected = self
            .comparison
            .candidates
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != selected_index)
            .map(|(_, candidate)| candidate)
            .min_by(|left, right| {
                left.analysis
                    .selected_ns()
                    .total_cmp(&right.analysis.selected_ns())
            })
            .expect("compiler selection requires at least two candidates");

        let selected_predicted_ns = selected.analysis.selected_ns();
        let rejected_predicted_ns = rejected.analysis.selected_ns();

        let predicted_speedup = if selected_predicted_ns > 0.0 {
            rejected_predicted_ns / selected_predicted_ns
        } else {
            1.0
        };

        let predicted_time_saved_ns = (rejected_predicted_ns - selected_predicted_ns).max(0.0);

        let selected_gate_count = selected.analysis.structural.gate_count;
        let rejected_gate_count = rejected.analysis.structural.gate_count;

        let selected_depth = selected.analysis.structural.max_depth;
        let rejected_depth = rejected.analysis.structural.max_depth;

        let gate_delta = selected_gate_count as isize - rejected_gate_count as isize;

        let depth_delta = selected_depth as isize - rejected_depth as isize;

        let reason = decision_reason(
            ReasonCandidate {
                name: &selected.name,
                gates: selected_gate_count,
                depth: selected_depth,
                predicted_ns: selected_predicted_ns,
            },
            ReasonCandidate {
                name: &rejected.name,
                gates: rejected_gate_count,
                depth: rejected_depth,
                predicted_ns: rejected_predicted_ns,
            },
        );

        CompilerDecisionSummary {
            selected_name: selected.name.clone(),
            rejected_name: rejected.name.clone(),
            selected_predicted_ns,
            rejected_predicted_ns,
            predicted_speedup,
            predicted_time_saved_ns,
            selected_gate_count,
            rejected_gate_count,
            gate_delta,
            selected_depth,
            rejected_depth,
            depth_delta,
            selected_strategy: selected.analysis.recommended_strategy,
            rejected_strategy: rejected.analysis.recommended_strategy,
            reason,
        }
    }
}

#[derive(Debug)]
pub enum CompilerV4Error {
    Analysis(fhec_analyzer::AnalysisError),
    Lowering(CircuitLoweringError),
}

impl From<fhec_analyzer::AnalysisError> for CompilerV4Error {
    fn from(value: fhec_analyzer::AnalysisError) -> Self {
        Self::Analysis(value)
    }
}

impl From<CircuitLoweringError> for CompilerV4Error {
    fn from(value: CircuitLoweringError) -> Self {
        Self::Lowering(value)
    }
}

pub fn lower_to_candidates(function: &Function) -> Vec<CompilerCandidate> {
    let prefix = lower_function_to_circuit_with_adder(function, AdderStrategy::ParallelPrefix);

    let ripple = lower_function_to_circuit_with_adder(function, AdderStrategy::RippleCarry);

    vec![
        CompilerCandidate {
            implementation: CircuitImplementation::ParallelPrefix,
            circuit_function: prefix,
        },
        CompilerCandidate {
            implementation: CircuitImplementation::RippleCarry,
            circuit_function: ripple,
        },
    ]
}

pub fn select_circuit(
    function: &Function,
    model: &V4CostModel,
) -> Result<CircuitSelection, CompilerV4Error> {
    validate_circuit_operands(function)?;
    let mut candidates = lower_to_candidates(function);

    let baseline_names = candidates
        .iter()
        .map(|candidate| candidate.implementation.name().to_string())
        .collect::<Vec<_>>();

    let baseline_analyzer_candidates = candidates
        .iter()
        .zip(baseline_names.iter())
        .map(|(candidate, name)| CircuitCandidate {
            name,
            circuit: &candidate.circuit_function.circuit,
            outputs: &candidate.circuit_function.outputs,
        })
        .collect::<Vec<_>>();

    let baseline_comparison = compare_candidates(&baseline_analyzer_candidates, model)?;

    let baseline_selected = &baseline_comparison.candidates[baseline_comparison.selected_index];

    let optimizer_hints = generate_optimizer_hints(&baseline_selected.analysis);

    let optimization_plan = build_optimization_plan(&optimizer_hints);

    let mut optimized_function = function.clone();

    optimize_function_with_plan(&mut optimized_function, &optimization_plan);

    validate_circuit_operands(&optimized_function)?;
    let optimized_candidates = lower_to_candidates(&optimized_function);

    let baseline_count = candidates.len();

    candidates.extend(optimized_candidates);

    let candidate_names = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            if index < baseline_count {
                candidate.implementation.name().to_string()
            } else {
                format!("{}-optimized", candidate.implementation.name())
            }
        })
        .collect::<Vec<_>>();

    let analyzer_candidates = candidates
        .iter()
        .zip(candidate_names.iter())
        .map(|(candidate, name)| CircuitCandidate {
            name,
            circuit: &candidate.circuit_function.circuit,
            outputs: &candidate.circuit_function.outputs,
        })
        .collect::<Vec<_>>();

    let comparison = compare_candidates(&analyzer_candidates, model)?;

    let selected_index = comparison.selected_index;

    let optimized = selected_index >= baseline_count;

    let selected = candidates.remove(selected_index);

    let attribution = analyze_cost_attribution(
        &selected.circuit_function.circuit,
        &selected.circuit_function.attribution_regions,
        model,
    );

    Ok(CircuitSelection {
        implementation: selected.implementation,
        optimized,
        circuit_function: selected.circuit_function,
        comparison,
        attribution,
    })
}

pub fn render_compiler_v4_report(selection: &CircuitSelection) -> String {
    let summary = selection.decision_summary();

    let best_baseline = selection
        .comparison
        .candidates
        .iter()
        .filter(|candidate| !candidate.name.ends_with("-optimized"))
        .min_by(|left, right| {
            left.analysis
                .selected_ns()
                .total_cmp(&right.analysis.selected_ns())
        });

    let selected_candidate = &selection.comparison.candidates[selection.comparison.selected_index];

    let optimization_improvement = best_baseline.map(|baseline| {
        let baseline_ns = baseline.analysis.selected_ns();
        let selected_ns = selected_candidate.analysis.selected_ns();

        if baseline_ns > 0.0 {
            ((baseline_ns - selected_ns) / baseline_ns * 100.0).max(0.0)
        } else {
            0.0
        }
    });

    let mut output = String::new();

    writeln!(output, "FHEC COMPILER V5.6").unwrap();
    writeln!(output, "==================").unwrap();
    writeln!(output).unwrap();

    writeln!(output, "OPTIMIZATION").unwrap();
    writeln!(output, "============").unwrap();

    writeln!(
        output,
        "  Optimized candidate selected: {}",
        if selection.optimized { "YES" } else { "NO" }
    )
    .unwrap();

    writeln!(output, "  Selected candidate: {}", selected_candidate.name).unwrap();

    if let Some(baseline) = best_baseline {
        writeln!(output, "  Best baseline candidate: {}", baseline.name).unwrap();

        writeln!(
            output,
            "  Best baseline latency: {:.3} ms",
            baseline.analysis.selected_ns() / 1_000_000.0
        )
        .unwrap();

        writeln!(
            output,
            "  Selected latency: {:.3} ms",
            selected_candidate.analysis.selected_ns() / 1_000_000.0
        )
        .unwrap();
    }

    if let Some(improvement) = optimization_improvement {
        writeln!(
            output,
            "  Predicted optimization improvement: {:.2}%",
            improvement
        )
        .unwrap();
    }

    writeln!(output).unwrap();

    writeln!(output, "CANDIDATES").unwrap();
    writeln!(output, "==========").unwrap();

    for candidate in &selection.comparison.candidates {
        writeln!(output, "{}", candidate.name).unwrap();

        writeln!(
            output,
            "  Gates: {}",
            candidate.analysis.structural.gate_count
        )
        .unwrap();

        writeln!(
            output,
            "  Compute gates: {}",
            candidate.analysis.structural.gate_counts.compute_total()
        )
        .unwrap();

        writeln!(
            output,
            "  Depth: {}",
            candidate.analysis.structural.max_depth
        )
        .unwrap();

        writeln!(
            output,
            "  Runtime strategy: {:?}",
            candidate.analysis.recommended_strategy
        )
        .unwrap();

        writeln!(
            output,
            "  Predicted latency: {:.3} ms",
            candidate.analysis.selected_ns() / 1_000_000.0
        )
        .unwrap();

        writeln!(
            output,
            "  Prediction interval: {:.3}-{:.3} ms",
            candidate.analysis.prediction_interval.lower_milliseconds(),
            candidate.analysis.prediction_interval.upper_milliseconds()
        )
        .unwrap();

        writeln!(
            output,
            "  Interval margin: +/-{:.1}%",
            candidate.analysis.prediction_interval.margin_percentage()
        )
        .unwrap();

        writeln!(
            output,
            "  Calibration variability: {:.1}%",
            candidate
                .analysis
                .prediction_interval
                .variability_percentage()
        )
        .unwrap();

        writeln!(
            output,
            "  Interval confidence: {:?}",
            candidate.analysis.prediction_interval.confidence
        )
        .unwrap();
        writeln!(
            output,
            "  Interval source: {:?}",
            candidate.analysis.prediction_interval.source
        )
        .unwrap();

        writeln!(
            output,
            "  Produced ciphertexts: {}",
            candidate.analysis.memory.produced_ciphertexts
        )
        .unwrap();

        writeln!(
            output,
            "  Peak live ciphertexts: {}",
            candidate.analysis.memory.peak_live_ciphertexts
        )
        .unwrap();

        if candidate.analysis.memory.bytes_known() {
            writeln!(
                output,
                "  Peak ciphertext memory: {:.3} MiB",
                candidate.analysis.memory.peak_live_bytes as f64 / (1024.0 * 1024.0)
            )
            .unwrap();

            writeln!(
                output,
                "  Total ciphertext allocation: {:.3} MiB",
                candidate.analysis.memory.total_ciphertext_bytes_created as f64 / (1024.0 * 1024.0)
            )
            .unwrap();
        }

        if candidate.analysis.crypto.has_crypto_work() {
            writeln!(
                output,
                "  Bootstraps: {}",
                candidate.analysis.crypto.operations.bootstraps
            )
            .unwrap();

            writeln!(
                output,
                "  Key switches: {}",
                candidate.analysis.crypto.operations.key_switches
            )
            .unwrap();

            writeln!(
                output,
                "  Refreshes: {}",
                candidate.analysis.crypto.operations.refreshes
            )
            .unwrap();

            writeln!(
                output,
                "  Crypto profile: {:?}",
                candidate.analysis.crypto.source
            )
            .unwrap();

            if candidate.analysis.crypto.total_crypto_ns > 0.0 {
                writeln!(
                    output,
                    "  Estimated crypto cost: {:.3} ms",
                    candidate.analysis.crypto.total_crypto_milliseconds()
                )
                .unwrap();

                writeln!(
                    output,
                    "  Dominant crypto primitive: {:?}",
                    candidate.analysis.crypto.dominant
                )
                .unwrap();
            }
        }

        writeln!(
            output,
            "  Noise behavior: {:?}",
            candidate.analysis.noise.behavior
        )
        .unwrap();

        writeln!(
            output,
            "  Refresh operations: {}",
            candidate.analysis.noise.refresh_operations
        )
        .unwrap();

        writeln!(
            output,
            "  Refresh coverage: {:.1}%",
            candidate.analysis.noise.refresh_percentage()
        )
        .unwrap();

        writeln!(
            output,
            "  Max unrefreshed depth: {}",
            candidate.analysis.noise.max_unrefreshed_depth
        )
        .unwrap();

        writeln!(output, "  Noise risk: {:?}", candidate.analysis.noise.risk).unwrap();

        let optimizer_hints = generate_optimizer_hints(&candidate.analysis);

        if !optimizer_hints.is_empty() {
            writeln!(output, "  Optimizer hints:").unwrap();

            for hint in &optimizer_hints.hints {
                writeln!(
                    output,
                    "    {:?} [{:?}] target={:?} current={:.2}",
                    hint.kind, hint.priority, hint.target, hint.current_value
                )
                .unwrap();
            }
        }

        writeln!(output).unwrap();
    }

    writeln!(output, "DECISION").unwrap();
    writeln!(output, "========").unwrap();

    writeln!(output, "Selected: {}", summary.selected_name).unwrap();

    writeln!(output, "Rejected: {}", summary.rejected_name).unwrap();

    writeln!(
        output,
        "Selected predicted latency: {:.3} ms",
        summary.selected_predicted_ns / 1_000_000.0
    )
    .unwrap();

    writeln!(
        output,
        "Rejected predicted latency: {:.3} ms",
        summary.rejected_predicted_ns / 1_000_000.0
    )
    .unwrap();

    writeln!(
        output,
        "Predicted advantage: {:.2}x",
        summary.predicted_speedup
    )
    .unwrap();

    writeln!(
        output,
        "Predicted time saved: {:.3} ms",
        summary.predicted_time_saved_ns / 1_000_000.0
    )
    .unwrap();

    writeln!(output).unwrap();

    writeln!(output, "Selected gates: {}", summary.selected_gate_count).unwrap();

    writeln!(output, "Rejected gates: {}", summary.rejected_gate_count).unwrap();

    writeln!(output, "Gate difference: {:+}", summary.gate_delta).unwrap();

    writeln!(output, "Selected depth: {}", summary.selected_depth).unwrap();

    writeln!(output, "Rejected depth: {}", summary.rejected_depth).unwrap();

    writeln!(output, "Depth difference: {:+}", summary.depth_delta).unwrap();

    writeln!(
        output,
        "Selected runtime strategy: {:?}",
        summary.selected_strategy
    )
    .unwrap();

    writeln!(
        output,
        "Rejected runtime strategy: {:?}",
        summary.rejected_strategy
    )
    .unwrap();

    writeln!(output).unwrap();

    writeln!(output, "Reason:").unwrap();
    writeln!(output, "{}", summary.reason).unwrap();

    writeln!(output).unwrap();

    writeln!(output, "IR COST ATTRIBUTION").unwrap();
    writeln!(output, "===================").unwrap();

    for region in &selection.attribution.regions {
        writeln!(
            output,
            "IR #{} {}",
            region.operation_index, region.operation_kind
        )
        .unwrap();

        writeln!(output, "  Generated gates: {}", region.gate_count).unwrap();

        writeln!(output, "  Compute gates: {}", region.compute_gate_count).unwrap();

        writeln!(
            output,
            "  Ciphertexts produced: {}",
            region.ciphertexts_produced
        )
        .unwrap();

        writeln!(
            output,
            "  Bootstraps: {}",
            region.crypto_operations.bootstraps
        )
        .unwrap();

        writeln!(
            output,
            "  Key switches: {}",
            region.crypto_operations.key_switches
        )
        .unwrap();

        writeln!(
            output,
            "  Estimated gate work: {:.3} ms",
            region.estimated_gate_work_milliseconds()
        )
        .unwrap();

        writeln!(output, "  Work share: {:.1}%", region.work_share * 100.0).unwrap();
    }

    if let Some(dominant) = selection.attribution.dominant_region() {
        writeln!(output).unwrap();

        writeln!(
            output,
            "Dominant IR operation: #{} {} ({:.1}% of attributed gate work)",
            dominant.operation_index,
            dominant.operation_kind,
            dominant.work_share * 100.0
        )
        .unwrap();
    }

    writeln!(
        output,
        "Unattributed gates: {}",
        selection.attribution.unattributed_gate_count
    )
    .unwrap();

    output
}

struct ReasonCandidate<'a> {
    name: &'a str,
    gates: usize,
    depth: usize,
    predicted_ns: f64,
}

fn decision_reason(selected: ReasonCandidate<'_>, rejected: ReasonCandidate<'_>) -> String {
    let selected_name = selected.name;
    let rejected_name = rejected.name;
    let selected_gates = selected.gates;
    let rejected_gates = rejected.gates;
    let selected_depth = selected.depth;
    let rejected_depth = rejected.depth;
    let selected_ns = selected.predicted_ns;
    let rejected_ns = rejected.predicted_ns;
    if selected_ns > rejected_ns {
        return format!(
            "{} was selected despite having higher predicted execution latency than {}. Review the candidate-selection policy.",
            selected_name, rejected_name
        );
    }

    if selected_ns == rejected_ns {
        return format!(
            "{} and {} have equal predicted execution latency; {} was selected by the candidate-selection tie-breaker.",
            selected_name, rejected_name, selected_name
        );
    }

    if selected_gates > rejected_gates && selected_depth < rejected_depth {
        return format!(
            "{} uses more gates than {}, but reduces circuit depth enough that the calibrated backend model predicts lower execution latency.",
            selected_name, rejected_name,
        );
    }

    if selected_gates < rejected_gates && selected_depth <= rejected_depth {
        return format!(
            "{} has both fewer gates and no greater circuit depth than {}, producing the lower predicted backend execution latency.",
            selected_name, rejected_name,
        );
    }

    if selected_depth < rejected_depth {
        return format!(
            "{} has a shallower dependency structure than {}, and the calibrated backend model predicts that the additional parallelism outweighs the remaining structural costs.",
            selected_name, rejected_name,
        );
    }

    if selected_gates < rejected_gates {
        return format!(
            "{} reduces gate work relative to {}, and the calibrated backend model predicts that this produces the lower execution latency.",
            selected_name, rejected_name,
        );
    }

    format!(
        "{} has the lowest predicted backend execution latency after evaluating gate costs, circuit depth, parallelism, and runtime strategy.",
        selected_name,
    )
}

#[cfg(test)]
mod malformed_ir_regression {
    use super::*;
    use fhec_ir::{Operation, Type, ValueId};

    #[test]
    fn selection_rejects_missing_copy_operand() {
        let mut function = Function::new("invalid_copy", vec![], Type::Bool);
        function.push_operation(Operation::Copy {
            result: ValueId(1),
            operand: ValueId(999),
        });
        assert!(matches!(
            select_circuit(&function, &V4CostModel::default()),
            Err(CompilerV4Error::Lowering(
                CircuitLoweringError::MissingOperand(ValueId(999))
            ))
        ));
    }

    #[test]
    fn selection_rejects_missing_arithmetic_operand() {
        let mut function = Function::new("invalid_add", vec![], Type::Bool);
        function.push_operation(Operation::Add {
            result: ValueId(1),
            lhs: ValueId(998),
            rhs: ValueId(999),
        });
        assert!(matches!(
            select_circuit(&function, &V4CostModel::default()),
            Err(CompilerV4Error::Lowering(
                CircuitLoweringError::MissingOperand(ValueId(998))
            ))
        ));
    }
}

#[cfg(test)]
mod tests {
    use fhec_analyzer::v4::V4CostModel;
    use fhec_frontend::{lexer::Lexer, parser::Parser};

    use crate::{CircuitImplementation, Compiler, render_compiler_v4_report};

    fn compile_add() -> (Compiler, fhec_ir::Module) {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().unwrap();

        let program = Parser::new(tokens).parse().unwrap();

        let compiler = Compiler::new();
        let module = compiler.lower(&program).expect("lowering failed");

        (compiler, module)
    }

    #[test]
    fn compiler_generates_two_addition_candidates() {
        let (compiler, module) = compile_add();

        let function = &module.functions[0];

        let candidates = compiler.lower_to_circuit_candidates(function);

        assert_eq!(candidates.len(), 2);

        assert_eq!(
            candidates[0].implementation,
            CircuitImplementation::ParallelPrefix
        );

        assert_eq!(
            candidates[1].implementation,
            CircuitImplementation::RippleCarry
        );

        for candidate in &candidates {
            candidate.circuit_function.verify(function).unwrap();
        }
    }

    #[test]
    fn v4_compiler_selects_a_candidate() {
        let (compiler, module) = compile_add();

        let function = &module.functions[0];

        let selection = compiler
            .lower_to_circuit_v4(function, &V4CostModel::default())
            .unwrap();

        assert_eq!(selection.comparison.candidates.len(), 4);

        selection.circuit_function.verify(function).unwrap();
    }

    #[test]
    fn decision_summary_compares_candidates() {
        let (compiler, module) = compile_add();

        let function = &module.functions[0];

        let selection = compiler
            .lower_to_circuit_v4(function, &V4CostModel::default())
            .unwrap();

        let summary = selection.decision_summary();

        assert!(summary.selected_predicted_ns <= summary.rejected_predicted_ns);

        assert!(summary.predicted_speedup >= 1.0);

        assert!(!summary.reason.is_empty());
    }

    #[test]
    fn compiler_report_contains_optimization_summary() {
        let (compiler, module) = compile_add();

        let function = &module.functions[0];

        let selection = compiler
            .lower_to_circuit_v4(function, &V4CostModel::default())
            .unwrap();

        let report = render_compiler_v4_report(&selection);

        assert!(report.contains("OPTIMIZATION"));
        assert!(report.contains("Optimized candidate selected:"));
        assert!(report.contains("Selected candidate:"));
        assert!(report.contains("Best baseline candidate:"));
        assert!(report.contains("Predicted optimization improvement:"));
    }
    #[test]
    fn compiler_report_contains_reason() {
        let (compiler, module) = compile_add();

        let function = &module.functions[0];

        let selection = compiler
            .lower_to_circuit_v4(function, &V4CostModel::default())
            .unwrap();

        let report = render_compiler_v4_report(&selection);

        assert!(report.contains("FHEC COMPILER V5.6"));

        assert!(report.contains("parallel-prefix"));
        assert!(report.contains("ripple-carry"));

        assert!(report.contains("Produced ciphertexts:"));

        assert!(report.contains("Peak live ciphertexts:"));

        assert!(report.contains("Selected:"));
        assert!(report.contains("Reason:"));
    }
}
