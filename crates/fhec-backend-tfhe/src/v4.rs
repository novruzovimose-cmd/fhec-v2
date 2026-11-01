use fhec_analyzer::uncertainty::{PredictionUncertaintyProfile, relative_mad};
use fhec_analyzer::v4::{
    GateKind, GateScalingProfiles, HardwareProfile, ScalingCurve, V4CostModel, V4GateLatencies,
};
use serde::{Deserialize, Serialize};
use std::{hint::black_box, time::Instant};

use fhec_backend::FheBackend;

use rayon::prelude::*;

use tfhe::boolean::prelude::Ciphertext;

use crate::{TfheBackend, TfheCalibration, TfheCalibrationError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V4TfheCalibration {
    pub baseline: TfheCalibration,

    pub scaling: GateScalingProfiles,

    pub mixed_layer_scaling: ScalingCurve,

    pub steady_mixed_layer_scaling: ScalingCurve,

    pub narrow_deep_scaling: ScalingCurve,

    pub mixed_reference_gate_ns: f64,

    pub calibration_widths: Vec<usize>,

    pub narrow_widths: Vec<usize>,

    pub samples_per_width: usize,

    pub steady_repetitions: usize,

    pub narrow_depth_repetitions: usize,

    pub ciphertext_bytes: usize,

    pub profile_time_ns: f64,

    pub calibration_relative_mad: f64,
}

impl V4TfheCalibration {
    pub fn cost_model(&self) -> V4CostModel {
        V4CostModel::calibrated(
            V4GateLatencies {
                not_ns: self.baseline.not_ns,
                and_ns: self.baseline.and_ns,
                or_ns: self.baseline.or_ns,
                xor_ns: self.baseline.xor_ns,
                mux_ns: self.baseline.mux_ns,
            },
            self.baseline.layer_sync_ns,
            self.baseline.parallel_efficiency,
            HardwareProfile {
                worker_threads: self.baseline.rayon_threads,

                calibration_width: self.baseline.parallel_width,
            },
            self.scaling,
        )
        .with_mixed_layer_scaling(self.mixed_layer_scaling, self.mixed_reference_gate_ns)
        .with_steady_mixed_layer_scaling(self.steady_mixed_layer_scaling)
        .with_narrow_deep_scaling(self.narrow_deep_scaling, 4)
        .with_ciphertext_bytes(self.ciphertext_bytes)
        .with_uncertainty_profile(PredictionUncertaintyProfile::backend_calibrated(
            self.calibration_relative_mad,
            self.samples_per_width.max(5),
        ))
        .with_crypto_profile(crate::crypto::boolean_crypto_profile())
        .with_noise_profile(crate::crypto::boolean_noise_profile())
    }
}

impl TfheBackend {
    pub fn calibrate_v4(&self, samples: usize) -> Result<V4TfheCalibration, TfheCalibrationError> {
        let baseline = self.calibrate(samples)?;

        let profile_start = Instant::now();

        let a = self.encrypt_bool(true)?;

        let b = self.encrypt_bool(false)?;

        let c = self.encrypt_bool(true)?;

        let ciphertext_bytes = bincode::serialized_size(&a)
            .map_err(TfheCalibrationError::CiphertextSerialization)?
            as usize;

        let mut widths = vec![1usize, 2, 4, 8, 16, 32]
            .into_iter()
            .filter(|width| *width <= baseline.parallel_width)
            .collect::<Vec<_>>();

        if !widths.contains(&baseline.parallel_width) {
            widths.push(baseline.parallel_width);
        }

        widths.sort_unstable();
        widths.dedup();

        let narrow_widths = (1usize..=4)
            .filter(|width| *width <= baseline.parallel_width)
            .collect::<Vec<_>>();

        let samples_per_width = samples.clamp(3, 7);

        let steady_repetitions = 3usize;

        let narrow_depth_repetitions = 8usize;

        let mut scaling = GateScalingProfiles::default();

        let mut mixed_layer_scaling = ScalingCurve::new();

        let mut steady_mixed_layer_scaling = ScalingCurve::new();

        let mut narrow_deep_scaling = ScalingCurve::new();

        for width in &widths {
            let not_ns = measure_parallel_median(*width, samples_per_width, || self.not(&a))?;

            scaling.curve_mut(GateKind::Not).push(*width, not_ns);

            let and_ns = measure_parallel_median(*width, samples_per_width, || self.and(&a, &b))?;

            scaling.curve_mut(GateKind::And).push(*width, and_ns);

            let or_ns = measure_parallel_median(*width, samples_per_width, || self.or(&a, &b))?;

            scaling.curve_mut(GateKind::Or).push(*width, or_ns);

            let xor_ns = measure_parallel_median(*width, samples_per_width, || self.xor(&a, &b))?;

            scaling.curve_mut(GateKind::Xor).push(*width, xor_ns);

            let mux_ns =
                measure_parallel_median(*width, samples_per_width, || self.mux(&a, &b, &c))?;

            scaling.curve_mut(GateKind::Mux).push(*width, mux_ns);

            let mixed_ns = measure_mixed_parallel_median(self, *width, samples_per_width, &a, &b)?;

            mixed_layer_scaling.push(*width, mixed_ns);

            let steady_ns = measure_mixed_steady_median(
                self,
                *width,
                samples_per_width,
                steady_repetitions,
                &a,
                &b,
            )?;

            steady_mixed_layer_scaling.push(*width, steady_ns);
        }

        for width in &narrow_widths {
            let narrow_ns = measure_narrow_deep_median(
                self,
                *width,
                samples_per_width,
                narrow_depth_repetitions,
                &a,
                &b,
            )?;

            narrow_deep_scaling.push(*width, narrow_ns);
        }

        let variability_width = baseline.parallel_width.clamp(1, 8);

        let variability_samples = samples_per_width.max(5);

        let mut variability_timings = Vec::with_capacity(variability_samples);

        let _ = measure_mixed_parallel_once(self, variability_width, &a, &b)?;

        for _ in 0..variability_samples {
            variability_timings.push(measure_mixed_parallel_once(
                self,
                variability_width,
                &a,
                &b,
            )?);
        }

        let calibration_relative_mad = relative_mad(&variability_timings);

        let mixed_reference_gate_ns =
            (baseline.and_ns * 2.0 + baseline.or_ns + baseline.xor_ns) / 4.0;

        let profile_time_ns = profile_start.elapsed().as_nanos() as f64;

        Ok(V4TfheCalibration {
            baseline,
            scaling,
            mixed_layer_scaling,
            steady_mixed_layer_scaling,
            narrow_deep_scaling,
            mixed_reference_gate_ns,
            calibration_widths: widths,
            narrow_widths,
            samples_per_width,
            steady_repetitions,
            narrow_depth_repetitions,
            ciphertext_bytes,
            calibration_relative_mad,
            profile_time_ns,
        })
    }

    pub fn calibrate_v4_cost_model(
        &self,
        samples: usize,
    ) -> Result<(V4TfheCalibration, V4CostModel), TfheCalibrationError> {
        let calibration = self.calibrate_v4(samples)?;

        let model = calibration.cost_model();

        Ok((calibration, model))
    }
}

fn measure_parallel_median<F>(
    width: usize,
    samples: usize,
    operation: F,
) -> Result<f64, TfheCalibrationError>
where
    F: Fn() -> Result<Ciphertext, ()> + Sync,
{
    let width = width.max(1);

    let samples = samples.max(1);

    let _ = measure_parallel_once(width, &operation)?;

    let mut timings = Vec::with_capacity(samples);

    for _ in 0..samples {
        timings.push(measure_parallel_once(width, &operation)?);
    }

    Ok(median(&mut timings))
}

fn measure_parallel_once<F>(width: usize, operation: &F) -> Result<f64, TfheCalibrationError>
where
    F: Fn() -> Result<Ciphertext, ()> + Sync,
{
    let start = Instant::now();

    let results = (0..width)
        .into_par_iter()
        .map(|_| operation())
        .collect::<Result<Vec<_>, ()>>()?;

    black_box(results);

    Ok(start.elapsed().as_nanos() as f64)
}

fn measure_mixed_parallel_median(
    backend: &TfheBackend,
    width: usize,
    samples: usize,
    a: &Ciphertext,
    b: &Ciphertext,
) -> Result<f64, TfheCalibrationError> {
    let width = width.max(1);

    let samples = samples.max(1);

    let _ = measure_mixed_parallel_once(backend, width, a, b)?;

    let mut timings = Vec::with_capacity(samples);

    for _ in 0..samples {
        timings.push(measure_mixed_parallel_once(backend, width, a, b)?);
    }

    Ok(median(&mut timings))
}

fn measure_mixed_parallel_once(
    backend: &TfheBackend,
    width: usize,
    a: &Ciphertext,
    b: &Ciphertext,
) -> Result<f64, TfheCalibrationError> {
    let start = Instant::now();

    let results = (0..width)
        .into_par_iter()
        .map(|index| match index % 4 {
            0 | 1 => backend.and(a, b),

            2 => backend.or(a, b),

            _ => backend.xor(a, b),
        })
        .collect::<Result<Vec<_>, ()>>()?;

    black_box(results);

    Ok(start.elapsed().as_nanos() as f64)
}

fn measure_mixed_steady_median(
    backend: &TfheBackend,
    width: usize,
    samples: usize,
    repetitions: usize,
    a: &Ciphertext,
    b: &Ciphertext,
) -> Result<f64, TfheCalibrationError> {
    let width = width.max(1);

    let samples = samples.max(1);

    let repetitions = repetitions.max(1);

    let _ = measure_mixed_steady_once(backend, width, repetitions, a, b)?;

    let mut timings = Vec::with_capacity(samples);

    for _ in 0..samples {
        timings.push(measure_mixed_steady_once(
            backend,
            width,
            repetitions,
            a,
            b,
        )?);
    }

    Ok(median(&mut timings))
}

fn measure_mixed_steady_once(
    backend: &TfheBackend,
    width: usize,
    repetitions: usize,
    a: &Ciphertext,
    b: &Ciphertext,
) -> Result<f64, TfheCalibrationError> {
    let start = Instant::now();

    for _ in 0..repetitions {
        let results = (0..width)
            .into_par_iter()
            .map(|index| match index % 4 {
                0 | 1 => backend.and(a, b),

                2 => backend.or(a, b),

                _ => backend.xor(a, b),
            })
            .collect::<Result<Vec<_>, ()>>()?;

        black_box(results);
    }

    let total_ns = start.elapsed().as_nanos() as f64;

    Ok(total_ns / repetitions as f64)
}

fn measure_narrow_deep_median(
    backend: &TfheBackend,
    width: usize,
    samples: usize,
    repetitions: usize,
    a: &Ciphertext,
    b: &Ciphertext,
) -> Result<f64, TfheCalibrationError> {
    let width = width.max(1);

    let samples = samples.max(1);

    let repetitions = repetitions.max(1);

    let _ = measure_narrow_deep_once(backend, width, repetitions, a, b)?;

    let mut timings = Vec::with_capacity(samples);

    for _ in 0..samples {
        timings.push(measure_narrow_deep_once(backend, width, repetitions, a, b)?);
    }

    Ok(median(&mut timings))
}

fn measure_narrow_deep_once(
    backend: &TfheBackend,
    width: usize,
    repetitions: usize,
    a: &Ciphertext,
    b: &Ciphertext,
) -> Result<f64, TfheCalibrationError> {
    let mut current = vec![a.clone(); width];

    let start = Instant::now();

    for round in 0..repetitions {
        current = current
            .into_par_iter()
            .enumerate()
            .map(|(index, value)| match (round + index) % 4 {
                0 | 1 => backend.and(&value, b),

                2 => backend.or(&value, b),

                _ => backend.xor(&value, b),
            })
            .collect::<Result<Vec<_>, ()>>()?;

        black_box(&current);
    }

    let total_ns = start.elapsed().as_nanos() as f64;

    Ok(total_ns / repetitions as f64)
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.sort_by(|left, right| left.total_cmp(right));

    let middle = values.len() / 2;

    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_handles_odd_samples() {
        let mut values = [30.0, 10.0, 20.0];

        assert_eq!(median(&mut values,), 20.0);
    }

    #[test]
    fn median_handles_even_samples() {
        let mut values = [40.0, 10.0, 30.0, 20.0];

        assert_eq!(median(&mut values,), 25.0);
    }

    #[test]
    #[ignore]
    fn v4_calibration_builds_profiles() {
        let backend = TfheBackend::new();

        let calibration = backend.calibrate_v4(2).unwrap();

        assert!(calibration.scaling.is_complete());

        assert!(!calibration.mixed_layer_scaling.is_empty());

        assert!(!calibration.steady_mixed_layer_scaling.is_empty());

        assert!(!calibration.narrow_deep_scaling.is_empty());

        assert_eq!(calibration.narrow_widths, vec![1, 2, 3, 4,]);

        assert!(calibration.mixed_reference_gate_ns > 0.0);

        assert!(calibration.ciphertext_bytes > 0);

        assert!(calibration.samples_per_width >= 3);
        assert!(calibration.calibration_relative_mad >= 0.0);
        assert!(calibration.calibration_relative_mad < 1.0);

        let model = calibration.cost_model();

        assert!(model.uncertainty_profile.is_available());

        assert!(model.has_empirical_scaling());

        assert!(model.has_mixed_layer_scaling());

        assert!(model.has_steady_mixed_layer_scaling());

        assert!(model.has_narrow_deep_scaling());
        assert!(model.ciphertext_bytes > 0);
    }
}
