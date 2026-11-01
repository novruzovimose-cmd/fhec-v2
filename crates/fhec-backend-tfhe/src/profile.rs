use std::{error::Error, fmt, path::Path};

use fhec_analyzer::{
    hardware_profile::{
        HardwareFingerprint, HardwareProfileFile, HardwareProfileIoError, HardwareProfileMetadata,
        HardwareProfileSource,
    },
    v4::V4CostModel,
};

use crate::{TfheBackend, TfheCalibrationError, v4::V4TfheCalibration};

pub const TFHE_V4_PROFILE_BACKEND: &str = "tfhe-boolean-v4";

#[derive(Debug, Clone, PartialEq)]
pub enum V4ProfileLoad {
    Loaded(Box<V4TfheCalibration>),
    Missing,
    Incompatible,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum V4ProfileOrigin {
    PersistentProfile,
    FreshCalibration,
}

#[derive(Debug, Clone)]
pub struct V4ResolvedProfile {
    pub calibration: V4TfheCalibration,
    pub model: V4CostModel,
    pub origin: V4ProfileOrigin,
}

impl V4ResolvedProfile {
    pub fn calibration_reused(&self) -> bool {
        self.origin == V4ProfileOrigin::PersistentProfile
    }

    pub fn status_name(&self) -> &'static str {
        match self.origin {
            V4ProfileOrigin::PersistentProfile => "Loaded",
            V4ProfileOrigin::FreshCalibration => "Calibrated",
        }
    }

    pub fn render_report(&self) -> String {
        format!(
            "FHEC HARDWARE PROFILE\n\
             =====================\n\
             Backend: {}\n\
             Status: {}\n\
             Origin: {:?}\n\
             Calibration reused: {}\n\
             Rayon threads: {}\n\
             Calibration width: {}\n\
             Ciphertext size: {} bytes\n\
             Calibration variability: {:.2}%\n",
            TFHE_V4_PROFILE_BACKEND,
            self.status_name(),
            self.origin,
            if self.calibration_reused() {
                "YES"
            } else {
                "NO"
            },
            self.calibration.baseline.rayon_threads,
            self.calibration.baseline.parallel_width,
            self.calibration.ciphertext_bytes,
            self.calibration.calibration_relative_mad * 100.0,
        )
    }
}

#[derive(Debug)]
pub enum V4ProfileResolveError {
    Io(HardwareProfileIoError),
    CalibrationFailed(TfheCalibrationError),
}

impl fmt::Display for V4ProfileResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::CalibrationFailed(error) => {
                write!(formatter, "TFHE V4 calibration failed: {error}")
            }
        }
    }
}

impl Error for V4ProfileResolveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::CalibrationFailed(error) => Some(error),
        }
    }
}

impl From<HardwareProfileIoError> for V4ProfileResolveError {
    fn from(value: HardwareProfileIoError) -> Self {
        Self::Io(value)
    }
}

pub fn save_v4_profile(
    path: impl AsRef<Path>,
    calibration: &V4TfheCalibration,
) -> Result<(), HardwareProfileIoError> {
    let metadata = HardwareProfileMetadata::new(
        TFHE_V4_PROFILE_BACKEND,
        HardwareFingerprint::current(),
        HardwareProfileSource::FreshCalibration,
    );

    let profile = HardwareProfileFile::new(metadata, calibration.clone());

    profile.save_json(path)
}

pub fn load_v4_profile(path: impl AsRef<Path>) -> Result<V4ProfileLoad, HardwareProfileIoError> {
    let profile = match HardwareProfileFile::<V4TfheCalibration>::load_json(path) {
        Ok(profile) => profile,
        Err(HardwareProfileIoError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(V4ProfileLoad::Missing);
        }
        Err(error) => {
            let invalid_json = matches!(&error, HardwareProfileIoError::Json(_));
            let oversized = matches!(
                &error,
                HardwareProfileIoError::Io(io_error)
                    if io_error.kind() == std::io::ErrorKind::InvalidData
            );

            if invalid_json || oversized {
                return Ok(V4ProfileLoad::Invalid);
            }
            return Err(error);
        }
    };

    let current_fingerprint = HardwareFingerprint::current();

    if !profile
        .metadata
        .is_compatible(TFHE_V4_PROFILE_BACKEND, &current_fingerprint)
    {
        return Ok(V4ProfileLoad::Incompatible);
    }

    if !valid_calibration(&profile.payload) {
        return Ok(V4ProfileLoad::Invalid);
    }

    Ok(V4ProfileLoad::Loaded(Box::new(profile.payload)))
}

fn valid_scaling_curve(curve: &fhec_analyzer::v4::ScalingCurve, max_width: usize) -> bool {
    if curve.len > fhec_analyzer::v4::MAX_SCALING_POINTS {
        return false;
    }

    let points = &curve.points[..curve.len];

    points.iter().all(|point| {
        point.width > 0
            && point.width <= max_width
            && point.parallel_ns.is_finite()
            && point.parallel_ns > 0.0
    }) && points.windows(2).all(|pair| pair[0].width < pair[1].width)
}

fn valid_calibration(calibration: &V4TfheCalibration) -> bool {
    let baseline = &calibration.baseline;
    let timings = [
        baseline.not_ns,
        baseline.and_ns,
        baseline.or_ns,
        baseline.xor_ns,
        baseline.mux_ns,
        baseline.parallel_batch_speedup,
        baseline.parallel_efficiency,
        calibration.mixed_reference_gate_ns,
    ];
    let max_calibrated_width = baseline.parallel_width;
    let gate_curves_valid = fhec_analyzer::v4::GateKind::ALL
        .iter()
        .all(|kind| valid_scaling_curve(calibration.scaling.curve(*kind), max_calibrated_width));

    baseline.samples > 0
        && baseline.rayon_threads > 0
        && baseline.rayon_threads <= 4096
        && baseline.parallel_width == baseline.rayon_threads.saturating_mul(2).clamp(4, 32)
        && baseline.parallel_efficiency <= 1.0
        && calibration.samples_per_width > 0
        && calibration.steady_repetitions > 0
        && calibration.narrow_depth_repetitions > 0
        && calibration.ciphertext_bytes > 0
        && calibration.ciphertext_bytes <= 1_048_576
        && !calibration.calibration_widths.is_empty()
        && !calibration.narrow_widths.is_empty()
        && calibration
            .calibration_widths
            .iter()
            .all(|width| *width > 0 && *width <= baseline.parallel_width)
        && calibration
            .narrow_widths
            .iter()
            .all(|width| *width > 0 && *width <= 4)
        && calibration
            .calibration_widths
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        && calibration
            .narrow_widths
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        && timings
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
        && baseline.layer_sync_ns.is_finite()
        && baseline.layer_sync_ns >= 0.0
        && calibration.profile_time_ns.is_finite()
        && calibration.profile_time_ns >= 0.0
        && calibration.calibration_relative_mad.is_finite()
        && calibration.calibration_relative_mad >= 0.0
        && gate_curves_valid
        && valid_scaling_curve(&calibration.mixed_layer_scaling, max_calibrated_width)
        && valid_scaling_curve(
            &calibration.steady_mixed_layer_scaling,
            max_calibrated_width,
        )
        && valid_scaling_curve(&calibration.narrow_deep_scaling, 4)
}

impl TfheBackend {
    pub fn load_or_calibrate_v4_profile(
        &self,
        path: impl AsRef<Path>,
        samples: usize,
    ) -> Result<V4ResolvedProfile, V4ProfileResolveError> {
        let path = path.as_ref();

        match load_v4_profile(path)? {
            V4ProfileLoad::Loaded(calibration) => {
                let model = calibration.cost_model();

                Ok(V4ResolvedProfile {
                    calibration: *calibration,
                    model,
                    origin: V4ProfileOrigin::PersistentProfile,
                })
            }

            V4ProfileLoad::Missing | V4ProfileLoad::Incompatible | V4ProfileLoad::Invalid => {
                let calibration = self
                    .calibrate_v4(samples)
                    .map_err(V4ProfileResolveError::CalibrationFailed)?;

                save_v4_profile(path, &calibration)?;

                let model = calibration.cost_model();

                Ok(V4ResolvedProfile {
                    calibration,
                    model,
                    origin: V4ProfileOrigin::FreshCalibration,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use fhec_analyzer::v4::{GateScalingProfiles, ScalingCurve};

    use crate::{TfheBackend, TfheCalibration, v4::V4TfheCalibration};

    use super::{V4ProfileLoad, V4ProfileOrigin, load_v4_profile, save_v4_profile};

    fn test_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!(
            "fhec-tfhe-{name}-{}-{unique}.json",
            std::process::id()
        ))
    }

    fn test_calibration() -> V4TfheCalibration {
        let baseline = TfheCalibration {
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
        };

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
            baseline,
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

    #[test]
    fn missing_profile_is_reported() {
        let path = test_path("missing");

        let result = load_v4_profile(path).unwrap();

        assert!(matches!(result, V4ProfileLoad::Missing));
    }

    #[test]
    fn invalid_profile_is_reported() {
        let path = test_path("invalid");

        fs::write(&path, "{ invalid json").unwrap();

        let result = load_v4_profile(&path).unwrap();

        assert!(matches!(result, V4ProfileLoad::Invalid));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn loaded_profile_variant_is_compact() {
        assert!(std::mem::size_of::<V4ProfileLoad>() <= 32);
    }

    #[test]
    fn calibration_round_trips_through_profile() {
        let path = test_path("round-trip");

        let calibration = test_calibration();

        save_v4_profile(&path, &calibration).unwrap();

        let loaded = load_v4_profile(&path).unwrap();

        match loaded {
            V4ProfileLoad::Loaded(loaded_calibration) => {
                assert_eq!(*loaded_calibration, calibration);
            }
            other => {
                panic!("expected loaded profile, got {other:?}");
            }
        }

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn resolved_profile_report_describes_persistent_profile() {
        let calibration = test_calibration();
        let model = calibration.cost_model();

        let resolved = super::V4ResolvedProfile {
            calibration,
            model,
            origin: V4ProfileOrigin::PersistentProfile,
        };

        let report = resolved.render_report();

        assert!(report.contains("FHEC HARDWARE PROFILE"));
        assert!(report.contains("Backend: tfhe-boolean-v4"));
        assert!(report.contains("Status: Loaded"));
        assert!(report.contains("Origin: PersistentProfile"));
        assert!(report.contains("Calibration reused: YES"));
        assert!(report.contains("Ciphertext size: 3260 bytes"));
        assert!(report.contains("Calibration variability: 2.00%"));
    }
    #[test]
    #[ignore]
    fn real_profile_is_saved_then_reused() {
        let path = test_path("real-persistence");

        if path.exists() {
            fs::remove_file(&path).unwrap();
        }

        let backend = TfheBackend::new();

        let first = backend.load_or_calibrate_v4_profile(&path, 2).unwrap();

        assert_eq!(first.origin, V4ProfileOrigin::FreshCalibration);

        assert!(path.exists());

        assert!(first.model.uncertainty_profile.is_available());

        assert!(first.model.ciphertext_bytes > 0);

        println!();
        println!("FIRST RUN");
        println!("=========");
        println!("{}", first.render_report());

        let second = backend.load_or_calibrate_v4_profile(&path, 2).unwrap();

        assert_eq!(second.origin, V4ProfileOrigin::PersistentProfile);
        assert_eq!(
            second.calibration.baseline.samples,
            first.calibration.baseline.samples
        );

        assert_eq!(
            second.calibration.baseline.rayon_threads,
            first.calibration.baseline.rayon_threads
        );

        assert_eq!(
            second.calibration.baseline.parallel_width,
            first.calibration.baseline.parallel_width
        );

        assert_eq!(
            second.calibration.calibration_widths,
            first.calibration.calibration_widths
        );

        assert_eq!(
            second.calibration.narrow_widths,
            first.calibration.narrow_widths
        );

        assert_eq!(
            second.calibration.ciphertext_bytes,
            first.calibration.ciphertext_bytes
        );

        assert!(
            (second.calibration.baseline.parallel_efficiency
                - first.calibration.baseline.parallel_efficiency)
                .abs()
                < 1e-12
        );

        assert!(
            (second.calibration.calibration_relative_mad
                - first.calibration.calibration_relative_mad)
                .abs()
                < 1e-12
        );

        assert!(second.model.uncertainty_profile.is_available());

        assert_eq!(second.model.ciphertext_bytes, first.model.ciphertext_bytes);

        println!();
        println!("SECOND RUN");
        println!("==========");
        println!("{}", second.render_report());

        fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejects_zero_gate_latency_in_persisted_profile() {
        let path = test_path("zero-latency");
        let mut calibration = test_calibration();
        calibration.baseline.and_ns = 0.0;
        save_v4_profile(&path, &calibration).unwrap();
        let loaded = load_v4_profile(&path).unwrap();
        assert!(matches!(loaded, V4ProfileLoad::Invalid));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_invalid_scaling_length_in_persisted_profile() {
        let path = test_path("scaling-length");
        let mut calibration = test_calibration();
        calibration.mixed_layer_scaling.len = fhec_analyzer::v4::MAX_SCALING_POINTS + 1;
        save_v4_profile(&path, &calibration).unwrap();
        let loaded = load_v4_profile(&path).unwrap();
        assert!(matches!(loaded, V4ProfileLoad::Invalid));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_zero_ciphertext_bytes_in_persisted_profile() {
        let path = test_path("ciphertext-size");
        let mut calibration = test_calibration();
        calibration.ciphertext_bytes = 0;
        save_v4_profile(&path, &calibration).unwrap();
        let loaded = load_v4_profile(&path).unwrap();
        assert!(matches!(loaded, V4ProfileLoad::Invalid));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_outdated_profile_version() {
        let path = test_path("old-version");
        save_v4_profile(&path, &test_calibration()).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        let modified = contents.replacen("\"format_version\": 1", "\"format_version\": 0", 1);
        assert_ne!(contents, modified);
        fs::write(&path, modified).unwrap();
        let loaded = load_v4_profile(&path).unwrap();
        assert!(matches!(loaded, V4ProfileLoad::Incompatible));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn load_or_calibrate_uses_existing_profile() {
        let path = test_path("load-existing");

        let calibration = test_calibration();

        save_v4_profile(&path, &calibration).unwrap();

        let backend = TfheBackend::new();

        let resolved = backend.load_or_calibrate_v4_profile(&path, 2).unwrap();

        assert_eq!(resolved.origin, V4ProfileOrigin::PersistentProfile);

        assert_eq!(resolved.calibration, calibration);

        assert!(resolved.model.uncertainty_profile.is_available());

        assert_eq!(resolved.model.ciphertext_bytes, 3260);

        fs::remove_file(path).unwrap();
    }
}
