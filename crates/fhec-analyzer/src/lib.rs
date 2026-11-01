pub mod attribution;
pub mod crypto;
pub mod hardware_profile;
pub mod hints;
pub mod noise;
pub mod v4;

use fhec_circuit::{Circuit, Gate, VerificationError, WireId};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GateCounts {
    pub constants: usize,
    pub inputs: usize,
    pub not: usize,
    pub and: usize,
    pub or: usize,
    pub xor: usize,
    pub mux: usize,
}

impl GateCounts {
    pub fn total(&self) -> usize {
        self.constants + self.inputs + self.not + self.and + self.or + self.xor + self.mux
    }

    pub fn compute_total(&self) -> usize {
        self.not + self.and + self.or + self.xor + self.mux
    }

    pub fn weighted_compute_cost(&self, model: &CostModel) -> f64 {
        self.not as f64 * model.not_weight
            + self.and as f64 * model.and_weight
            + self.or as f64 * model.or_weight
            + self.xor as f64 * model.xor_weight
            + self.mux as f64 * model.mux_weight
    }

    pub fn maximum_gate_cost(&self, model: &CostModel) -> f64 {
        let mut maximum: f64 = 0.0;

        if self.not > 0 {
            maximum = maximum.max(model.not_weight);
        }

        if self.and > 0 {
            maximum = maximum.max(model.and_weight);
        }

        if self.or > 0 {
            maximum = maximum.max(model.or_weight);
        }

        if self.xor > 0 {
            maximum = maximum.max(model.xor_weight);
        }

        if self.mux > 0 {
            maximum = maximum.max(model.mux_weight);
        }

        maximum
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitAnalysis {
    pub gate_count: usize,
    pub wire_count: usize,
    pub gate_counts: GateCounts,
    pub max_depth: usize,
    pub critical_output_depth: usize,
    pub output_depths: Vec<usize>,
    pub max_fanout: usize,
    pub max_parallelism: usize,
    pub max_compute_parallelism: usize,
    pub gates_per_depth: Vec<usize>,
    pub compute_gate_counts_per_depth: Vec<GateCounts>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostModelSource {
    Heuristic,
    BackendCalibrated,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostModel {
    pub not_weight: f64,
    pub and_weight: f64,
    pub or_weight: f64,
    pub xor_weight: f64,
    pub mux_weight: f64,
    pub parallel_efficiency: f64,
    pub layer_sync_cost: f64,
    pub source: CostModelSource,
}

impl CostModel {
    pub fn calibrated_nanoseconds(
        not_ns: f64,
        and_ns: f64,
        or_ns: f64,
        xor_ns: f64,
        mux_ns: f64,
        parallel_efficiency: f64,
        layer_sync_ns: f64,
    ) -> Self {
        Self {
            not_weight: not_ns,
            and_weight: and_ns,
            or_weight: or_ns,
            xor_weight: xor_ns,
            mux_weight: mux_ns,
            parallel_efficiency: parallel_efficiency.clamp(0.01, 1.0),
            layer_sync_cost: layer_sync_ns.max(0.0),
            source: CostModelSource::BackendCalibrated,
        }
    }

    pub fn is_calibrated(&self) -> bool {
        self.source == CostModelSource::BackendCalibrated
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            not_weight: 1.0,
            and_weight: 1.0,
            or_weight: 1.0,
            xor_weight: 1.0,
            mux_weight: 1.5,
            parallel_efficiency: 0.70,
            layer_sync_cost: 0.25,
            source: CostModelSource::Heuristic,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerCostAnalysis {
    pub depth: usize,
    pub gate_counts: GateCounts,
    pub compute_gate_count: usize,
    pub sequential_cost: f64,
    pub parallel_cost: f64,
    pub effective_parallelism: f64,
}

impl LayerCostAnalysis {
    pub fn sequential_milliseconds(&self, source: CostModelSource) -> Option<f64> {
        if source != CostModelSource::BackendCalibrated {
            return None;
        }

        Some(self.sequential_cost / 1_000_000.0)
    }

    pub fn parallel_milliseconds(&self, source: CostModelSource) -> Option<f64> {
        if source != CostModelSource::BackendCalibrated {
            return None;
        }

        Some(self.parallel_cost / 1_000_000.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrategyAnalysis {
    pub compute_gate_count: usize,
    pub execution_layers: usize,
    pub average_compute_parallelism: f64,
    pub max_compute_parallelism: usize,
    pub weighted_work: f64,
    pub sequential_score: f64,
    pub parallel_score: f64,
    pub estimated_speedup: f64,
    pub recommended_strategy: ExecutionStrategy,
    pub parallel_friendly: bool,
    pub cost_model_source: CostModelSource,
    pub layer_costs: Vec<LayerCostAnalysis>,
}

impl StrategyAnalysis {
    pub fn estimated_sequential_milliseconds(&self) -> Option<f64> {
        if self.cost_model_source != CostModelSource::BackendCalibrated {
            return None;
        }

        Some(self.sequential_score / 1_000_000.0)
    }

    pub fn estimated_parallel_milliseconds(&self) -> Option<f64> {
        if self.cost_model_source != CostModelSource::BackendCalibrated {
            return None;
        }

        Some(self.parallel_score / 1_000_000.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError {
    InvalidCircuit(VerificationError),
    InvalidOutputWire(WireId),
}

pub fn analyze_circuit(
    circuit: &Circuit,
    outputs: &[WireId],
) -> Result<CircuitAnalysis, AnalysisError> {
    circuit.verify().map_err(AnalysisError::InvalidCircuit)?;

    for output in outputs {
        if !circuit.contains_wire(*output) {
            return Err(AnalysisError::InvalidOutputWire(*output));
        }
    }

    let mut gate_counts = GateCounts::default();

    let mut depths = vec![0usize; circuit.wire_count()];

    let mut fanout = vec![0usize; circuit.wire_count()];

    let mut gates_per_depth = Vec::new();

    let mut compute_gate_counts_per_depth: Vec<GateCounts> = Vec::new();

    for gate in circuit.gates() {
        let (wire, depth, compute_kind) = match gate {
            Gate::Constant { wire, .. } => {
                gate_counts.constants += 1;

                (*wire, 0, None)
            }

            Gate::Input { wire, .. } => {
                gate_counts.inputs += 1;

                (*wire, 0, None)
            }

            Gate::Not { wire, operand, .. } => {
                gate_counts.not += 1;

                fanout[operand.0 as usize] += 1;

                (*wire, depths[operand.0 as usize] + 1, Some(0usize))
            }

            Gate::And { wire, lhs, rhs, .. } => {
                gate_counts.and += 1;

                fanout[lhs.0 as usize] += 1;

                fanout[rhs.0 as usize] += 1;

                (
                    *wire,
                    depths[lhs.0 as usize].max(depths[rhs.0 as usize]) + 1,
                    Some(1usize),
                )
            }

            Gate::Or { wire, lhs, rhs, .. } => {
                gate_counts.or += 1;

                fanout[lhs.0 as usize] += 1;

                fanout[rhs.0 as usize] += 1;

                (
                    *wire,
                    depths[lhs.0 as usize].max(depths[rhs.0 as usize]) + 1,
                    Some(2usize),
                )
            }

            Gate::Xor { wire, lhs, rhs, .. } => {
                gate_counts.xor += 1;

                fanout[lhs.0 as usize] += 1;

                fanout[rhs.0 as usize] += 1;

                (
                    *wire,
                    depths[lhs.0 as usize].max(depths[rhs.0 as usize]) + 1,
                    Some(3usize),
                )
            }

            Gate::Mux {
                wire,
                condition,
                if_true,
                if_false,
                ..
            } => {
                gate_counts.mux += 1;

                fanout[condition.0 as usize] += 1;

                fanout[if_true.0 as usize] += 1;

                fanout[if_false.0 as usize] += 1;

                let parent_depth = depths[condition.0 as usize]
                    .max(depths[if_true.0 as usize])
                    .max(depths[if_false.0 as usize]);

                (*wire, parent_depth + 1, Some(4usize))
            }
        };

        depths[wire.0 as usize] = depth;

        if gates_per_depth.len() <= depth {
            gates_per_depth.resize(depth + 1, 0);
        }

        gates_per_depth[depth] += 1;

        if let Some(kind) = compute_kind {
            if compute_gate_counts_per_depth.len() <= depth {
                compute_gate_counts_per_depth.resize_with(depth + 1, GateCounts::default);
            }

            let counts = &mut compute_gate_counts_per_depth[depth];

            match kind {
                0 => {
                    counts.not += 1;
                }

                1 => {
                    counts.and += 1;
                }

                2 => {
                    counts.or += 1;
                }

                3 => {
                    counts.xor += 1;
                }

                4 => {
                    counts.mux += 1;
                }

                _ => unreachable!(),
            }
        }
    }

    let output_depths = outputs
        .iter()
        .map(|wire| depths[wire.0 as usize])
        .collect::<Vec<_>>();

    let max_depth = depths.iter().copied().max().unwrap_or(0);

    let critical_output_depth = output_depths.iter().copied().max().unwrap_or(0);

    let max_fanout = fanout.iter().copied().max().unwrap_or(0);

    let max_parallelism = gates_per_depth.iter().copied().max().unwrap_or(0);

    let max_compute_parallelism = compute_gate_counts_per_depth
        .iter()
        .map(GateCounts::compute_total)
        .max()
        .unwrap_or(0);

    Ok(CircuitAnalysis {
        gate_count: circuit.gate_count(),
        wire_count: circuit.wire_count(),
        gate_counts,
        max_depth,
        critical_output_depth,
        output_depths,
        max_fanout,
        max_parallelism,
        max_compute_parallelism,
        gates_per_depth,
        compute_gate_counts_per_depth,
    })
}

pub fn analyze_execution_strategy(
    analysis: &CircuitAnalysis,
    model: &CostModel,
) -> StrategyAnalysis {
    let compute_gate_count = analysis.gate_counts.compute_total();

    let efficiency = model.parallel_efficiency.clamp(0.01, 1.0);

    let weighted_work = analysis.gate_counts.weighted_compute_cost(model);

    let mut layer_costs = Vec::new();

    for (depth, counts) in analysis.compute_gate_counts_per_depth.iter().enumerate() {
        let layer_gate_count = counts.compute_total();

        if layer_gate_count == 0 {
            continue;
        }

        let sequential_cost = counts.weighted_compute_cost(model);

        let effective_parallelism =
            (layer_gate_count as f64 * efficiency).clamp(1.0, layer_gate_count as f64);

        let average_parallel_cost = sequential_cost / effective_parallelism;

        let critical_gate_cost = counts.maximum_gate_cost(model);

        let parallel_cost = average_parallel_cost.max(critical_gate_cost) + model.layer_sync_cost;

        layer_costs.push(LayerCostAnalysis {
            depth,
            gate_counts: counts.clone(),
            compute_gate_count: layer_gate_count,
            sequential_cost,
            parallel_cost,
            effective_parallelism,
        });
    }

    let execution_layers = layer_costs.len();

    let average_compute_parallelism = if execution_layers == 0 {
        0.0
    } else {
        compute_gate_count as f64 / execution_layers as f64
    };

    let sequential_score = layer_costs
        .iter()
        .map(|layer| layer.sequential_cost)
        .sum::<f64>();

    let parallel_score = layer_costs
        .iter()
        .map(|layer| layer.parallel_cost)
        .sum::<f64>();

    let estimated_speedup = if parallel_score > 0.0 {
        sequential_score / parallel_score
    } else {
        1.0
    };

    let parallel_friendly = compute_gate_count >= 4
        && analysis.max_compute_parallelism >= 2
        && parallel_score < sequential_score;

    let recommended_strategy = if parallel_friendly {
        ExecutionStrategy::Parallel
    } else {
        ExecutionStrategy::Sequential
    };

    StrategyAnalysis {
        compute_gate_count,
        execution_layers,
        average_compute_parallelism,
        max_compute_parallelism: analysis.max_compute_parallelism,
        weighted_work,
        sequential_score,
        parallel_score,
        estimated_speedup,
        recommended_strategy,
        parallel_friendly,
        cost_model_source: model.source,
        layer_costs,
    }
}

pub fn analyze_circuit_strategy(
    circuit: &Circuit,
    outputs: &[WireId],
    model: &CostModel,
) -> Result<(CircuitAnalysis, StrategyAnalysis), AnalysisError> {
    let circuit_analysis = analyze_circuit(circuit, outputs)?;

    let strategy_analysis = analyze_execution_strategy(&circuit_analysis, model);

    Ok((circuit_analysis, strategy_analysis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_gate_counts() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let not = circuit.add_not(xor);

        let output = circuit.add_and(not, a);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        assert_eq!(analysis.gate_count, 5);

        assert_eq!(analysis.wire_count, 5);

        assert_eq!(analysis.gate_counts.inputs, 2);

        assert_eq!(analysis.gate_counts.xor, 1);

        assert_eq!(analysis.gate_counts.not, 1);

        assert_eq!(analysis.gate_counts.and, 1);

        assert_eq!(analysis.gate_counts.compute_total(), 3);

        assert_eq!(analysis.gate_counts.total(), analysis.gate_count);
    }

    #[test]
    fn analyzes_circuit_depth() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let not = circuit.add_not(xor);

        let output = circuit.add_and(not, a);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        assert_eq!(analysis.max_depth, 3);

        assert_eq!(analysis.critical_output_depth, 3);

        assert_eq!(analysis.output_depths, vec![3]);
    }

    #[test]
    fn analyzes_parallelism() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let not = circuit.add_not(xor);

        let output = circuit.add_and(not, a);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        assert_eq!(analysis.gates_per_depth, vec![2, 1, 1, 1]);

        assert_eq!(analysis.max_parallelism, 2);

        assert_eq!(analysis.max_compute_parallelism, 1);
    }

    #[test]
    fn records_gate_types_per_depth() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let and = circuit.add_and(a, b);

        let output = circuit.add_or(xor, and);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        assert_eq!(analysis.compute_gate_counts_per_depth[1].xor, 1);

        assert_eq!(analysis.compute_gate_counts_per_depth[1].and, 1);

        assert_eq!(analysis.compute_gate_counts_per_depth[2].or, 1);
    }

    #[test]
    fn analyzes_fanout() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let and = circuit.add_and(a, b);

        let xor = circuit.add_xor(a, b);

        let output = circuit.add_or(and, xor);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        assert_eq!(analysis.max_fanout, 2);

        assert_eq!(analysis.max_compute_parallelism, 2);
    }

    #[test]
    fn analyzes_mux_depth() {
        let mut circuit = Circuit::new();

        let condition = circuit.add_input();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_mux(condition, a, b);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        assert_eq!(analysis.max_depth, 1);

        assert_eq!(analysis.gate_counts.mux, 1);

        assert_eq!(analysis.max_parallelism, 3);

        assert_eq!(analysis.max_compute_parallelism, 1);
    }

    #[test]
    fn rejects_invalid_output() {
        let circuit = Circuit::new();

        let result = analyze_circuit(&circuit, &[WireId(99)]);

        assert_eq!(result, Err(AnalysisError::InvalidOutputWire(WireId(99))));
    }

    #[test]
    fn sequential_chain_prefers_sequential_execution() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let first = circuit.add_xor(a, b);

        let second = circuit.add_not(first);

        let output = circuit.add_and(second, a);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        let strategy = analyze_execution_strategy(&analysis, &CostModel::default());

        assert_eq!(strategy.recommended_strategy, ExecutionStrategy::Sequential);

        assert!(!strategy.parallel_friendly);
    }

    #[test]
    fn wide_circuit_prefers_parallel_execution() {
        let mut circuit = Circuit::new();

        let mut current = Vec::new();

        for _ in 0..8 {
            let a = circuit.add_input();

            let b = circuit.add_input();

            current.push(circuit.add_xor(a, b));
        }

        while current.len() > 1 {
            let mut next = Vec::new();

            for pair in current.chunks(2) {
                if pair.len() == 2 {
                    next.push(circuit.add_or(pair[0], pair[1]));
                } else {
                    next.push(pair[0]);
                }
            }

            current = next;
        }

        let output = current[0];

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        let strategy = analyze_execution_strategy(&analysis, &CostModel::default());

        assert_eq!(strategy.recommended_strategy, ExecutionStrategy::Parallel);

        assert!(strategy.parallel_friendly);

        assert!(strategy.estimated_speedup > 1.0);
    }

    #[test]
    fn strategy_analysis_reports_work() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let output = circuit.add_and(xor, a);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        let strategy = analyze_execution_strategy(&analysis, &CostModel::default());

        assert_eq!(strategy.compute_gate_count, 2);

        assert_eq!(strategy.execution_layers, 2);

        assert!(strategy.weighted_work > 0.0);

        assert_eq!(strategy.layer_costs.len(), 2);
    }

    #[test]
    fn layer_model_accounts_for_gate_costs() {
        let mut circuit = Circuit::new();

        let condition = circuit.add_input();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let mux = circuit.add_mux(condition, a, b);

        let output = circuit.add_or(xor, mux);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        let model = CostModel::calibrated_nanoseconds(1.0, 10.0, 20.0, 30.0, 50.0, 0.5, 0.0);

        let strategy = analyze_execution_strategy(&analysis, &model);

        assert_eq!(strategy.layer_costs.len(), 2);

        assert_eq!(strategy.layer_costs[0].compute_gate_count, 2);

        assert_eq!(strategy.layer_costs[0].sequential_cost, 80.0);

        assert_eq!(strategy.layer_costs[1].sequential_cost, 20.0);
    }

    #[test]
    fn combined_analysis_returns_both_reports() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_xor(a, b);

        let (circuit_analysis, strategy_analysis) =
            analyze_circuit_strategy(&circuit, &[output], &CostModel::default()).unwrap();

        assert_eq!(circuit_analysis.gate_count, 3);

        assert_eq!(strategy_analysis.compute_gate_count, 1);
    }

    #[test]
    fn calibrated_model_reports_time() {
        let model = CostModel::calibrated_nanoseconds(10.0, 100.0, 100.0, 100.0, 150.0, 0.5, 20.0);

        assert!(model.is_calibrated());

        assert_eq!(model.source, CostModelSource::BackendCalibrated);
    }

    #[test]
    fn calibrated_layer_analysis_reports_milliseconds() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_and(a, b);

        let analysis = analyze_circuit(&circuit, &[output]).unwrap();

        let model = CostModel::calibrated_nanoseconds(
            1_000_000.0,
            10_000_000.0,
            10_000_000.0,
            10_000_000.0,
            20_000_000.0,
            0.5,
            0.0,
        );

        let strategy = analyze_execution_strategy(&analysis, &model);

        assert_eq!(strategy.layer_costs.len(), 1);

        assert_eq!(
            strategy.layer_costs[0].sequential_milliseconds(model.source,),
            Some(10.0)
        );
    }
}

pub mod uncertainty;
