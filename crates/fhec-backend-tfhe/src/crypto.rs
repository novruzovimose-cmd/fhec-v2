use fhec_analyzer::noise::NoiseProfile;

use fhec_analyzer::crypto::{CryptoCostProfile, CryptoPrimitiveCounts, GateCryptoProfile};

pub fn boolean_crypto_profile() -> CryptoCostProfile {
    let bootstrapped_gate = CryptoPrimitiveCounts {
        bootstraps: 1,
        key_switches: 1,
        refreshes: 0,
    };

    let mux = CryptoPrimitiveCounts {
        bootstraps: 2,
        key_switches: 2,
        refreshes: 0,
    };

    CryptoCostProfile::backend_declared(
        GateCryptoProfile {
            not: CryptoPrimitiveCounts::default(),
            and: bootstrapped_gate,
            or: bootstrapped_gate,
            xor: bootstrapped_gate,
            mux,
        },
        0.0,
        0.0,
        0.0,
    )
}

pub fn boolean_noise_profile() -> NoiseProfile {
    NoiseProfile::gate_bootstrapped()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_profile_declares_crypto_operations() {
        let profile = boolean_crypto_profile();

        assert!(profile.is_available());

        assert_eq!(profile.gates.not.bootstraps, 0);

        assert_eq!(profile.gates.and.bootstraps, 1);
        assert_eq!(profile.gates.and.key_switches, 1);

        assert_eq!(profile.gates.or.bootstraps, 1);
        assert_eq!(profile.gates.or.key_switches, 1);

        assert_eq!(profile.gates.xor.bootstraps, 1);
        assert_eq!(profile.gates.xor.key_switches, 1);

        assert_eq!(profile.gates.mux.bootstraps, 2);
        assert_eq!(profile.gates.mux.key_switches, 2);
    }
}
