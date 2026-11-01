use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use fhec_analyzer::hardware_profile::{
    HardwareFingerprint, HardwareProfileFile, HardwareProfileMetadata, HardwareProfileSource,
};
use fhec_analyzer::v4::{GateScalingProfiles, ScalingCurve};
use fhec_backend_tfhe::{
    TfheCalibration,
    profile::{TFHE_V4_PROFILE_BACKEND, V4ProfileLoad, load_v4_profile},
    v4::V4TfheCalibration,
};

fn fixture() -> V4TfheCalibration {
    let mut mixed_layer_scaling = ScalingCurve::new();
    mixed_layer_scaling.push(1, 20.0);
    mixed_layer_scaling.push(8, 70.0);
    let mut steady_mixed_layer_scaling = ScalingCurve::new();
    steady_mixed_layer_scaling.push(1, 19.0);
    steady_mixed_layer_scaling.push(8, 65.0);
    let mut narrow_deep_scaling = ScalingCurve::new();
    narrow_deep_scaling.push(1, 20.0);
    narrow_deep_scaling.push(4, 50.0);
    V4TfheCalibration {
        baseline: TfheCalibration {
            samples: 3,
            rayon_threads: 8,
            parallel_width: 16,
            not_ns: 10.0,
            and_ns: 20.0,
            or_ns: 21.0,
            xor_ns: 22.0,
            mux_ns: 40.0,
            parallel_batch_speedup: 4.0,
            parallel_efficiency: 0.5,
            layer_sync_ns: 5.0,
        },
        scaling: GateScalingProfiles::default(),
        mixed_layer_scaling,
        steady_mixed_layer_scaling,
        narrow_deep_scaling,
        mixed_reference_gate_ns: 20.75,
        calibration_widths: vec![1, 2, 4, 8, 16],
        narrow_widths: vec![1, 2, 3, 4],
        samples_per_width: 5,
        steady_repetitions: 3,
        narrow_depth_repetitions: 8,
        ciphertext_bytes: 3260,
        profile_time_ns: 1_000_000.0,
        calibration_relative_mad: 0.02,
    }
}

fn profile_path() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fhec-b1-profile-{}-{stamp}.json",
        std::process::id()
    ))
}

fn loaded(calibration: V4TfheCalibration) -> V4ProfileLoad {
    let path = profile_path();
    let metadata = HardwareProfileMetadata::new(
        TFHE_V4_PROFILE_BACKEND,
        HardwareFingerprint::current(),
        HardwareProfileSource::FreshCalibration,
    );
    HardwareProfileFile::new(metadata, calibration)
        .save_json(&path)
        .unwrap();
    let result = load_v4_profile(&path).unwrap();
    fs::remove_file(&path).unwrap();
    result
}

#[test]
fn rejects_unsorted_and_duplicate_gate_scaling_points() {
    let mut calibration = fixture();
    calibration.scaling.and.push(1, 10.0);
    calibration.scaling.and.push(4, 25.0);
    assert!(matches!(
        loaded(calibration.clone()),
        V4ProfileLoad::Loaded(_)
    ));

    calibration.scaling.and.points[0].width = 4;
    calibration.scaling.and.points[1].width = 1;
    assert!(matches!(
        loaded(calibration.clone()),
        V4ProfileLoad::Invalid
    ));

    calibration.scaling.and.points[1].width = 4;
    assert!(matches!(loaded(calibration), V4ProfileLoad::Invalid));
}

#[test]
fn rejects_unsorted_or_duplicate_sample_widths() {
    let mut calibration = fixture();
    calibration.calibration_widths = vec![1, 4, 2];
    assert!(matches!(
        loaded(calibration.clone()),
        V4ProfileLoad::Invalid
    ));

    calibration.calibration_widths = vec![1, 2, 2];
    assert!(matches!(
        loaded(calibration.clone()),
        V4ProfileLoad::Invalid
    ));

    calibration.calibration_widths = vec![1, 2, 4];
    calibration.narrow_widths = vec![1, 3, 3];
    assert!(matches!(loaded(calibration), V4ProfileLoad::Invalid));
}

#[test]
fn rejects_impossible_parallel_efficiency() {
    let mut calibration = fixture();
    calibration.baseline.parallel_efficiency = 1.2;
    assert!(matches!(loaded(calibration), V4ProfileLoad::Invalid));
}

#[test]
fn oversized_profile_recalibrates_instead_of_propagating_io_error() {
    let path = profile_path();
    fs::write(&path, vec![b' '; 1_048_577]).unwrap();
    let result = load_v4_profile(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(matches!(result, V4ProfileLoad::Invalid));
}

#[test]
fn rejects_impossible_calibration_parallel_width() {
    let mut calibration = fixture();
    calibration.baseline.parallel_width = usize::MAX;
    assert!(matches!(loaded(calibration), V4ProfileLoad::Invalid));
}

#[test]
fn rejects_calibration_widths_beyond_measured_parallel_width() {
    let mut calibration = fixture();
    calibration.calibration_widths.push(usize::MAX);
    assert!(matches!(loaded(calibration), V4ProfileLoad::Invalid));
}

#[test]
fn rejects_scaling_curves_with_impossible_widths() {
    let mut calibration = fixture();
    calibration.mixed_layer_scaling.points[1].width = usize::MAX;
    assert!(matches!(loaded(calibration), V4ProfileLoad::Invalid));
}

#[test]
fn rejects_narrow_widths_greater_than_four() {
    let mut calibration = fixture();
    calibration.narrow_widths.push(8);
    assert!(matches!(loaded(calibration), V4ProfileLoad::Invalid));
}

#[test]
fn persisted_profile_contains_measurements_not_secret_keys() {
    let path = profile_path();
    let calibration = fixture();
    fhec_backend_tfhe::profile::save_v4_profile(&path, &calibration).unwrap();
    let json = fs::read_to_string(&path).unwrap();
    for forbidden in ["client_key", "server_key", "secret_key", "plaintext_inputs"] {
        assert!(
            !json.contains(forbidden),
            "unexpected private key/input field"
        );
    }
    assert!(json.contains("parallel_width"));
    fs::remove_file(path).unwrap();
}
