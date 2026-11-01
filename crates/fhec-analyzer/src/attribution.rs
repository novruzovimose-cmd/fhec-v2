use fhec_circuit::{Circuit, Gate};

use crate::{
    crypto::CryptoPrimitiveCounts,
    v4::{GateKind, V4CostModel},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributionRegion {
    pub operation_index: usize,
    pub operation_kind: String,
    pub gate_start: usize,
    pub gate_end: usize,
}

impl AttributionRegion {
    pub fn new(
        operation_index: usize,
        operation_kind: impl Into<String>,
        gate_start: usize,
        gate_end: usize,
    ) -> Self {
        Self {
            operation_index,
            operation_kind: operation_kind.into(),
            gate_start,
            gate_end,
        }
    }

    pub fn gate_count(&self) -> usize {
        self.gate_end.saturating_sub(self.gate_start)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegionCostAttribution {
    pub operation_index: usize,
    pub operation_kind: String,
    pub gate_start: usize,
    pub gate_end: usize,
    pub gate_count: usize,
    pub compute_gate_count: usize,
    pub ciphertexts_produced: usize,
    pub crypto_operations: CryptoPrimitiveCounts,
    pub estimated_gate_work_ns: f64,
    pub work_share: f64,
}

impl RegionCostAttribution {
    pub fn estimated_gate_work_milliseconds(&self) -> f64 {
        self.estimated_gate_work_ns / 1_000_000.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostAttributionAnalysis {
    pub regions: Vec<RegionCostAttribution>,
    pub total_attributed_gate_work_ns: f64,
    pub unattributed_gate_count: usize,
}

impl CostAttributionAnalysis {
    pub fn dominant_region(&self) -> Option<&RegionCostAttribution> {
        self.regions.iter().max_by(|left, right| {
            left.estimated_gate_work_ns
                .total_cmp(&right.estimated_gate_work_ns)
        })
    }
}

pub fn analyze_cost_attribution(
    circuit: &Circuit,
    regions: &[AttributionRegion],
    model: &V4CostModel,
) -> CostAttributionAnalysis {
    let mut raw_regions = Vec::with_capacity(regions.len());

    let mut total_attributed_gates = 0usize;
    let mut total_attributed_gate_work_ns = 0.0;

    for region in regions {
        let start = region.gate_start.min(circuit.gate_count());
        let end = region.gate_end.min(circuit.gate_count()).max(start);

        let gates = &circuit.gates()[start..end];

        let mut compute_gate_count = 0usize;
        let mut ciphertexts_produced = 0usize;
        let mut crypto_operations = CryptoPrimitiveCounts::default();

        let mut estimated_gate_work_ns = 0.0;

        for gate in gates {
            match gate {
                Gate::Constant { .. } => {}

                Gate::Input { .. } => {
                    ciphertexts_produced += 1;
                }

                Gate::Not { .. } => {
                    compute_gate_count += 1;
                    ciphertexts_produced += 1;

                    estimated_gate_work_ns += model.gate_ns(GateKind::Not);

                    crypto_operations =
                        crypto_operations.saturating_add(model.crypto_profile.gates.not);
                }

                Gate::And { .. } => {
                    compute_gate_count += 1;
                    ciphertexts_produced += 1;

                    estimated_gate_work_ns += model.gate_ns(GateKind::And);

                    crypto_operations =
                        crypto_operations.saturating_add(model.crypto_profile.gates.and);
                }

                Gate::Or { .. } => {
                    compute_gate_count += 1;
                    ciphertexts_produced += 1;

                    estimated_gate_work_ns += model.gate_ns(GateKind::Or);

                    crypto_operations =
                        crypto_operations.saturating_add(model.crypto_profile.gates.or);
                }

                Gate::Xor { .. } => {
                    compute_gate_count += 1;
                    ciphertexts_produced += 1;

                    estimated_gate_work_ns += model.gate_ns(GateKind::Xor);

                    crypto_operations =
                        crypto_operations.saturating_add(model.crypto_profile.gates.xor);
                }

                Gate::Mux { .. } => {
                    compute_gate_count += 1;
                    ciphertexts_produced += 1;

                    estimated_gate_work_ns += model.gate_ns(GateKind::Mux);

                    crypto_operations =
                        crypto_operations.saturating_add(model.crypto_profile.gates.mux);
                }
            }
        }

        total_attributed_gates += gates.len();

        total_attributed_gate_work_ns += estimated_gate_work_ns;

        raw_regions.push((
            region,
            gates.len(),
            compute_gate_count,
            ciphertexts_produced,
            crypto_operations,
            estimated_gate_work_ns,
        ));
    }

    let regions = raw_regions
        .into_iter()
        .map(
            |(
                region,
                gate_count,
                compute_gate_count,
                ciphertexts_produced,
                crypto_operations,
                estimated_gate_work_ns,
            )| {
                let work_share = if total_attributed_gate_work_ns > 0.0 {
                    estimated_gate_work_ns / total_attributed_gate_work_ns
                } else {
                    0.0
                };

                RegionCostAttribution {
                    operation_index: region.operation_index,
                    operation_kind: region.operation_kind.clone(),
                    gate_start: region.gate_start,
                    gate_end: region.gate_end,
                    gate_count,
                    compute_gate_count,
                    ciphertexts_produced,
                    crypto_operations,
                    estimated_gate_work_ns,
                    work_share,
                }
            },
        )
        .collect();

    CostAttributionAnalysis {
        regions,
        total_attributed_gate_work_ns,
        unattributed_gate_count: circuit.gate_count().saturating_sub(total_attributed_gates),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribution_tracks_gate_ranges() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();
        let b = circuit.add_input();

        let start = circuit.gate_count();

        let x = circuit.add_xor(a, b);
        let output = circuit.add_and(x, a);

        let end = circuit.gate_count();

        let regions = vec![AttributionRegion::new(0, "TestOperation", start, end)];

        let analysis = analyze_cost_attribution(&circuit, &regions, &V4CostModel::default());

        assert_eq!(analysis.regions.len(), 1);

        assert_eq!(analysis.regions[0].gate_count, 2);

        assert_eq!(analysis.regions[0].compute_gate_count, 2);

        assert_eq!(analysis.regions[0].operation_kind, "TestOperation");

        assert_eq!(output.0, 3);
    }

    #[test]
    fn attribution_reports_unattributed_inputs() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();
        let b = circuit.add_input();

        let start = circuit.gate_count();

        circuit.add_and(a, b);

        let end = circuit.gate_count();

        let regions = vec![AttributionRegion::new(0, "And", start, end)];

        let analysis = analyze_cost_attribution(&circuit, &regions, &V4CostModel::default());

        assert_eq!(analysis.unattributed_gate_count, 2);
    }

    #[test]
    fn attribution_identifies_dominant_region() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();
        let b = circuit.add_input();

        let first_start = circuit.gate_count();

        circuit.add_not(a);

        let first_end = circuit.gate_count();

        let second_start = circuit.gate_count();

        let x = circuit.add_and(a, b);
        circuit.add_and(x, b);

        let second_end = circuit.gate_count();

        let regions = vec![
            AttributionRegion::new(0, "Not", first_start, first_end),
            AttributionRegion::new(1, "AndChain", second_start, second_end),
        ];

        let analysis = analyze_cost_attribution(&circuit, &regions, &V4CostModel::default());

        assert_eq!(
            analysis.dominant_region().unwrap().operation_kind,
            "AndChain"
        );
    }
}
