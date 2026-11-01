use fhec_circuit::{Circuit, Gate, WireId};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::{
    AnalysisError, CircuitAnalysis, CostModelSource, GateCounts, analyze_circuit,
    crypto::{CryptoCostAnalysis, CryptoCostProfile, analyze_crypto_cost},
    noise::{NoiseAnalysis, NoiseProfile, analyze_noise},
    uncertainty::{PredictionInterval, PredictionUncertaintyProfile, estimate_prediction_interval},
};

pub const MAX_SCALING_POINTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateKind {
    Not,
    And,
    Or,
    Xor,
    Mux,
}

impl GateKind {
    pub const ALL: [GateKind; 5] = [
        GateKind::Not,
        GateKind::And,
        GateKind::Or,
        GateKind::Xor,
        GateKind::Mux,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScalingPoint {
    pub width: usize,
    pub parallel_ns: f64,
}

impl Default for ScalingPoint {
    fn default() -> Self {
        Self {
            width: 0,
            parallel_ns: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScalingCurve {
    pub points: [ScalingPoint; MAX_SCALING_POINTS],
    pub len: usize,
}

impl Default for ScalingCurve {
    fn default() -> Self {
        Self::new()
    }
}

impl ScalingCurve {
    pub fn new() -> Self {
        Self {
            points: [ScalingPoint::default(); MAX_SCALING_POINTS],
            len: 0,
        }
    }

    pub fn push(&mut self, width: usize, parallel_ns: f64) {
        if width == 0 || !parallel_ns.is_finite() || parallel_ns <= 0.0 {
            return;
        }

        if let Some(index) = self
            .points
            .iter()
            .take(self.len)
            .position(|point| point.width == width)
        {
            self.points[index] = ScalingPoint { width, parallel_ns };

            return;
        }

        if self.len >= MAX_SCALING_POINTS {
            return;
        }

        self.points[self.len] = ScalingPoint { width, parallel_ns };

        self.len += 1;

        self.points[..self.len].sort_by_key(|point| point.width);
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn max_width(&self) -> usize {
        if self.len == 0 {
            0
        } else {
            self.points[self.len - 1].width
        }
    }

    pub fn latency_ns(&self, width: usize, single_gate_ns: f64) -> f64 {
        if width == 0 {
            return 0.0;
        }

        if self.len == 0 {
            return single_gate_ns.max(0.0) * width as f64;
        }

        if width <= self.points[0].width {
            let first = self.points[0];

            if width == first.width {
                return first.parallel_ns;
            }

            let ratio = width as f64 / first.width as f64;

            return (first.parallel_ns * ratio).max(single_gate_ns.max(0.0));
        }

        for index in 1..self.len {
            let left = self.points[index - 1];
            let right = self.points[index];

            if width == right.width {
                return right.parallel_ns;
            }

            if width < right.width {
                let span = (right.width - left.width) as f64;

                let offset = (width - left.width) as f64;

                let t = offset / span;

                return left.parallel_ns + (right.parallel_ns - left.parallel_ns) * t;
            }
        }

        let last = self.points[self.len - 1];

        let waves = width.div_ceil(last.width).max(1);

        last.parallel_ns * waves as f64
    }

    pub fn speedup(&self, width: usize, single_gate_ns: f64) -> f64 {
        if width == 0 {
            return 1.0;
        }

        let sequential = single_gate_ns.max(0.0) * width as f64;

        let parallel = self.latency_ns(width, single_gate_ns);

        if sequential <= 0.0 || parallel <= 0.0 {
            1.0
        } else {
            (sequential / parallel).clamp(1.0, width as f64)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GateScalingProfiles {
    pub not: ScalingCurve,
    pub and: ScalingCurve,
    pub or: ScalingCurve,
    pub xor: ScalingCurve,
    pub mux: ScalingCurve,
}

impl GateScalingProfiles {
    pub fn curve(&self, kind: GateKind) -> &ScalingCurve {
        match kind {
            GateKind::Not => &self.not,
            GateKind::And => &self.and,
            GateKind::Or => &self.or,
            GateKind::Xor => &self.xor,
            GateKind::Mux => &self.mux,
        }
    }

    pub fn curve_mut(&mut self, kind: GateKind) -> &mut ScalingCurve {
        match kind {
            GateKind::Not => &mut self.not,
            GateKind::And => &mut self.and,
            GateKind::Or => &mut self.or,
            GateKind::Xor => &mut self.xor,
            GateKind::Mux => &mut self.mux,
        }
    }

    pub fn populated_curve_count(&self) -> usize {
        GateKind::ALL
            .iter()
            .filter(|kind| !self.curve(**kind).is_empty())
            .count()
    }

    pub fn is_complete(&self) -> bool {
        self.populated_curve_count() == GateKind::ALL.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub worker_threads: usize,
    pub calibration_width: usize,
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self {
            worker_threads: 1,
            calibration_width: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct V4CostModel {
    pub not_ns: f64,
    pub and_ns: f64,
    pub or_ns: f64,
    pub xor_ns: f64,
    pub mux_ns: f64,
    pub layer_sync_ns: f64,
    pub fallback_parallel_efficiency: f64,
    pub chunk_size: usize,
    pub source: CostModelSource,
    pub hardware: HardwareProfile,
    pub scaling: GateScalingProfiles,
    pub mixed_layer_scaling: ScalingCurve,
    pub steady_mixed_layer_scaling: ScalingCurve,
    pub narrow_deep_scaling: ScalingCurve,
    pub narrow_deep_max_width: usize,
    pub mixed_reference_gate_ns: f64,
    pub ciphertext_bytes: usize,
    pub crypto_profile: CryptoCostProfile,
    pub noise_profile: NoiseProfile,
    pub uncertainty_profile: PredictionUncertaintyProfile,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct V4GateLatencies {
    pub not_ns: f64,
    pub and_ns: f64,
    pub or_ns: f64,
    pub xor_ns: f64,
    pub mux_ns: f64,
}

impl V4CostModel {
    pub fn calibrated(
        gate_latencies: V4GateLatencies,
        layer_sync_ns: f64,
        fallback_parallel_efficiency: f64,
        hardware: HardwareProfile,
        scaling: GateScalingProfiles,
    ) -> Self {
        let V4GateLatencies {
            not_ns,
            and_ns,
            or_ns,
            xor_ns,
            mux_ns,
        } = gate_latencies;

        let mixed_reference_gate_ns =
            (and_ns.max(0.0) * 2.0 + or_ns.max(0.0) + xor_ns.max(0.0)) / 4.0;

        Self {
            not_ns: not_ns.max(0.0),

            and_ns: and_ns.max(0.0),

            or_ns: or_ns.max(0.0),

            xor_ns: xor_ns.max(0.0),

            mux_ns: mux_ns.max(0.0),

            layer_sync_ns: layer_sync_ns.max(0.0),

            fallback_parallel_efficiency: fallback_parallel_efficiency.clamp(0.01, 1.0),

            chunk_size: 4,

            source: CostModelSource::BackendCalibrated,

            hardware: HardwareProfile {
                worker_threads: hardware.worker_threads.max(1),

                calibration_width: hardware.calibration_width.max(1),
            },

            scaling,

            mixed_layer_scaling: ScalingCurve::new(),

            steady_mixed_layer_scaling: ScalingCurve::new(),

            narrow_deep_scaling: ScalingCurve::new(),

            narrow_deep_max_width: 0,

            mixed_reference_gate_ns,

            ciphertext_bytes: 0,
            crypto_profile: CryptoCostProfile::default(),
            noise_profile: NoiseProfile::default(),
            uncertainty_profile: PredictionUncertaintyProfile::default(),
        }
    }

    pub fn with_ciphertext_bytes(mut self, ciphertext_bytes: usize) -> Self {
        self.ciphertext_bytes = ciphertext_bytes;
        self
    }
    pub fn with_uncertainty_profile(
        mut self,
        uncertainty_profile: PredictionUncertaintyProfile,
    ) -> Self {
        self.uncertainty_profile = uncertainty_profile;
        self
    }

    pub fn with_crypto_profile(mut self, crypto_profile: CryptoCostProfile) -> Self {
        self.crypto_profile = crypto_profile;
        self
    }

    pub fn with_noise_profile(mut self, noise_profile: NoiseProfile) -> Self {
        self.noise_profile = noise_profile;
        self
    }

    pub fn gate_ns(&self, kind: GateKind) -> f64 {
        match kind {
            GateKind::Not => self.not_ns,

            GateKind::And => self.and_ns,

            GateKind::Or => self.or_ns,

            GateKind::Xor => self.xor_ns,

            GateKind::Mux => self.mux_ns,
        }
    }

    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size.max(1);

        self
    }

    pub fn with_mixed_layer_scaling(
        mut self,
        scaling: ScalingCurve,
        reference_gate_ns: f64,
    ) -> Self {
        self.mixed_layer_scaling = scaling;

        self.mixed_reference_gate_ns = reference_gate_ns.max(0.000001);

        self
    }

    pub fn with_steady_mixed_layer_scaling(mut self, scaling: ScalingCurve) -> Self {
        self.steady_mixed_layer_scaling = scaling;

        self
    }

    pub fn with_narrow_deep_scaling(mut self, scaling: ScalingCurve, max_width: usize) -> Self {
        self.narrow_deep_scaling = scaling;

        self.narrow_deep_max_width = max_width.max(1);

        self
    }

    pub fn is_calibrated(&self) -> bool {
        self.source == CostModelSource::BackendCalibrated
    }

    pub fn has_empirical_scaling(&self) -> bool {
        self.scaling.populated_curve_count() > 0
    }

    pub fn has_mixed_layer_scaling(&self) -> bool {
        !self.mixed_layer_scaling.is_empty() && self.mixed_reference_gate_ns > 0.0
    }

    pub fn has_steady_mixed_layer_scaling(&self) -> bool {
        !self.steady_mixed_layer_scaling.is_empty() && self.mixed_reference_gate_ns > 0.0
    }

    pub fn has_narrow_deep_scaling(&self) -> bool {
        !self.narrow_deep_scaling.is_empty()
            && self.narrow_deep_max_width > 0
            && self.mixed_reference_gate_ns > 0.0
    }
}

impl Default for V4CostModel {
    fn default() -> Self {
        Self {
            not_ns: 1.0,
            and_ns: 1.0,
            or_ns: 1.0,
            xor_ns: 1.0,
            mux_ns: 1.5,
            layer_sync_ns: 0.25,
            fallback_parallel_efficiency: 0.70,
            chunk_size: 4,
            source: CostModelSource::Heuristic,
            hardware: HardwareProfile::default(),
            scaling: GateScalingProfiles::default(),
            mixed_layer_scaling: ScalingCurve::new(),
            steady_mixed_layer_scaling: ScalingCurve::new(),
            narrow_deep_scaling: ScalingCurve::new(),
            narrow_deep_max_width: 0,
            mixed_reference_gate_ns: 1.0,

            ciphertext_bytes: 0,
            crypto_profile: CryptoCostProfile::default(),
            noise_profile: NoiseProfile::default(),
            uncertainty_profile: PredictionUncertaintyProfile::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum V4ExecutionStrategy {
    Sequential,
    ChunkedParallel,
    Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionConfidence {
    Uncalibrated,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleneckKind {
    DepthBound,
    ParallelismBound,
    GateCostBound,
    SynchronizationBound,
    Balanced,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CiphertextMemoryAnalysis {
    pub ciphertext_bytes: usize,
    pub produced_ciphertexts: usize,
    pub peak_live_ciphertexts: usize,
    pub peak_live_bytes: usize,
    pub total_ciphertext_bytes_created: usize,
    pub live_ciphertexts_per_depth: Vec<usize>,
}

impl CiphertextMemoryAnalysis {
    pub fn bytes_known(&self) -> bool {
        self.ciphertext_bytes > 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct V4LayerCost {
    pub depth: usize,
    pub gate_counts: GateCounts,
    pub compute_gate_count: usize,
    pub sequential_ns: f64,
    pub parallel_ns: f64,
    pub chunked_parallel_ns: f64,
    pub empirical_speedup: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BottleneckAnalysis {
    pub kind: BottleneckKind,
    pub severity: f64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct V4StrategyAnalysis {
    pub structural: CircuitAnalysis,
    pub layer_costs: Vec<V4LayerCost>,
    pub memory: CiphertextMemoryAnalysis,
    pub crypto: CryptoCostAnalysis,
    pub noise: NoiseAnalysis,
    pub prediction_interval: PredictionInterval,
    pub sequential_ns: f64,
    pub parallel_ns: f64,
    pub chunked_parallel_ns: f64,
    pub weighted_critical_path_ns: f64,
    pub estimated_speedup: f64,
    pub recommended_strategy: V4ExecutionStrategy,
    pub confidence: PredictionConfidence,
    pub bottleneck: BottleneckAnalysis,
}

impl V4StrategyAnalysis {
    pub fn selected_ns(&self) -> f64 {
        match self.recommended_strategy {
            V4ExecutionStrategy::Sequential => self.sequential_ns,

            V4ExecutionStrategy::ChunkedParallel => self.chunked_parallel_ns,

            V4ExecutionStrategy::Parallel => self.parallel_ns,
        }
    }

    pub fn sequential_milliseconds(&self) -> f64 {
        self.sequential_ns / 1_000_000.0
    }

    pub fn parallel_milliseconds(&self) -> f64 {
        self.parallel_ns / 1_000_000.0
    }

    pub fn chunked_parallel_milliseconds(&self) -> f64 {
        self.chunked_parallel_ns / 1_000_000.0
    }

    pub fn selected_milliseconds(&self) -> f64 {
        self.selected_ns() / 1_000_000.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionSample {
    pub predicted_ns: f64,
    pub actual_ns: f64,
    pub absolute_error_ns: f64,
    pub error_percent: f64,
}

impl PredictionSample {
    pub fn new(predicted_ns: f64, actual_ns: f64) -> Self {
        let predicted_ns = predicted_ns.max(0.0);

        let actual_ns = actual_ns.max(0.0);

        let absolute_error_ns = (predicted_ns - actual_ns).abs();

        let error_percent = if actual_ns > 0.0 {
            absolute_error_ns / actual_ns * 100.0
        } else {
            0.0
        };

        Self {
            predicted_ns,
            actual_ns,
            absolute_error_ns,
            error_percent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PredictionTracker {
    pub samples: Vec<PredictionSample>,
}

impl PredictionTracker {
    pub fn record(&mut self, predicted_ns: f64, actual_ns: f64) -> PredictionSample {
        let sample = PredictionSample::new(predicted_ns, actual_ns);

        self.samples.push(sample.clone());

        sample
    }

    pub fn mean_absolute_percentage_error(&self) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }

        Some(
            self.samples
                .iter()
                .map(|sample| sample.error_percent)
                .sum::<f64>()
                / self.samples.len() as f64,
        )
    }

    pub fn confidence(&self, model: &V4CostModel) -> PredictionConfidence {
        if !model.is_calibrated() {
            return PredictionConfidence::Uncalibrated;
        }

        let complete_profile = model.scaling.is_complete()
            && model.has_mixed_layer_scaling()
            && model.has_steady_mixed_layer_scaling()
            && model.has_narrow_deep_scaling();
        match self.mean_absolute_percentage_error() {
            None => {
                if complete_profile {
                    PredictionConfidence::Medium
                } else {
                    PredictionConfidence::Low
                }
            }

            Some(error) if error <= 10.0 && self.samples.len() >= 3 => PredictionConfidence::High,

            Some(error) if error <= 25.0 => PredictionConfidence::Medium,

            Some(_) => PredictionConfidence::Low,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitCandidate<'a> {
    pub name: &'a str,
    pub circuit: &'a Circuit,
    pub outputs: &'a [WireId],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateReport {
    pub name: String,
    pub analysis: V4StrategyAnalysis,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateComparison {
    pub candidates: Vec<CandidateReport>,

    pub selected_index: usize,
}

impl CandidateComparison {
    pub fn selected(&self) -> &CandidateReport {
        &self.candidates[self.selected_index]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationComparison {
    pub before: V4StrategyAnalysis,

    pub after: V4StrategyAnalysis,

    pub gate_delta: isize,

    pub depth_delta: isize,

    pub predicted_latency_delta_ns: f64,

    pub predicted_speedup: f64,
}

pub fn analyze_v4(
    circuit: &Circuit,
    outputs: &[WireId],
    model: &V4CostModel,
) -> Result<V4StrategyAnalysis, AnalysisError> {
    let structural = analyze_circuit(circuit, outputs)?;
    let memory = analyze_ciphertext_memory_with_depth(
        circuit,
        outputs,
        structural.max_depth,
        model.ciphertext_bytes,
    );

    let crypto = analyze_crypto_cost(&structural.gate_counts, &model.crypto_profile);
    let noise = analyze_noise(
        structural.gate_counts.compute_total(),
        structural.max_depth,
        &crypto,
        &model.noise_profile,
    );

    let mut seen_compute_layer = false;

    let layer_costs = structural
        .compute_gate_counts_per_depth
        .iter()
        .enumerate()
        .filter_map(|(depth, counts)| {
            if counts.compute_total() == 0 {
                None
            } else {
                let steady_state = seen_compute_layer;

                seen_compute_layer = true;

                Some(analyze_layer(depth, counts, model, steady_state))
            }
        })
        .collect::<Vec<_>>();

    let sequential_ns = layer_costs
        .iter()
        .map(|layer| layer.sequential_ns)
        .sum::<f64>();

    let parallel_ns = layer_costs
        .iter()
        .map(|layer| layer.parallel_ns)
        .sum::<f64>();

    let chunked_parallel_ns = layer_costs
        .iter()
        .map(|layer| layer.chunked_parallel_ns)
        .sum::<f64>();

    let weighted_critical_path_ns = weighted_critical_path(circuit, outputs, model);

    let recommended_strategy = choose_strategy(
        sequential_ns,
        chunked_parallel_ns,
        parallel_ns,
        structural.gate_counts.compute_total(),
    );

    let selected_ns = match recommended_strategy {
        V4ExecutionStrategy::Sequential => sequential_ns,

        V4ExecutionStrategy::ChunkedParallel => chunked_parallel_ns,

        V4ExecutionStrategy::Parallel => parallel_ns,
    };
    let prediction_interval = estimate_prediction_interval(selected_ns, &model.uncertainty_profile);

    let estimated_speedup = if selected_ns > 0.0 {
        sequential_ns / selected_ns
    } else {
        1.0
    };

    let confidence = if !model.is_calibrated() {
        PredictionConfidence::Uncalibrated
    } else if model.scaling.is_complete()
        && model.has_mixed_layer_scaling()
        && model.has_steady_mixed_layer_scaling()
        && model.has_narrow_deep_scaling()
    {
        PredictionConfidence::Medium
    } else {
        PredictionConfidence::Low
    };

    let bottleneck = detect_bottleneck(
        &structural,
        model,
        sequential_ns,
        parallel_ns,
        weighted_critical_path_ns,
    );

    Ok(V4StrategyAnalysis {
        structural,
        layer_costs,
        memory,
        crypto,
        noise,
        prediction_interval,
        sequential_ns,
        parallel_ns,
        chunked_parallel_ns,
        weighted_critical_path_ns,
        estimated_speedup,
        recommended_strategy,
        confidence,
        bottleneck,
    })
}

pub fn compare_candidates(
    candidates: &[CircuitCandidate<'_>],

    model: &V4CostModel,
) -> Result<CandidateComparison, AnalysisError> {
    assert!(!candidates.is_empty(), "candidate list must not be empty");

    let mut reports = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        reports.push(CandidateReport {
            name: candidate.name.to_string(),

            analysis: analyze_v4(candidate.circuit, candidate.outputs, model)?,
        });
    }

    let selected_index = reports
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            left.analysis
                .selected_ns()
                .total_cmp(&right.analysis.selected_ns())
        })
        .map(|(index, _)| index)
        .unwrap_or(0);

    Ok(CandidateComparison {
        candidates: reports,

        selected_index,
    })
}

pub fn compare_optimization(
    before: &Circuit,
    before_outputs: &[WireId],
    after: &Circuit,
    after_outputs: &[WireId],
    model: &V4CostModel,
) -> Result<OptimizationComparison, AnalysisError> {
    let before_analysis = analyze_v4(before, before_outputs, model)?;

    let after_analysis = analyze_v4(after, after_outputs, model)?;

    let before_latency = before_analysis.selected_ns();

    let after_latency = after_analysis.selected_ns();

    let predicted_speedup = if after_latency > 0.0 {
        before_latency / after_latency
    } else {
        1.0
    };

    Ok(OptimizationComparison {
        gate_delta: after_analysis.structural.gate_count as isize
            - before_analysis.structural.gate_count as isize,

        depth_delta: after_analysis.structural.max_depth as isize
            - before_analysis.structural.max_depth as isize,

        predicted_latency_delta_ns: after_latency - before_latency,

        predicted_speedup,

        before: before_analysis,

        after: after_analysis,
    })
}

pub fn render_v4_report(analysis: &V4StrategyAnalysis) -> String {
    let mut report = String::new();

    let _ = writeln!(report, "FHEC ANALYZER V4");

    let _ = writeln!(report, "================");

    let _ = writeln!(report, "Gates: {}", analysis.structural.gate_count);

    let _ = writeln!(
        report,
        "Compute gates: {}",
        analysis.structural.gate_counts.compute_total()
    );

    let _ = writeln!(report, "Depth: {}", analysis.structural.max_depth);

    let _ = writeln!(
        report,
        "Produced ciphertexts: {}",
        analysis.memory.produced_ciphertexts
    );

    let _ = writeln!(
        report,
        "Peak live ciphertexts: {}",
        analysis.memory.peak_live_ciphertexts
    );

    if analysis.memory.bytes_known() {
        let _ = writeln!(
            report,
            "Peak ciphertext memory: {:.3} MiB",
            analysis.memory.peak_live_bytes as f64 / (1024.0 * 1024.0)
        );

        let _ = writeln!(
            report,
            "Total ciphertext allocation: {:.3} MiB",
            analysis.memory.total_ciphertext_bytes_created as f64 / (1024.0 * 1024.0)
        );
    }

    if analysis.crypto.has_crypto_work() {
        let _ = writeln!(report);
        let _ = writeln!(report, "CRYPTO OPERATIONS");
        let _ = writeln!(report, "=================");
        let _ = writeln!(
            report,
            "Bootstraps: {}",
            analysis.crypto.operations.bootstraps
        );
        let _ = writeln!(
            report,
            "Key switches: {}",
            analysis.crypto.operations.key_switches
        );
        let _ = writeln!(
            report,
            "Refreshes: {}",
            analysis.crypto.operations.refreshes
        );
        let _ = writeln!(
            report,
            "Estimated bootstrap cost: {:.3} ms",
            analysis.crypto.bootstrap_milliseconds()
        );
        let _ = writeln!(
            report,
            "Estimated key-switch cost: {:.3} ms",
            analysis.crypto.key_switch_milliseconds()
        );
        let _ = writeln!(
            report,
            "Estimated refresh cost: {:.3} ms",
            analysis.crypto.refresh_milliseconds()
        );
        let _ = writeln!(
            report,
            "Estimated crypto cost: {:.3} ms",
            analysis.crypto.total_crypto_milliseconds()
        );
        let _ = writeln!(
            report,
            "Dominant crypto primitive: {:?}",
            analysis.crypto.dominant
        );
        let _ = writeln!(report);
    }

    let _ = writeln!(
        report,
        "Weighted critical path: {:.3} ms",
        analysis.weighted_critical_path_ns / 1_000_000.0
    );

    let _ = writeln!(
        report,
        "Sequential estimate: {:.3} ms",
        analysis.sequential_milliseconds()
    );

    let _ = writeln!(
        report,
        "Chunked estimate: {:.3} ms",
        analysis.chunked_parallel_milliseconds()
    );

    let _ = writeln!(
        report,
        "Parallel estimate: {:.3} ms",
        analysis.parallel_milliseconds()
    );

    let _ = writeln!(report, "Selected: {:?}", analysis.recommended_strategy);

    let _ = writeln!(report, "Confidence: {:?}", analysis.confidence);

    let _ = writeln!(
        report,
        "Bottleneck: {:?} ({:.0}%)",
        analysis.bottleneck.kind,
        analysis.bottleneck.severity * 100.0
    );

    let _ = writeln!(report, "Reason: {}", analysis.bottleneck.message);

    let _ = writeln!(report);

    let _ = writeln!(report, "LAYERS");

    let _ = writeln!(report, "======");

    for layer in &analysis.layer_costs {
        let _ = writeln!(
            report,
            "Depth {}: gates={}, seq={:.3} ms, chunked={:.3} ms, parallel={:.3} ms, speedup={:.2}x",
            layer.depth,
            layer.compute_gate_count,
            layer.sequential_ns / 1_000_000.0,
            layer.chunked_parallel_ns / 1_000_000.0,
            layer.parallel_ns / 1_000_000.0,
            layer.empirical_speedup,
        );
    }

    report
}

fn analyze_layer(
    depth: usize,
    counts: &GateCounts,
    model: &V4CostModel,
    steady_state: bool,
) -> V4LayerCost {
    let compute_gate_count = counts.compute_total();

    let sequential_ns = weighted_gate_work(counts, model);

    let parallel_ns = if model.has_empirical_scaling() || model.has_mixed_layer_scaling() {
        empirical_layer_latency(counts, model, steady_state)
    } else {
        fallback_parallel_latency(compute_gate_count, sequential_ns, model)
    };

    let chunked_parallel_ns = chunked_layer_latency(counts, model);

    let empirical_speedup = if parallel_ns > 0.0 {
        sequential_ns / parallel_ns
    } else {
        1.0
    };

    V4LayerCost {
        depth,
        gate_counts: counts.clone(),
        compute_gate_count,
        sequential_ns,
        parallel_ns,
        chunked_parallel_ns,
        empirical_speedup,
    }
}

fn weighted_gate_work(counts: &GateCounts, model: &V4CostModel) -> f64 {
    GateKind::ALL
        .iter()
        .map(|kind| gate_count(counts, *kind) as f64 * model.gate_ns(*kind))
        .sum()
}

fn empirical_layer_latency(counts: &GateCounts, model: &V4CostModel, steady_state: bool) -> f64 {
    let total_count = counts.compute_total();

    if total_count == 0 {
        return 0.0;
    }

    let total_work = weighted_gate_work(counts, model);

    if total_work <= 0.0 {
        return 0.0;
    }

    let can_use_mixed_profile =
        model.has_mixed_layer_scaling() && counts.mux == 0 && counts.not == 0;

    if can_use_mixed_profile {
        let reference_gate_ns = model.mixed_reference_gate_ns.max(0.000001);

        let actual_average_gate_ns = total_work / total_count as f64;

        let composition_scale = actual_average_gate_ns / reference_gate_ns;

        let use_narrow_deep = steady_state
            && model.has_narrow_deep_scaling()
            && total_count <= model.narrow_deep_max_width;

        let curve = if use_narrow_deep {
            &model.narrow_deep_scaling
        } else if steady_state && model.has_steady_mixed_layer_scaling() {
            &model.steady_mixed_layer_scaling
        } else {
            &model.mixed_layer_scaling
        };

        let measured_latency = curve.latency_ns(total_count, reference_gate_ns);

        return measured_latency * composition_scale;
    }

    let mut weighted_speedup = 0.0;

    for kind in GateKind::ALL {
        let count = gate_count(counts, kind);

        if count == 0 {
            continue;
        }

        let gate_ns = model.gate_ns(kind);

        let work = gate_ns * count as f64;

        let curve = model.scaling.curve(kind);

        let speedup = if curve.is_empty() {
            let capacity = model.hardware.worker_threads.min(count).max(1) as f64;

            (capacity * model.fallback_parallel_efficiency).max(1.0)
        } else {
            curve.speedup(count, gate_ns)
        };

        weighted_speedup += work * speedup;
    }

    let effective_speedup = (weighted_speedup / total_work).max(1.0);

    total_work / effective_speedup + model.layer_sync_ns
}
fn fallback_parallel_latency(gate_count: usize, sequential_ns: f64, model: &V4CostModel) -> f64 {
    if gate_count == 0 {
        return 0.0;
    }

    let capacity = model.hardware.worker_threads.min(gate_count).max(1) as f64;

    let effective = (capacity * model.fallback_parallel_efficiency).max(1.0);

    sequential_ns / effective + model.layer_sync_ns
}

fn chunked_layer_latency(counts: &GateCounts, model: &V4CostModel) -> f64 {
    let total_count = counts.compute_total();

    if total_count == 0 {
        return 0.0;
    }

    let chunk_size = model.chunk_size.max(1);

    let chunk_count = total_count.div_ceil(chunk_size);

    let worker_chunks = model.hardware.worker_threads.min(chunk_count).max(1);

    let total_work = weighted_gate_work(counts, model);

    let average_chunk_work = total_work / chunk_count as f64;

    let waves = chunk_count.div_ceil(worker_chunks).max(1);

    average_chunk_work * waves as f64 + model.layer_sync_ns
}
pub fn analyze_ciphertext_memory(
    circuit: &Circuit,
    outputs: &[WireId],
    ciphertext_bytes: usize,
) -> Result<CiphertextMemoryAnalysis, AnalysisError> {
    let structural = analyze_circuit(circuit, outputs)?;

    Ok(analyze_ciphertext_memory_with_depth(
        circuit,
        outputs,
        structural.max_depth,
        ciphertext_bytes,
    ))
}

fn analyze_ciphertext_memory_with_depth(
    circuit: &Circuit,
    outputs: &[WireId],
    final_depth: usize,
    ciphertext_bytes: usize,
) -> CiphertextMemoryAnalysis {
    let wire_count = circuit.wire_count();

    let mut produced_depth = vec![0usize; wire_count];
    let mut last_use_depth = vec![0usize; wire_count];
    let mut is_ciphertext = vec![false; wire_count];

    for gate in circuit.gates() {
        match gate {
            Gate::Constant { wire, .. } => {
                produced_depth[wire.0 as usize] = 0;
            }

            Gate::Input { wire, .. } => {
                let index = wire.0 as usize;

                produced_depth[index] = 0;
                last_use_depth[index] = 0;
                is_ciphertext[index] = true;
            }

            Gate::Not { wire, operand, .. } => {
                let depth = produced_depth[operand.0 as usize] + 1;

                let wire_index = wire.0 as usize;
                let operand_index = operand.0 as usize;

                produced_depth[wire_index] = depth;
                last_use_depth[wire_index] = depth;
                is_ciphertext[wire_index] = true;

                last_use_depth[operand_index] = last_use_depth[operand_index].max(depth);
            }

            Gate::And { wire, lhs, rhs, .. }
            | Gate::Or { wire, lhs, rhs, .. }
            | Gate::Xor { wire, lhs, rhs, .. } => {
                let depth = produced_depth[lhs.0 as usize].max(produced_depth[rhs.0 as usize]) + 1;

                let wire_index = wire.0 as usize;
                let lhs_index = lhs.0 as usize;
                let rhs_index = rhs.0 as usize;

                produced_depth[wire_index] = depth;
                last_use_depth[wire_index] = depth;
                is_ciphertext[wire_index] = true;

                last_use_depth[lhs_index] = last_use_depth[lhs_index].max(depth);

                last_use_depth[rhs_index] = last_use_depth[rhs_index].max(depth);
            }

            Gate::Mux {
                wire,
                condition,
                if_true,
                if_false,
                ..
            } => {
                let depth = produced_depth[condition.0 as usize]
                    .max(produced_depth[if_true.0 as usize])
                    .max(produced_depth[if_false.0 as usize])
                    + 1;

                let wire_index = wire.0 as usize;
                let condition_index = condition.0 as usize;
                let true_index = if_true.0 as usize;
                let false_index = if_false.0 as usize;

                produced_depth[wire_index] = depth;
                last_use_depth[wire_index] = depth;
                is_ciphertext[wire_index] = true;

                last_use_depth[condition_index] = last_use_depth[condition_index].max(depth);

                last_use_depth[true_index] = last_use_depth[true_index].max(depth);

                last_use_depth[false_index] = last_use_depth[false_index].max(depth);
            }
        }
    }

    for output in outputs {
        let index = output.0 as usize;

        if index < wire_count && is_ciphertext[index] {
            last_use_depth[index] = last_use_depth[index].max(final_depth);
        }
    }

    let produced_ciphertexts = is_ciphertext.iter().filter(|value| **value).count();

    let mut live_ciphertexts_per_depth = vec![0usize; final_depth.saturating_add(1)];

    for wire_index in 0..wire_count {
        if !is_ciphertext[wire_index] {
            continue;
        }

        let start = produced_depth[wire_index];
        let end = last_use_depth[wire_index];

        if start >= live_ciphertexts_per_depth.len() {
            continue;
        }

        let capped_end = end.min(live_ciphertexts_per_depth.len() - 1);

        for count in live_ciphertexts_per_depth
            .iter_mut()
            .take(capped_end + 1)
            .skip(start)
        {
            *count += 1;
        }
    }

    let peak_live_ciphertexts = live_ciphertexts_per_depth
        .iter()
        .copied()
        .max()
        .unwrap_or(0);

    let peak_live_bytes = peak_live_ciphertexts.saturating_mul(ciphertext_bytes);

    let total_ciphertext_bytes_created = produced_ciphertexts.saturating_mul(ciphertext_bytes);

    CiphertextMemoryAnalysis {
        ciphertext_bytes,
        produced_ciphertexts,
        peak_live_ciphertexts,
        peak_live_bytes,
        total_ciphertext_bytes_created,
        live_ciphertexts_per_depth,
    }
}

fn weighted_critical_path(circuit: &Circuit, outputs: &[WireId], model: &V4CostModel) -> f64 {
    let mut costs = vec![0.0f64; circuit.wire_count()];

    for gate in circuit.gates() {
        let (wire, parents, gate_cost) = match gate {
            Gate::Constant { wire, .. } | Gate::Input { wire, .. } => {
                (*wire, [None, None, None], 0.0)
            }

            Gate::Not { wire, operand, .. } => (*wire, [Some(*operand), None, None], model.not_ns),

            Gate::And { wire, lhs, rhs, .. } => {
                (*wire, [Some(*lhs), Some(*rhs), None], model.and_ns)
            }

            Gate::Or { wire, lhs, rhs, .. } => (*wire, [Some(*lhs), Some(*rhs), None], model.or_ns),

            Gate::Xor { wire, lhs, rhs, .. } => {
                (*wire, [Some(*lhs), Some(*rhs), None], model.xor_ns)
            }

            Gate::Mux {
                wire,
                condition,
                if_true,
                if_false,
                ..
            } => (
                *wire,
                [Some(*condition), Some(*if_true), Some(*if_false)],
                model.mux_ns,
            ),
        };

        let parent_cost = parents
            .iter()
            .flatten()
            .map(|parent| costs[parent.0 as usize])
            .fold(0.0f64, f64::max);

        costs[wire.0 as usize] = parent_cost + gate_cost;
    }

    outputs
        .iter()
        .map(|wire| costs[wire.0 as usize])
        .fold(0.0f64, f64::max)
}

fn choose_strategy(
    sequential_ns: f64,
    chunked_parallel_ns: f64,
    parallel_ns: f64,
    compute_gate_count: usize,
) -> V4ExecutionStrategy {
    if compute_gate_count < 4 {
        return V4ExecutionStrategy::Sequential;
    }

    let mut best = (V4ExecutionStrategy::Sequential, sequential_ns);

    if chunked_parallel_ns < best.1 {
        best = (V4ExecutionStrategy::ChunkedParallel, chunked_parallel_ns);
    }

    if parallel_ns < best.1 {
        best = (V4ExecutionStrategy::Parallel, parallel_ns);
    }

    best.0
}

fn detect_bottleneck(
    structural: &CircuitAnalysis,
    model: &V4CostModel,
    sequential_ns: f64,
    parallel_ns: f64,
    critical_path_ns: f64,
) -> BottleneckAnalysis {
    let compute = structural.gate_counts.compute_total().max(1);

    let execution_layers = structural.max_depth.max(1);

    let average_parallelism = compute as f64 / execution_layers as f64;

    let critical_ratio = if sequential_ns > 0.0 {
        (critical_path_ns / sequential_ns).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let sync_total = model.layer_sync_ns * execution_layers as f64;

    let sync_ratio = if parallel_ns > 0.0 {
        (sync_total / parallel_ns).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let weighted_average_gate = sequential_ns / compute as f64;

    let maximum_gate = GateKind::ALL
        .iter()
        .map(|kind| model.gate_ns(*kind))
        .fold(0.0f64, f64::max);

    if critical_ratio >= 0.55 {
        return BottleneckAnalysis {
            kind:
                BottleneckKind::DepthBound,

            severity:
                critical_ratio,

            message:
                "The latency-weighted dependency chain dominates total work; reducing circuit depth is the highest-value optimization."
                    .to_string(),
        };
    }

    if average_parallelism < 2.0 {
        return BottleneckAnalysis {
            kind:
                BottleneckKind::ParallelismBound,

            severity:
                (
                    1.0
                        - average_parallelism
                            / 2.0
                )
                    .clamp(
                        0.0,
                        1.0,
                    ),

            message:
                "The circuit exposes too little independent work per layer for the available workers."
                    .to_string(),
        };
    }

    if sync_ratio >= 0.20 {
        return BottleneckAnalysis {
            kind:
                BottleneckKind::SynchronizationBound,

            severity:
                sync_ratio,

            message:
                "Layer synchronization is a significant fraction of predicted parallel latency; wider layers or fewer barriers would help."
                    .to_string(),
        };
    }

    if weighted_average_gate > 0.0 && maximum_gate >= weighted_average_gate * 1.75 {
        return BottleneckAnalysis {
            kind:
                BottleneckKind::GateCostBound,

            severity:
                (
                    (
                        maximum_gate
                            / weighted_average_gate
                            - 1.0
                    ) / 3.0
                )
                    .clamp(
                        0.0,
                        1.0,
                    ),

            message:
                "A high-cost gate family dominates the backend model; lowering or replacing those operations is likely to matter more than gate-count reduction alone."
                    .to_string(),
        };
    }

    BottleneckAnalysis {
        kind: BottleneckKind::Balanced,

        severity: 0.0,

        message: "No single structural or backend cost dominates the current prediction."
            .to_string(),
    }
}

fn gate_count(counts: &GateCounts, kind: GateKind) -> usize {
    match kind {
        GateKind::Not => counts.not,

        GateKind::And => counts.and,

        GateKind::Or => counts.or,

        GateKind::Xor => counts.xor,

        GateKind::Mux => counts.mux,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_curve_interpolates() {
        let mut curve = ScalingCurve::new();

        curve.push(1, 10.0);

        curve.push(4, 20.0);

        let value = curve.latency_ns(2, 10.0);

        assert!(value > 10.0);

        assert!(value < 20.0);
    }

    #[test]
    fn scaling_curve_handles_waves() {
        let mut curve = ScalingCurve::new();

        curve.push(4, 20.0);

        assert_eq!(curve.latency_ns(9, 10.0,), 60.0);
    }

    #[test]
    fn v4_prefers_parallel_for_wide_layer() {
        let mut circuit = Circuit::new();

        let mut outputs = Vec::new();

        for _ in 0..8 {
            let a = circuit.add_input();

            let b = circuit.add_input();

            outputs.push(circuit.add_xor(a, b));
        }

        let mut scaling = GateScalingProfiles::default();

        scaling.xor.push(1, 10.0);

        scaling.xor.push(8, 20.0);

        let model = V4CostModel::calibrated(
            V4GateLatencies {
                not_ns: 10.0,
                and_ns: 10.0,
                or_ns: 10.0,
                xor_ns: 10.0,
                mux_ns: 20.0,
            },
            0.0,
            0.7,
            HardwareProfile {
                worker_threads: 8,
                calibration_width: 8,
            },
            scaling,
        );

        let analysis = analyze_v4(&circuit, &outputs, &model).unwrap();

        assert_eq!(analysis.recommended_strategy, V4ExecutionStrategy::Parallel);
    }

    #[test]
    fn prediction_tracker_reports_error() {
        let mut tracker = PredictionTracker::default();

        let sample = tracker.record(200.0, 100.0);

        assert_eq!(sample.error_percent, 100.0);
    }

    #[test]
    fn candidate_comparison_selects_lower_cost() {
        let mut first = Circuit::new();

        let a = first.add_input();

        let b = first.add_input();

        let first_output = first.add_and(a, b);

        let mut second = Circuit::new();

        let c = second.add_input();

        let d = second.add_input();

        let mid = second.add_and(c, d);

        let second_output = second.add_not(mid);

        let first_outputs = [first_output];

        let second_outputs = [second_output];

        let candidates = [
            CircuitCandidate {
                name: "first",
                circuit: &first,
                outputs: &first_outputs,
            },
            CircuitCandidate {
                name: "second",
                circuit: &second,
                outputs: &second_outputs,
            },
        ];

        let comparison = compare_candidates(&candidates, &V4CostModel::default()).unwrap();

        assert_eq!(comparison.selected().name, "first");
    }

    #[test]
    fn mixed_layer_profile_overrides_synthetic_gate_combination() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let c = circuit.add_input();

        let d = circuit.add_input();

        let first = circuit.add_and(a, b);

        let second = circuit.add_or(c, d);

        let third = circuit.add_xor(a, c);

        let fourth = circuit.add_and(b, d);

        let outputs = [first, second, third, fourth];

        let scaling = GateScalingProfiles::default();

        let mut mixed = ScalingCurve::new();

        mixed.push(1, 10.0);

        mixed.push(4, 20.0);

        let model = V4CostModel::calibrated(
            V4GateLatencies {
                not_ns: 10.0,
                and_ns: 10.0,
                or_ns: 10.0,
                xor_ns: 10.0,
                mux_ns: 20.0,
            },
            0.0,
            0.7,
            HardwareProfile {
                worker_threads: 4,
                calibration_width: 4,
            },
            scaling,
        )
        .with_mixed_layer_scaling(mixed, 10.0);

        let analysis = analyze_v4(&circuit, &outputs, &model).unwrap();

        assert_eq!(analysis.layer_costs.len(), 1);

        assert_eq!(analysis.layer_costs[0].parallel_ns, 20.0);
    }
}

#[test]
fn steady_profile_is_used_after_first_compute_layer() {
    let mut circuit = Circuit::new();

    let a = circuit.add_input();

    let b = circuit.add_input();

    let first = circuit.add_and(a, b);

    let second = circuit.add_and(first, b);

    let outputs = [second];

    let scaling = GateScalingProfiles::default();

    let mut mixed = ScalingCurve::new();

    mixed.push(1, 20.0);

    let mut steady = ScalingCurve::new();

    steady.push(1, 10.0);

    let model = V4CostModel::calibrated(
        V4GateLatencies {
            not_ns: 10.0,
            and_ns: 10.0,
            or_ns: 10.0,
            xor_ns: 10.0,
            mux_ns: 20.0,
        },
        0.0,
        0.7,
        HardwareProfile {
            worker_threads: 4,
            calibration_width: 4,
        },
        scaling,
    )
    .with_mixed_layer_scaling(mixed, 10.0)
    .with_steady_mixed_layer_scaling(steady);

    let analysis = analyze_v4(&circuit, &outputs, &model).unwrap();

    assert_eq!(analysis.layer_costs.len(), 2);

    assert_eq!(analysis.layer_costs[0].parallel_ns, 20.0);

    assert_eq!(analysis.layer_costs[1].parallel_ns, 10.0);
}

#[test]
fn ciphertext_memory_tracks_lifetimes() {
    let mut circuit = Circuit::new();

    let a = circuit.add_input();
    let b = circuit.add_input();

    let first = circuit.add_and(a, b);
    let second = circuit.add_xor(first, b);

    let outputs = [second];

    let memory = analyze_ciphertext_memory(&circuit, &outputs, 1024).unwrap();

    assert_eq!(memory.produced_ciphertexts, 4);

    assert_eq!(memory.peak_live_ciphertexts, 3);

    assert_eq!(memory.peak_live_bytes, 3072);

    assert_eq!(memory.total_ciphertext_bytes_created, 4096);

    assert_eq!(memory.live_ciphertexts_per_depth, vec![2, 3, 3]);
}
