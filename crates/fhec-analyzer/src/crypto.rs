use crate::GateCounts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoProfileSource {
    Unknown,
    BackendDeclared,
    BackendCalibrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoPrimitiveKind {
    Bootstrap,
    KeySwitch,
    Refresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CryptoPrimitiveCounts {
    pub bootstraps: usize,
    pub key_switches: usize,
    pub refreshes: usize,
}

impl CryptoPrimitiveCounts {
    pub fn total(self) -> usize {
        self.bootstraps + self.key_switches + self.refreshes
    }

    pub fn scaled(self, count: usize) -> Self {
        Self {
            bootstraps: self.bootstraps.saturating_mul(count),
            key_switches: self.key_switches.saturating_mul(count),
            refreshes: self.refreshes.saturating_mul(count),
        }
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self {
            bootstraps: self.bootstraps.saturating_add(other.bootstraps),
            key_switches: self.key_switches.saturating_add(other.key_switches),
            refreshes: self.refreshes.saturating_add(other.refreshes),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GateCryptoProfile {
    pub not: CryptoPrimitiveCounts,
    pub and: CryptoPrimitiveCounts,
    pub or: CryptoPrimitiveCounts,
    pub xor: CryptoPrimitiveCounts,
    pub mux: CryptoPrimitiveCounts,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CryptoCostProfile {
    pub gates: GateCryptoProfile,
    pub bootstrap_ns: f64,
    pub key_switch_ns: f64,
    pub refresh_ns: f64,
    pub source: CryptoProfileSource,
}

impl CryptoCostProfile {
    pub fn backend_declared(
        gates: GateCryptoProfile,
        bootstrap_ns: f64,
        key_switch_ns: f64,
        refresh_ns: f64,
    ) -> Self {
        Self {
            gates,
            bootstrap_ns: bootstrap_ns.max(0.0),
            key_switch_ns: key_switch_ns.max(0.0),
            refresh_ns: refresh_ns.max(0.0),
            source: CryptoProfileSource::BackendDeclared,
        }
    }

    pub fn backend_calibrated(
        gates: GateCryptoProfile,
        bootstrap_ns: f64,
        key_switch_ns: f64,
        refresh_ns: f64,
    ) -> Self {
        Self {
            gates,
            bootstrap_ns: bootstrap_ns.max(0.0),
            key_switch_ns: key_switch_ns.max(0.0),
            refresh_ns: refresh_ns.max(0.0),
            source: CryptoProfileSource::BackendCalibrated,
        }
    }

    pub fn is_available(&self) -> bool {
        self.source != CryptoProfileSource::Unknown
    }

    pub fn is_calibrated(&self) -> bool {
        self.source == CryptoProfileSource::BackendCalibrated
    }
}

impl Default for CryptoCostProfile {
    fn default() -> Self {
        Self {
            gates: GateCryptoProfile::default(),
            bootstrap_ns: 0.0,
            key_switch_ns: 0.0,
            refresh_ns: 0.0,
            source: CryptoProfileSource::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CryptoCostAnalysis {
    pub operations: CryptoPrimitiveCounts,
    pub bootstrap_ns: f64,
    pub key_switch_ns: f64,
    pub refresh_ns: f64,
    pub total_crypto_ns: f64,
    pub dominant: Option<CryptoPrimitiveKind>,
    pub source: CryptoProfileSource,
}

impl CryptoCostAnalysis {
    pub fn bootstrap_milliseconds(&self) -> f64 {
        self.bootstrap_ns / 1_000_000.0
    }

    pub fn key_switch_milliseconds(&self) -> f64 {
        self.key_switch_ns / 1_000_000.0
    }

    pub fn refresh_milliseconds(&self) -> f64 {
        self.refresh_ns / 1_000_000.0
    }

    pub fn total_crypto_milliseconds(&self) -> f64 {
        self.total_crypto_ns / 1_000_000.0
    }

    pub fn has_crypto_work(&self) -> bool {
        self.operations.total() > 0
    }
}

pub fn analyze_crypto_cost(counts: &GateCounts, profile: &CryptoCostProfile) -> CryptoCostAnalysis {
    let operations = profile
        .gates
        .not
        .scaled(counts.not)
        .saturating_add(profile.gates.and.scaled(counts.and))
        .saturating_add(profile.gates.or.scaled(counts.or))
        .saturating_add(profile.gates.xor.scaled(counts.xor))
        .saturating_add(profile.gates.mux.scaled(counts.mux));

    let bootstrap_ns = operations.bootstraps as f64 * profile.bootstrap_ns;

    let key_switch_ns = operations.key_switches as f64 * profile.key_switch_ns;

    let refresh_ns = operations.refreshes as f64 * profile.refresh_ns;

    let total_crypto_ns = bootstrap_ns + key_switch_ns + refresh_ns;

    let dominant = dominant_primitive(bootstrap_ns, key_switch_ns, refresh_ns);

    CryptoCostAnalysis {
        operations,
        bootstrap_ns,
        key_switch_ns,
        refresh_ns,
        total_crypto_ns,
        dominant,
        source: profile.source,
    }
}

fn dominant_primitive(
    bootstrap_ns: f64,
    key_switch_ns: f64,
    refresh_ns: f64,
) -> Option<CryptoPrimitiveKind> {
    if bootstrap_ns <= 0.0 && key_switch_ns <= 0.0 && refresh_ns <= 0.0 {
        return None;
    }

    if bootstrap_ns >= key_switch_ns && bootstrap_ns >= refresh_ns {
        return Some(CryptoPrimitiveKind::Bootstrap);
    }

    if key_switch_ns >= bootstrap_ns && key_switch_ns >= refresh_ns {
        return Some(CryptoPrimitiveKind::KeySwitch);
    }

    Some(CryptoPrimitiveKind::Refresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_counts_scale_and_add() {
        let first = CryptoPrimitiveCounts {
            bootstraps: 1,
            key_switches: 2,
            refreshes: 0,
        }
        .scaled(3);

        let second = CryptoPrimitiveCounts {
            bootstraps: 2,
            key_switches: 0,
            refreshes: 1,
        };

        let combined = first.saturating_add(second);

        assert_eq!(combined.bootstraps, 5);
        assert_eq!(combined.key_switches, 6);
        assert_eq!(combined.refreshes, 1);
        assert_eq!(combined.total(), 12);
    }

    #[test]
    fn crypto_analysis_counts_backend_operations() {
        let counts = GateCounts {
            not: 1,
            and: 2,
            or: 1,
            xor: 3,
            mux: 1,
            ..GateCounts::default()
        };

        let profile = CryptoCostProfile::backend_declared(
            GateCryptoProfile {
                not: CryptoPrimitiveCounts {
                    bootstraps: 0,
                    key_switches: 0,
                    refreshes: 0,
                },
                and: CryptoPrimitiveCounts {
                    bootstraps: 1,
                    key_switches: 1,
                    refreshes: 0,
                },
                or: CryptoPrimitiveCounts {
                    bootstraps: 1,
                    key_switches: 1,
                    refreshes: 0,
                },
                xor: CryptoPrimitiveCounts {
                    bootstraps: 1,
                    key_switches: 1,
                    refreshes: 0,
                },
                mux: CryptoPrimitiveCounts {
                    bootstraps: 2,
                    key_switches: 2,
                    refreshes: 0,
                },
            },
            10.0,
            5.0,
            0.0,
        );

        let analysis = analyze_crypto_cost(&counts, &profile);

        assert_eq!(analysis.operations.bootstraps, 8);
        assert_eq!(analysis.operations.key_switches, 8);
        assert_eq!(analysis.operations.refreshes, 0);

        assert_eq!(analysis.bootstrap_ns, 80.0);
        assert_eq!(analysis.key_switch_ns, 40.0);
        assert_eq!(analysis.total_crypto_ns, 120.0);

        assert_eq!(analysis.dominant, Some(CryptoPrimitiveKind::Bootstrap));
    }

    #[test]
    fn unknown_profile_reports_no_crypto_work() {
        let counts = GateCounts {
            and: 10,
            xor: 5,
            ..GateCounts::default()
        };

        let analysis = analyze_crypto_cost(&counts, &CryptoCostProfile::default());

        assert!(!analysis.has_crypto_work());

        assert_eq!(analysis.source, CryptoProfileSource::Unknown);

        assert_eq!(analysis.dominant, None);
    }
}
