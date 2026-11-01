pub mod crypto;
pub mod profile;
pub mod v4;

use std::{error::Error, fmt, hint::black_box, time::Instant};

use serde::{Deserialize, Serialize};

use fhec_analyzer::CostModel;
use fhec_backend::FheBackend;
use rayon::prelude::*;
use tfhe::boolean::prelude::*;

#[derive(Debug)]
pub enum TfheCalibrationError {
    BackendOperation,
    CiphertextSerialization(bincode::Error),
}

impl fmt::Display for TfheCalibrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendOperation => write!(formatter, "TFHE Boolean gate operation failed"),
            Self::CiphertextSerialization(error) => {
                write!(formatter, "TFHE ciphertext serialization failed: {error}")
            }
        }
    }
}

impl Error for TfheCalibrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CiphertextSerialization(error) => Some(error.as_ref()),
            Self::BackendOperation => None,
        }
    }
}

impl From<()> for TfheCalibrationError {
    fn from(_: ()) -> Self {
        Self::BackendOperation
    }
}

pub struct TfheBackend {
    client_key: ClientKey,
    server_key: ServerKey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TfheCalibration {
    pub samples: usize,
    pub rayon_threads: usize,
    pub parallel_width: usize,
    pub not_ns: f64,
    pub and_ns: f64,
    pub or_ns: f64,
    pub xor_ns: f64,
    pub mux_ns: f64,
    pub parallel_batch_speedup: f64,
    pub parallel_efficiency: f64,
    pub layer_sync_ns: f64,
}

impl TfheCalibration {
    pub fn cost_model(&self) -> CostModel {
        CostModel::calibrated_nanoseconds(
            self.not_ns,
            self.and_ns,
            self.or_ns,
            self.xor_ns,
            self.mux_ns,
            self.parallel_efficiency,
            self.layer_sync_ns,
        )
    }
}

impl TfheBackend {
    pub fn new() -> Self {
        let (client_key, server_key) = gen_keys();

        Self {
            client_key,
            server_key,
        }
    }

    pub fn calibrate(&self, samples: usize) -> Result<TfheCalibration, TfheCalibrationError> {
        let samples = samples.max(2);

        let a = self.encrypt_bool(true)?;

        let b = self.encrypt_bool(false)?;

        let c = self.encrypt_bool(true)?;

        black_box(self.not(&a)?);

        black_box(self.and(&a, &b)?);

        black_box(self.or(&a, &b)?);

        black_box(self.xor(&a, &b)?);

        black_box(self.mux(&a, &b, &c)?);

        let not_start = Instant::now();

        for _ in 0..samples {
            black_box(self.not(&a)?);
        }

        let not_ns = not_start.elapsed().as_nanos() as f64 / samples as f64;

        let and_start = Instant::now();

        for _ in 0..samples {
            black_box(self.and(&a, &b)?);
        }

        let and_ns = and_start.elapsed().as_nanos() as f64 / samples as f64;

        let or_start = Instant::now();

        for _ in 0..samples {
            black_box(self.or(&a, &b)?);
        }

        let or_ns = or_start.elapsed().as_nanos() as f64 / samples as f64;

        let xor_start = Instant::now();

        for _ in 0..samples {
            black_box(self.xor(&a, &b)?);
        }

        let xor_ns = xor_start.elapsed().as_nanos() as f64 / samples as f64;

        let mux_start = Instant::now();

        for _ in 0..samples {
            black_box(self.mux(&a, &b, &c)?);
        }

        let mux_ns = mux_start.elapsed().as_nanos() as f64 / samples as f64;

        let rayon_threads = rayon::current_num_threads().max(1);

        let parallel_width = (rayon_threads * 2).clamp(4, 32);

        let batch_repeats = samples.clamp(2, 4);

        let pairs = (0..parallel_width)
            .map(|index| {
                if index % 2 == 0 {
                    (a.clone(), b.clone())
                } else {
                    (c.clone(), a.clone())
                }
            })
            .collect::<Vec<_>>();

        let sequential_batch_start = Instant::now();

        for _ in 0..batch_repeats {
            for (lhs, rhs) in &pairs {
                black_box(self.and(lhs, rhs)?);
            }
        }

        let sequential_batch_ns = sequential_batch_start.elapsed().as_nanos() as f64;

        let parallel_batch_start = Instant::now();

        for _ in 0..batch_repeats {
            let results = pairs
                .par_iter()
                .map(|(lhs, rhs)| self.and(lhs, rhs))
                .collect::<Result<Vec<_>, ()>>()?;

            black_box(results);
        }

        let parallel_batch_ns = parallel_batch_start.elapsed().as_nanos() as f64;

        let parallel_batch_speedup = if parallel_batch_ns > 0.0 {
            sequential_batch_ns / parallel_batch_ns
        } else {
            1.0
        };

        let ideal_parallelism = parallel_width.min(rayon_threads).max(1) as f64;

        let parallel_efficiency = (parallel_batch_speedup / ideal_parallelism).clamp(0.05, 1.0);

        let observed_layer_ns = parallel_batch_ns / batch_repeats as f64;

        let expected_parallel_ns =
            (and_ns * parallel_width as f64) / (ideal_parallelism * parallel_efficiency);

        let layer_sync_ns = (observed_layer_ns - expected_parallel_ns).max(0.0);

        Ok(TfheCalibration {
            samples,
            rayon_threads,
            parallel_width,
            not_ns,
            and_ns,
            or_ns,
            xor_ns,
            mux_ns,
            parallel_batch_speedup,
            parallel_efficiency,
            layer_sync_ns,
        })
    }

    pub fn calibrate_cost_model(
        &self,
        samples: usize,
    ) -> Result<(TfheCalibration, CostModel), TfheCalibrationError> {
        let calibration = self.calibrate(samples)?;

        let model = calibration.cost_model();

        Ok((calibration, model))
    }
}

impl Default for TfheBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl FheBackend for TfheBackend {
    type Ciphertext = Ciphertext;

    type Error = ();

    fn encrypt_bool(&self, value: bool) -> Result<Self::Ciphertext, Self::Error> {
        Ok(self.client_key.encrypt(value))
    }

    fn decrypt_bool(&self, value: &Self::Ciphertext) -> Result<bool, Self::Error> {
        Ok(self.client_key.decrypt(value))
    }

    fn not(&self, value: &Self::Ciphertext) -> Result<Self::Ciphertext, Self::Error> {
        Ok(self.server_key.not(value))
    }

    fn and(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(self.server_key.and(lhs, rhs))
    }

    fn or(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(self.server_key.or(lhs, rhs))
    }

    fn xor(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(self.server_key.xor(lhs, rhs))
    }

    fn mux(
        &self,
        condition: &Self::Ciphertext,
        if_true: &Self::Ciphertext,
        if_false: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(self.server_key.mux(condition, if_true, if_false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tfhe_backend_executes_encrypted_boolean_operations() {
        let backend = TfheBackend::new();

        let a = backend.encrypt_bool(true).unwrap();

        let b = backend.encrypt_bool(false).unwrap();

        let and_result = backend.and(&a, &b).unwrap();

        let or_result = backend.or(&a, &b).unwrap();

        let xor_result = backend.xor(&a, &b).unwrap();

        let not_result = backend.not(&a).unwrap();

        let mux_result = backend.mux(&a, &a, &b).unwrap();

        assert!(!backend.decrypt_bool(&and_result,).unwrap());

        assert!(backend.decrypt_bool(&or_result,).unwrap());

        assert!(backend.decrypt_bool(&xor_result,).unwrap());

        assert!(!backend.decrypt_bool(&not_result,).unwrap());

        assert!(backend.decrypt_bool(&mux_result,).unwrap());
    }

    #[test]
    fn tfhe_backend_produces_calibrated_cost_model() {
        let backend = TfheBackend::new();

        let (calibration, model) = backend.calibrate_cost_model(2).unwrap();

        assert!(calibration.and_ns > 0.0);

        assert!(calibration.or_ns > 0.0);

        assert!(calibration.xor_ns > 0.0);

        assert!(calibration.mux_ns > 0.0);

        assert!(calibration.parallel_efficiency > 0.0);

        assert!(model.is_calibrated());
    }
}
