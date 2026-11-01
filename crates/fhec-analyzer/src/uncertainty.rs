#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintySource {
    Unknown,
    BackendCalibration,
    RuntimeFeedback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictionUncertaintyProfile {
    pub relative_mad: f64,
    pub sample_count: usize,
    pub minimum_relative_margin: f64,
    pub source: UncertaintySource,
}

impl PredictionUncertaintyProfile {
    pub fn backend_calibrated(relative_mad: f64, sample_count: usize) -> Self {
        Self {
            relative_mad: relative_mad.max(0.0),
            sample_count,
            minimum_relative_margin: 0.05,
            source: UncertaintySource::BackendCalibration,
        }
    }

    pub fn runtime_feedback(relative_mad: f64, sample_count: usize) -> Self {
        Self {
            relative_mad: relative_mad.max(0.0),
            sample_count,
            minimum_relative_margin: 0.03,
            source: UncertaintySource::RuntimeFeedback,
        }
    }

    pub fn is_available(&self) -> bool {
        self.source != UncertaintySource::Unknown
    }
}

impl Default for PredictionUncertaintyProfile {
    fn default() -> Self {
        Self {
            relative_mad: 0.0,
            sample_count: 0,
            minimum_relative_margin: 0.25,
            source: UncertaintySource::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictionInterval {
    pub predicted_ns: f64,
    pub lower_ns: f64,
    pub upper_ns: f64,
    pub relative_margin: f64,
    pub calibration_variability: f64,
    pub sample_count: usize,
    pub confidence: IntervalConfidence,
    pub source: UncertaintySource,
}

impl PredictionInterval {
    pub fn predicted_milliseconds(&self) -> f64 {
        self.predicted_ns / 1_000_000.0
    }

    pub fn lower_milliseconds(&self) -> f64 {
        self.lower_ns / 1_000_000.0
    }

    pub fn upper_milliseconds(&self) -> f64 {
        self.upper_ns / 1_000_000.0
    }

    pub fn margin_percentage(&self) -> f64 {
        self.relative_margin * 100.0
    }

    pub fn variability_percentage(&self) -> f64 {
        self.calibration_variability * 100.0
    }

    pub fn contains(&self, actual_ns: f64) -> bool {
        actual_ns >= self.lower_ns && actual_ns <= self.upper_ns
    }
}

pub fn estimate_prediction_interval(
    predicted_ns: f64,
    profile: &PredictionUncertaintyProfile,
) -> PredictionInterval {
    let predicted_ns = predicted_ns.max(0.0);

    let robust_margin = profile.relative_mad * 2.5;

    let relative_margin = robust_margin
        .max(profile.minimum_relative_margin)
        .clamp(0.02, 1.0);

    let lower_ns = predicted_ns * (1.0 - relative_margin);

    let upper_ns = predicted_ns * (1.0 + relative_margin);

    let confidence = interval_confidence(profile);

    PredictionInterval {
        predicted_ns,
        lower_ns,
        upper_ns,
        relative_margin,
        calibration_variability: profile.relative_mad,
        sample_count: profile.sample_count,
        confidence,
        source: profile.source,
    }
}

fn interval_confidence(profile: &PredictionUncertaintyProfile) -> IntervalConfidence {
    if !profile.is_available() || profile.sample_count < 3 {
        return IntervalConfidence::Low;
    }

    if profile.relative_mad <= 0.08 && profile.sample_count >= 5 {
        return IntervalConfidence::High;
    }

    if profile.relative_mad <= 0.20 {
        return IntervalConfidence::Medium;
    }

    IntervalConfidence::Low
}

pub fn relative_mad(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let median_value = median(values);

    if median_value <= 0.0 {
        return 0.0;
    }

    let deviations = values
        .iter()
        .map(|value| (value - median_value).abs())
        .collect::<Vec<_>>();

    median(&deviations) / median_value
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();

    sorted.sort_by(|left, right| left.total_cmp(right));

    let middle = sorted.len() / 2;

    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_mad_is_zero_for_stable_samples() {
        let values = [100.0, 100.0, 100.0];

        assert_eq!(relative_mad(&values), 0.0);
    }

    #[test]
    fn relative_mad_detects_variability() {
        let values = [90.0, 100.0, 110.0, 120.0, 80.0];

        let value = relative_mad(&values);

        assert!(value > 0.0);
        assert!(value < 0.2);
    }

    #[test]
    fn calibrated_interval_contains_prediction() {
        let profile = PredictionUncertaintyProfile::backend_calibrated(0.05, 5);

        let interval = estimate_prediction_interval(100_000_000.0, &profile);

        assert!(interval.lower_ns < interval.predicted_ns);

        assert!(interval.upper_ns > interval.predicted_ns);
    }

    #[test]
    fn stable_calibration_has_high_confidence() {
        let profile = PredictionUncertaintyProfile::backend_calibrated(0.03, 7);

        let interval = estimate_prediction_interval(100.0, &profile);

        assert_eq!(interval.confidence, IntervalConfidence::High);
    }

    #[test]
    fn unknown_profile_is_low_confidence() {
        let interval =
            estimate_prediction_interval(100.0, &PredictionUncertaintyProfile::default());

        assert_eq!(interval.confidence, IntervalConfidence::Low);
    }

    #[test]
    fn interval_reports_milliseconds() {
        let profile = PredictionUncertaintyProfile::backend_calibrated(0.04, 5);

        let interval = estimate_prediction_interval(200_000_000.0, &profile);

        assert_eq!(interval.predicted_milliseconds(), 200.0);

        assert!(interval.lower_milliseconds() < 200.0);

        assert!(interval.upper_milliseconds() > 200.0);
    }
}
