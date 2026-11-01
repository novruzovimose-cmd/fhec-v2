use crate::crypto::CryptoCostAnalysis;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseBehavior {
    Unknown,
    GateBootstrapped,
    Leveled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseRisk {
    Unknown,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseProfile {
    pub behavior: NoiseBehavior,
    pub max_safe_unrefreshed_depth: usize,
}

impl NoiseProfile {
    pub fn gate_bootstrapped() -> Self {
        Self {
            behavior: NoiseBehavior::GateBootstrapped,
            max_safe_unrefreshed_depth: 0,
        }
    }

    pub fn leveled(max_safe_unrefreshed_depth: usize) -> Self {
        Self {
            behavior: NoiseBehavior::Leveled,
            max_safe_unrefreshed_depth,
        }
    }

    pub fn is_known(&self) -> bool {
        self.behavior != NoiseBehavior::Unknown
    }
}

impl Default for NoiseProfile {
    fn default() -> Self {
        Self {
            behavior: NoiseBehavior::Unknown,
            max_safe_unrefreshed_depth: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseAnalysis {
    pub behavior: NoiseBehavior,
    pub compute_operations: usize,
    pub refresh_operations: usize,
    pub refresh_density: f64,
    pub max_unrefreshed_depth: usize,
    pub risk: NoiseRisk,
}

impl NoiseAnalysis {
    pub fn refresh_percentage(&self) -> f64 {
        self.refresh_density * 100.0
    }

    pub fn has_refresh_activity(&self) -> bool {
        self.refresh_operations > 0
    }
}

pub fn analyze_noise(
    compute_operations: usize,
    circuit_depth: usize,
    crypto: &CryptoCostAnalysis,
    profile: &NoiseProfile,
) -> NoiseAnalysis {
    let refresh_operations = crypto
        .operations
        .refreshes
        .saturating_add(crypto.operations.bootstraps);

    let refresh_density = if compute_operations == 0 {
        0.0
    } else {
        refresh_operations as f64 / compute_operations as f64
    };

    match profile.behavior {
        NoiseBehavior::GateBootstrapped => analyze_gate_bootstrapped(
            compute_operations,
            circuit_depth,
            refresh_operations,
            refresh_density,
        ),

        NoiseBehavior::Leveled => analyze_leveled(
            compute_operations,
            circuit_depth,
            refresh_operations,
            refresh_density,
            profile.max_safe_unrefreshed_depth,
        ),

        NoiseBehavior::Unknown => NoiseAnalysis {
            behavior: NoiseBehavior::Unknown,
            compute_operations,
            refresh_operations,
            refresh_density,
            max_unrefreshed_depth: circuit_depth,
            risk: NoiseRisk::Unknown,
        },
    }
}

fn analyze_gate_bootstrapped(
    compute_operations: usize,
    circuit_depth: usize,
    refresh_operations: usize,
    refresh_density: f64,
) -> NoiseAnalysis {
    if compute_operations == 0 {
        return NoiseAnalysis {
            behavior: NoiseBehavior::GateBootstrapped,
            compute_operations,
            refresh_operations,
            refresh_density,
            max_unrefreshed_depth: 0,
            risk: NoiseRisk::Low,
        };
    }

    let fully_refreshed = refresh_operations >= compute_operations;

    let max_unrefreshed_depth = if fully_refreshed { 0 } else { circuit_depth };

    let risk = if fully_refreshed {
        NoiseRisk::Low
    } else if refresh_operations > 0 {
        NoiseRisk::Medium
    } else {
        NoiseRisk::High
    };

    NoiseAnalysis {
        behavior: NoiseBehavior::GateBootstrapped,
        compute_operations,
        refresh_operations,
        refresh_density,
        max_unrefreshed_depth,
        risk,
    }
}

fn analyze_leveled(
    compute_operations: usize,
    circuit_depth: usize,
    refresh_operations: usize,
    refresh_density: f64,
    max_safe_unrefreshed_depth: usize,
) -> NoiseAnalysis {
    let max_unrefreshed_depth = if refresh_operations == 0 {
        circuit_depth
    } else {
        estimate_unrefreshed_depth(circuit_depth, refresh_operations)
    };

    let risk = leveled_risk(max_unrefreshed_depth, max_safe_unrefreshed_depth);

    NoiseAnalysis {
        behavior: NoiseBehavior::Leveled,
        compute_operations,
        refresh_operations,
        refresh_density,
        max_unrefreshed_depth,
        risk,
    }
}

fn estimate_unrefreshed_depth(circuit_depth: usize, refresh_operations: usize) -> usize {
    if circuit_depth == 0 {
        return 0;
    }

    if refresh_operations == 0 {
        return circuit_depth;
    }

    let segments = refresh_operations.saturating_add(1);

    circuit_depth.saturating_add(segments - 1) / segments
}

fn leveled_risk(max_unrefreshed_depth: usize, max_safe_unrefreshed_depth: usize) -> NoiseRisk {
    if max_safe_unrefreshed_depth == 0 {
        return NoiseRisk::Unknown;
    }

    if max_unrefreshed_depth <= max_safe_unrefreshed_depth {
        return NoiseRisk::Low;
    }

    if max_unrefreshed_depth <= max_safe_unrefreshed_depth.saturating_mul(2) {
        return NoiseRisk::Medium;
    }

    NoiseRisk::High
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::crypto::{CryptoCostAnalysis, CryptoPrimitiveCounts, CryptoProfileSource};

    fn crypto_analysis(bootstraps: usize, refreshes: usize) -> CryptoCostAnalysis {
        CryptoCostAnalysis {
            operations: CryptoPrimitiveCounts {
                bootstraps,
                key_switches: 0,
                refreshes,
            },
            bootstrap_ns: 0.0,
            key_switch_ns: 0.0,
            refresh_ns: 0.0,
            total_crypto_ns: 0.0,
            dominant: None,
            source: CryptoProfileSource::BackendDeclared,
        }
    }

    #[test]
    fn gate_bootstrapped_profile_reports_low_risk() {
        let crypto = crypto_analysis(10, 0);

        let analysis = analyze_noise(10, 5, &crypto, &NoiseProfile::gate_bootstrapped());

        assert_eq!(analysis.behavior, NoiseBehavior::GateBootstrapped);

        assert_eq!(analysis.refresh_operations, 10);

        assert_eq!(analysis.refresh_density, 1.0);

        assert_eq!(analysis.max_unrefreshed_depth, 0);

        assert_eq!(analysis.risk, NoiseRisk::Low);
    }

    #[test]
    fn partial_gate_bootstrapping_reports_medium_risk() {
        let crypto = crypto_analysis(5, 0);

        let analysis = analyze_noise(10, 6, &crypto, &NoiseProfile::gate_bootstrapped());

        assert_eq!(analysis.refresh_operations, 5);

        assert_eq!(analysis.refresh_density, 0.5);

        assert_eq!(analysis.max_unrefreshed_depth, 6);

        assert_eq!(analysis.risk, NoiseRisk::Medium);
    }

    #[test]
    fn missing_gate_bootstrapping_reports_high_risk() {
        let crypto = crypto_analysis(0, 0);

        let analysis = analyze_noise(10, 8, &crypto, &NoiseProfile::gate_bootstrapped());

        assert_eq!(analysis.refresh_operations, 0);

        assert_eq!(analysis.max_unrefreshed_depth, 8);

        assert_eq!(analysis.risk, NoiseRisk::High);
    }

    #[test]
    fn leveled_profile_reports_low_risk_inside_budget() {
        let crypto = crypto_analysis(0, 0);

        let analysis = analyze_noise(12, 5, &crypto, &NoiseProfile::leveled(6));

        assert_eq!(analysis.behavior, NoiseBehavior::Leveled);

        assert_eq!(analysis.max_unrefreshed_depth, 5);

        assert_eq!(analysis.risk, NoiseRisk::Low);
    }

    #[test]
    fn leveled_profile_reports_medium_risk() {
        let crypto = crypto_analysis(0, 0);

        let analysis = analyze_noise(20, 9, &crypto, &NoiseProfile::leveled(5));

        assert_eq!(analysis.risk, NoiseRisk::Medium);
    }

    #[test]
    fn leveled_profile_reports_high_risk() {
        let crypto = crypto_analysis(0, 0);

        let analysis = analyze_noise(30, 15, &crypto, &NoiseProfile::leveled(5));

        assert_eq!(analysis.risk, NoiseRisk::High);
    }

    #[test]
    fn refreshes_reduce_estimated_unrefreshed_depth() {
        let crypto = crypto_analysis(0, 3);

        let analysis = analyze_noise(20, 12, &crypto, &NoiseProfile::leveled(4));

        assert_eq!(analysis.refresh_operations, 3);

        assert_eq!(analysis.max_unrefreshed_depth, 3);

        assert_eq!(analysis.risk, NoiseRisk::Low);
    }

    #[test]
    fn unknown_profile_reports_unknown_risk() {
        let crypto = crypto_analysis(0, 0);

        let analysis = analyze_noise(10, 7, &crypto, &NoiseProfile::default());

        assert_eq!(analysis.behavior, NoiseBehavior::Unknown);

        assert_eq!(analysis.risk, NoiseRisk::Unknown);

        assert_eq!(analysis.max_unrefreshed_depth, 7);
    }

    #[test]
    fn empty_bootstrapped_circuit_is_low_risk() {
        let crypto = crypto_analysis(0, 0);

        let analysis = analyze_noise(0, 0, &crypto, &NoiseProfile::gate_bootstrapped());

        assert_eq!(analysis.refresh_operations, 0);

        assert_eq!(analysis.refresh_density, 0.0);

        assert_eq!(analysis.max_unrefreshed_depth, 0);

        assert_eq!(analysis.risk, NoiseRisk::Low);
    }

    #[test]
    fn refresh_percentage_is_reported() {
        let crypto = crypto_analysis(5, 0);

        let analysis = analyze_noise(10, 4, &crypto, &NoiseProfile::gate_bootstrapped());

        assert_eq!(analysis.refresh_percentage(), 50.0);

        assert!(analysis.has_refresh_activity());
    }
}
