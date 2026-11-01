use crate::{
    noise::NoiseRisk,
    v4::{BottleneckKind, V4ExecutionStrategy, V4StrategyAnalysis},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HintPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizerHintKind {
    ReduceDepth,
    IncreaseParallelism,
    ReduceCryptoOperations,
    ReducePeakMemory,
    ReduceSynchronization,
    ReplaceExpensiveGates,
    ReduceRefreshPressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMetric {
    CircuitDepth,
    Parallelism,
    BootstrapCount,
    PeakLiveCiphertexts,
    SynchronizationCost,
    GateCost,
    NoisePressure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizerHint {
    pub kind: OptimizerHintKind,
    pub priority: HintPriority,
    pub target: TargetMetric,
    pub current_value: f64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OptimizerHintSet {
    pub hints: Vec<OptimizerHint>,
}

impl OptimizerHintSet {
    pub fn is_empty(&self) -> bool {
        self.hints.is_empty()
    }

    pub fn len(&self) -> usize {
        self.hints.len()
    }

    pub fn contains(&self, kind: OptimizerHintKind) -> bool {
        self.hints.iter().any(|hint| hint.kind == kind)
    }

    pub fn highest_priority(&self) -> Option<HintPriority> {
        self.hints.iter().map(|hint| hint.priority).max()
    }

    fn push(&mut self, hint: OptimizerHint) {
        if !self.contains(hint.kind) {
            self.hints.push(hint);
        }
    }

    fn sort_by_priority(&mut self) {
        self.hints
            .sort_by_key(|left| std::cmp::Reverse(left.priority));
    }
}

pub fn generate_optimizer_hints(analysis: &V4StrategyAnalysis) -> OptimizerHintSet {
    let mut result = OptimizerHintSet::default();

    match analysis.bottleneck.kind {
        BottleneckKind::DepthBound => {
            result.push(OptimizerHint {
                kind: OptimizerHintKind::ReduceDepth,
                priority: HintPriority::High,
                target: TargetMetric::CircuitDepth,
                current_value: analysis.structural.max_depth as f64,
                reason: analysis.bottleneck.message.clone(),
            });
        }

        BottleneckKind::ParallelismBound => {
            result.push(OptimizerHint {
                kind: OptimizerHintKind::IncreaseParallelism,
                priority: HintPriority::High,
                target: TargetMetric::Parallelism,
                current_value: analysis.structural.max_compute_parallelism as f64,
                reason: analysis.bottleneck.message.clone(),
            });
        }

        BottleneckKind::SynchronizationBound => {
            result.push(OptimizerHint {
                kind: OptimizerHintKind::ReduceSynchronization,
                priority: HintPriority::High,
                target: TargetMetric::SynchronizationCost,
                current_value: analysis.parallel_ns,
                reason: analysis.bottleneck.message.clone(),
            });
        }

        BottleneckKind::GateCostBound => {
            result.push(OptimizerHint {
                kind: OptimizerHintKind::ReplaceExpensiveGates,
                priority: HintPriority::High,
                target: TargetMetric::GateCost,
                current_value: analysis.sequential_ns,
                reason: analysis.bottleneck.message.clone(),
            });
        }

        BottleneckKind::Balanced => {}
    }

    let compute = analysis.structural.gate_counts.compute_total();

    if analysis.crypto.operations.bootstraps > 0 {
        let density = if compute == 0 {
            0.0
        } else {
            analysis.crypto.operations.bootstraps as f64 / compute as f64
        };

        result.push(OptimizerHint {
            kind: OptimizerHintKind::ReduceCryptoOperations,
            priority: if density >= 1.0 {
                HintPriority::High
            } else {
                HintPriority::Medium
            },
            target: TargetMetric::BootstrapCount,
            current_value: analysis.crypto.operations.bootstraps as f64,
            reason: format!(
                "The candidate performs {} backend-declared bootstrap operations.",
                analysis.crypto.operations.bootstraps
            ),
        });
    }

    if analysis.memory.produced_ciphertexts > 0 {
        let live_ratio = analysis.memory.peak_live_ciphertexts as f64
            / analysis.memory.produced_ciphertexts as f64;

        if live_ratio >= 0.35 {
            result.push(OptimizerHint {
                kind: OptimizerHintKind::ReducePeakMemory,
                priority: if live_ratio >= 0.65 {
                    HintPriority::High
                } else {
                    HintPriority::Medium
                },
                target: TargetMetric::PeakLiveCiphertexts,
                current_value: analysis.memory.peak_live_ciphertexts as f64,
                reason: format!(
                    "{:.1}% of produced ciphertexts are simultaneously live at peak.",
                    live_ratio * 100.0
                ),
            });
        }
    }

    match analysis.noise.risk {
        NoiseRisk::High => {
            result.push(OptimizerHint {
                kind: OptimizerHintKind::ReduceRefreshPressure,
                priority: HintPriority::Critical,
                target: TargetMetric::NoisePressure,
                current_value: analysis.noise.max_unrefreshed_depth as f64,
                reason: "Noise analysis reports high accumulation risk.".to_string(),
            });
        }

        NoiseRisk::Medium => {
            result.push(OptimizerHint {
                kind: OptimizerHintKind::ReduceRefreshPressure,
                priority: HintPriority::High,
                target: TargetMetric::NoisePressure,
                current_value: analysis.noise.max_unrefreshed_depth as f64,
                reason: "Noise analysis reports medium accumulation risk.".to_string(),
            });
        }

        NoiseRisk::Low | NoiseRisk::Unknown => {}
    }

    if analysis.recommended_strategy == V4ExecutionStrategy::Sequential && compute >= 4 {
        result.push(OptimizerHint {
            kind: OptimizerHintKind::IncreaseParallelism,
            priority: HintPriority::Medium,
            target: TargetMetric::Parallelism,
            current_value:
                analysis.structural.max_compute_parallelism as f64,
            reason:
                "The candidate contains multiple compute operations but is still predicted to execute sequentially."
                    .to_string(),
        });
    }

    result.sort_by_priority();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    use fhec_circuit::Circuit;

    use crate::v4::{V4CostModel, analyze_v4};

    #[test]
    fn hint_set_orders_priorities() {
        let mut set = OptimizerHintSet::default();

        set.push(OptimizerHint {
            kind: OptimizerHintKind::ReducePeakMemory,
            priority: HintPriority::Medium,
            target: TargetMetric::PeakLiveCiphertexts,
            current_value: 10.0,
            reason: "memory".to_string(),
        });

        set.push(OptimizerHint {
            kind: OptimizerHintKind::ReduceDepth,
            priority: HintPriority::High,
            target: TargetMetric::CircuitDepth,
            current_value: 5.0,
            reason: "depth".to_string(),
        });

        set.sort_by_priority();

        assert_eq!(set.hints[0].priority, HintPriority::High);
    }

    #[test]
    fn deep_circuit_generates_machine_readable_hint() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();
        let b = circuit.add_input();

        let first = circuit.add_and(a, b);
        let second = circuit.add_and(first, b);
        let third = circuit.add_and(second, b);
        let output = circuit.add_and(third, b);

        let analysis = analyze_v4(&circuit, &[output], &V4CostModel::default()).unwrap();

        let hints = generate_optimizer_hints(&analysis);

        assert!(!hints.is_empty());

        assert!(hints.highest_priority().is_some());
    }

    #[test]
    fn duplicate_hint_kinds_are_not_added() {
        let mut set = OptimizerHintSet::default();

        let hint = OptimizerHint {
            kind: OptimizerHintKind::ReduceDepth,
            priority: HintPriority::High,
            target: TargetMetric::CircuitDepth,
            current_value: 10.0,
            reason: "depth".to_string(),
        };

        set.push(hint.clone());
        set.push(hint);

        assert_eq!(set.len(), 1);
    }
}
