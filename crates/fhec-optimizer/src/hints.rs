use fhec_analyzer::hints::{HintPriority, OptimizerHintKind, OptimizerHintSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationAction {
    ConstantFold,
    BooleanSimplify,
    DepthReduction,
    ParallelismExposure,
    CryptoOperationReduction,
    MemoryLifetimeReduction,
    SynchronizationReduction,
    ExpensiveGateLowering,
    RefreshPressureReduction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedOptimization {
    pub action: OptimizationAction,
    pub priority: HintPriority,
    pub currently_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationPlan {
    pub actions: Vec<PlannedOptimization>,
}

impl OptimizationPlan {
    pub fn has_action(&self, action: OptimizationAction) -> bool {
        self.actions.iter().any(|planned| planned.action == action)
    }

    pub fn unsupported_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|planned| !planned.currently_supported)
            .count()
    }
}

pub fn build_optimization_plan(hints: &OptimizerHintSet) -> OptimizationPlan {
    let mut actions = vec![
        PlannedOptimization {
            action: OptimizationAction::ConstantFold,
            priority: HintPriority::Low,
            currently_supported: true,
        },
        PlannedOptimization {
            action: OptimizationAction::BooleanSimplify,
            priority: HintPriority::Low,
            currently_supported: true,
        },
    ];

    for hint in &hints.hints {
        let action = match hint.kind {
            OptimizerHintKind::ReduceDepth => OptimizationAction::DepthReduction,

            OptimizerHintKind::IncreaseParallelism => OptimizationAction::ParallelismExposure,

            OptimizerHintKind::ReduceCryptoOperations => {
                OptimizationAction::CryptoOperationReduction
            }

            OptimizerHintKind::ReducePeakMemory => OptimizationAction::MemoryLifetimeReduction,

            OptimizerHintKind::ReduceSynchronization => {
                OptimizationAction::SynchronizationReduction
            }

            OptimizerHintKind::ReplaceExpensiveGates => OptimizationAction::ExpensiveGateLowering,

            OptimizerHintKind::ReduceRefreshPressure => {
                OptimizationAction::RefreshPressureReduction
            }
        };

        if let Some(planned) = actions.iter_mut().find(|planned| planned.action == action) {
            planned.priority = planned.priority.max(hint.priority);
        } else {
            actions.push(PlannedOptimization {
                action,
                priority: hint.priority,
                currently_supported: matches!(
                    action,
                    OptimizationAction::ExpensiveGateLowering
                        | OptimizationAction::CryptoOperationReduction
                        | OptimizationAction::DepthReduction
                        | OptimizationAction::ParallelismExposure
                ),
            });
        }
    }

    actions.sort_by_key(|left| std::cmp::Reverse(left.priority));

    OptimizationPlan { actions }
}

#[cfg(test)]
mod tests {
    use super::*;

    use fhec_analyzer::hints::{OptimizerHint, TargetMetric};

    #[test]
    fn plan_contains_existing_optimizer_passes() {
        let plan = build_optimization_plan(&OptimizerHintSet::default());

        assert!(plan.has_action(OptimizationAction::ConstantFold));

        assert!(plan.has_action(OptimizationAction::BooleanSimplify));
    }

    #[test]
    fn depth_hint_creates_depth_action() {
        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReduceDepth,
                priority: HintPriority::High,
                target: TargetMetric::CircuitDepth,
                current_value: 12.0,
                reason: "depth".to_string(),
            }],
        };

        let plan = build_optimization_plan(&hints);

        assert!(plan.has_action(OptimizationAction::DepthReduction));

        let planned = plan
            .actions
            .iter()
            .find(|planned| planned.action == OptimizationAction::DepthReduction)
            .expect("missing depth reduction action");

        assert!(planned.currently_supported);
    }

    #[test]
    fn crypto_hint_creates_crypto_action() {
        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReduceCryptoOperations,
                priority: HintPriority::High,
                target: TargetMetric::BootstrapCount,
                current_value: 75.0,
                reason: "crypto".to_string(),
            }],
        };

        let plan = build_optimization_plan(&hints);

        assert!(plan.has_action(OptimizationAction::CryptoOperationReduction));

        let planned = plan
            .actions
            .iter()
            .find(|planned| planned.action == OptimizationAction::CryptoOperationReduction)
            .expect("missing crypto operation reduction action");

        assert!(planned.currently_supported);
    }

    #[test]
    fn expensive_gate_hint_creates_supported_action() {
        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReplaceExpensiveGates,
                priority: HintPriority::High,
                target: TargetMetric::GateCost,
                current_value: 100.0,
                reason: "expensive gates".to_string(),
            }],
        };

        let plan = build_optimization_plan(&hints);

        let planned = plan
            .actions
            .iter()
            .find(|planned| planned.action == OptimizationAction::ExpensiveGateLowering)
            .expect("missing expensive gate lowering action");

        assert!(planned.currently_supported);
    }
    #[test]
    fn parallelism_hint_creates_supported_action() {
        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::IncreaseParallelism,
                priority: HintPriority::High,
                target: TargetMetric::Parallelism,
                current_value: 1.0,
                reason: "parallelism".to_string(),
            }],
        };

        let plan = build_optimization_plan(&hints);

        let planned = plan
            .actions
            .iter()
            .find(|planned| planned.action == OptimizationAction::ParallelismExposure)
            .expect("missing parallelism exposure action");

        assert!(planned.currently_supported);
    }
    #[test]
    fn memory_hint_creates_memory_action() {
        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReducePeakMemory,
                priority: HintPriority::Medium,
                target: TargetMetric::PeakLiveCiphertexts,
                current_value: 45.0,
                reason: "memory".to_string(),
            }],
        };

        let plan = build_optimization_plan(&hints);

        assert!(plan.has_action(OptimizationAction::MemoryLifetimeReduction));
    }

    #[test]
    fn duplicate_hints_keep_highest_priority_regardless_of_order() {
        for priorities in [
            [HintPriority::Low, HintPriority::Critical],
            [HintPriority::Critical, HintPriority::Low],
        ] {
            let hints = OptimizerHintSet {
                hints: priorities
                    .into_iter()
                    .map(|priority| OptimizerHint {
                        kind: OptimizerHintKind::ReduceDepth,
                        priority,
                        target: TargetMetric::CircuitDepth,
                        current_value: 8.0,
                        reason: "depth".to_string(),
                    })
                    .collect(),
            };

            let plan = build_optimization_plan(&hints);
            let depth = plan
                .actions
                .iter()
                .filter(|planned| planned.action == OptimizationAction::DepthReduction)
                .collect::<Vec<_>>();

            assert_eq!(depth.len(), 1);
            assert_eq!(depth[0].priority, HintPriority::Critical);
            assert!(depth[0].currently_supported);
            assert_eq!(plan.actions[0].action, OptimizationAction::DepthReduction);
        }
    }
}
