#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementQuality {
    pub samples: usize,
    pub median_ms: f64,
    pub minimum_ms: f64,
    pub maximum_ms: f64,
    pub cv_percent: f64,
    pub high_variability: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementError {
    EmptySamples,
    InvalidSample(usize),
}

impl std::fmt::Display for MeasurementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MeasurementError {}

pub fn summarize_measurements(samples: &[f64]) -> Result<MeasurementQuality, MeasurementError> {
    if samples.is_empty() {
        return Err(MeasurementError::EmptySamples);
    }

    for (index, &value) in samples.iter().enumerate() {
        if !value.is_finite() || value <= 0.0 {
            return Err(MeasurementError::InvalidSample(index));
        }
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);

    let count = sorted.len();
    let median_ms = if count % 2 == 0 {
        sorted[count / 2 - 1] / 2.0 + sorted[count / 2] / 2.0
    } else {
        sorted[count / 2]
    };

    let minimum_ms = sorted[0];
    let maximum_ms = sorted[count - 1];

    let scale = maximum_ms;
    let normalized_mean = sorted.iter().map(|x| x / scale).sum::<f64>() / count as f64;

    let normalized_variance = sorted
        .iter()
        .map(|x| {
            let difference = x / scale - normalized_mean;
            difference * difference
        })
        .sum::<f64>()
        / count as f64;

    let cv_percent = normalized_variance.sqrt() / normalized_mean * 100.0;

    Ok(MeasurementQuality {
        samples: count,
        median_ms,
        minimum_ms,
        maximum_ms,
        cv_percent,
        high_variability: count >= 3 && cv_percent > 10.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_measurements_have_zero_variability() {
        let result = summarize_measurements(&[10.0, 10.0, 10.0]).unwrap();

        assert_eq!(result.samples, 3);
        assert_eq!(result.median_ms, 10.0);
        assert_eq!(result.cv_percent, 0.0);
        assert!(!result.high_variability);
    }

    #[test]
    fn calculates_even_sample_median() {
        let result = summarize_measurements(&[40.0, 10.0, 30.0, 20.0]).unwrap();

        assert_eq!(result.median_ms, 25.0);
        assert_eq!(result.minimum_ms, 10.0);
        assert_eq!(result.maximum_ms, 40.0);
    }

    #[test]
    fn detects_high_variability() {
        let result = summarize_measurements(&[10.0, 20.0, 30.0]).unwrap();

        assert!(result.high_variability);
        assert!(result.cv_percent > 10.0);
    }

    #[test]
    fn rejects_invalid_samples() {
        assert_eq!(
            summarize_measurements(&[]),
            Err(MeasurementError::EmptySamples)
        );

        assert_eq!(
            summarize_measurements(&[1.0, f64::NAN]),
            Err(MeasurementError::InvalidSample(1))
        );

        assert_eq!(
            summarize_measurements(&[1.0, 0.0]),
            Err(MeasurementError::InvalidSample(1))
        );
    }
}
