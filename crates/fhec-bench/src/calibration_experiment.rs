use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone)]
pub struct Observation {
    pub workload: String,
    pub implementation: String,
    pub predicted_ns: f64,
    pub measured_ns: f64,
}

#[derive(Debug, Clone)]
pub struct Correction {
    pub implementation: String,
    pub factor: f64,
    pub observations: usize,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub workload: String,
    pub baseline_error_percent: f64,
    pub corrected_error_percent: f64,
    pub baseline_penalty_percent: f64,
    pub corrected_penalty_percent: f64,
}

#[derive(Debug, Clone)]
pub struct ExperimentReport {
    pub folds: Vec<ValidationResult>,
    pub mean_baseline_error_percent: f64,
    pub mean_corrected_error_percent: f64,
    pub adoption_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExperimentError {
    InsufficientWorkloads,
    InvalidObservation(usize),
    DuplicateObservation,
    MissingImplementation(String),
}

impl std::fmt::Display for ExperimentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ExperimentError {}

fn validate(observations: &[Observation]) -> Result<(), ExperimentError> {
    let mut seen = HashSet::new();

    for (index, observation) in observations.iter().enumerate() {
        if observation.workload.trim().is_empty()
            || observation.implementation.trim().is_empty()
            || !observation.predicted_ns.is_finite()
            || observation.predicted_ns <= 0.0
            || !observation.measured_ns.is_finite()
            || observation.measured_ns <= 0.0
        {
            return Err(ExperimentError::InvalidObservation(index));
        }

        if !seen.insert((
            observation.workload.as_str(),
            observation.implementation.as_str(),
        )) {
            return Err(ExperimentError::DuplicateObservation);
        }
    }

    Ok(())
}

pub fn fit_corrections(training: &[Observation]) -> Result<Vec<Correction>, ExperimentError> {
    validate(training)?;

    let mut groups: BTreeMap<&str, Vec<f64>> = BTreeMap::new();

    for observation in training {
        groups
            .entry(&observation.implementation)
            .or_default()
            .push(observation.measured_ns / observation.predicted_ns);
    }

    let mut corrections = Vec::new();

    for (implementation, ratios) in groups {
        let mut ratios = ratios;
        ratios.sort_by(f64::total_cmp);

        let middle = ratios.len() / 2;

        let median = if ratios.len() % 2 == 0 {
            ratios[middle - 1] / 2.0 + ratios[middle] / 2.0
        } else {
            ratios[middle]
        };

        corrections.push(Correction {
            implementation: implementation.to_string(),
            factor: median.clamp(0.5, 2.5),
            observations: ratios.len(),
        });
    }

    Ok(corrections)
}

fn selection_penalty(
    observations: &[&Observation],
    corrections: &BTreeMap<&str, f64>,
) -> Result<(f64, f64), ExperimentError> {
    let best_actual = observations
        .iter()
        .min_by(|a, b| a.measured_ns.total_cmp(&b.measured_ns))
        .expect("nonempty validation workload");

    let best_baseline = observations
        .iter()
        .min_by(|a, b| a.predicted_ns.total_cmp(&b.predicted_ns))
        .expect("nonempty validation workload");

    let mut best_corrected: Option<(&Observation, f64)> = None;

    for observation in observations {
        let factor = corrections
            .get(observation.implementation.as_str())
            .ok_or_else(|| {
                ExperimentError::MissingImplementation(observation.implementation.clone())
            })?;

        let predicted = observation.predicted_ns * factor;

        if best_corrected
            .as_ref()
            .is_none_or(|(_, best)| predicted < *best)
        {
            best_corrected = Some((observation, predicted));
        }
    }

    let best_corrected = best_corrected.expect("nonempty validation workload").0;

    let baseline_penalty =
        ((best_baseline.measured_ns / best_actual.measured_ns) - 1.0).max(0.0) * 100.0;

    let corrected_penalty =
        ((best_corrected.measured_ns / best_actual.measured_ns) - 1.0).max(0.0) * 100.0;

    Ok((baseline_penalty, corrected_penalty))
}

pub fn cross_validate(observations: &[Observation]) -> Result<ExperimentReport, ExperimentError> {
    validate(observations)?;

    let workloads = observations
        .iter()
        .map(|observation| observation.workload.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    if workloads.len() < 2 {
        return Err(ExperimentError::InsufficientWorkloads);
    }

    let mut folds = Vec::new();

    for workload in workloads {
        let training = observations
            .iter()
            .filter(|observation| observation.workload != workload)
            .cloned()
            .collect::<Vec<_>>();

        let validation = observations
            .iter()
            .filter(|observation| observation.workload == workload)
            .collect::<Vec<_>>();

        let corrections = fit_corrections(&training)?
            .into_iter()
            .map(|correction| (correction.implementation, correction.factor))
            .collect::<BTreeMap<_, _>>();

        let lookup = corrections
            .iter()
            .map(|(name, factor)| (name.as_str(), *factor))
            .collect::<BTreeMap<_, _>>();

        let mut baseline_error = 0.0;
        let mut corrected_error = 0.0;

        for observation in &validation {
            let factor = lookup
                .get(observation.implementation.as_str())
                .ok_or_else(|| {
                    ExperimentError::MissingImplementation(observation.implementation.clone())
                })?;

            baseline_error +=
                ((observation.predicted_ns / observation.measured_ns) - 1.0).abs() * 100.0;

            corrected_error +=
                (((observation.predicted_ns * factor) / observation.measured_ns) - 1.0).abs()
                    * 100.0;
        }

        let (baseline_penalty, corrected_penalty) = selection_penalty(&validation, &lookup)?;

        folds.push(ValidationResult {
            workload: workload.to_string(),
            baseline_error_percent: baseline_error / validation.len() as f64,
            corrected_error_percent: corrected_error / validation.len() as f64,
            baseline_penalty_percent: baseline_penalty,
            corrected_penalty_percent: corrected_penalty,
        });
    }

    let count = folds.len() as f64;

    let mean_baseline_error_percent = folds
        .iter()
        .map(|fold| fold.baseline_error_percent)
        .sum::<f64>()
        / count;

    let mean_corrected_error_percent = folds
        .iter()
        .map(|fold| fold.corrected_error_percent)
        .sum::<f64>()
        / count;

    let adoption_supported = folds.len() >= 5
        && mean_corrected_error_percent < mean_baseline_error_percent
        && folds
            .iter()
            .all(|fold| fold.corrected_error_percent <= fold.baseline_error_percent + 1e-9)
        && folds
            .iter()
            .all(|fold| fold.corrected_penalty_percent <= fold.baseline_penalty_percent + 1e-9);

    Ok(ExperimentReport {
        folds,
        mean_baseline_error_percent,
        mean_corrected_error_percent,
        adoption_supported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        workload: &str,
        implementation: &str,
        predicted: f64,
        measured: f64,
    ) -> Observation {
        Observation {
            workload: workload.into(),
            implementation: implementation.into(),
            predicted_ns: predicted,
            measured_ns: measured,
        }
    }

    #[test]
    fn learns_bounded_corrections() {
        let corrections = fit_corrections(&[
            observation("a", "prefix", 100.0, 200.0),
            observation("b", "prefix", 100.0, 400.0),
        ])
        .unwrap();

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].factor, 2.5);
    }

    #[test]
    fn detects_improved_held_out_predictions() {
        let mut samples = Vec::new();

        for index in 0..5 {
            let workload = format!("program_{index}");

            samples.push(observation(&workload, "prefix", 100.0, 150.0));
            samples.push(observation(&workload, "ripple", 200.0, 300.0));
        }

        let report = cross_validate(&samples).unwrap();

        assert!(report.adoption_supported);
        assert!(report.mean_corrected_error_percent < 1e-9);
        assert_eq!(report.folds.len(), 5);
    }

    #[test]
    fn rejects_insufficient_validation_evidence() {
        let report = cross_validate(&[
            observation("a", "prefix", 100.0, 150.0),
            observation("a", "ripple", 200.0, 300.0),
            observation("b", "prefix", 100.0, 150.0),
            observation("b", "ripple", 200.0, 300.0),
        ])
        .unwrap();

        assert!(!report.adoption_supported);
    }

    #[test]
    fn rejects_correction_that_regresses_one_workload() {
        let mut samples = Vec::new();

        for index in 0..5 {
            let workload = format!("workload_{index}");
            let factor = if index == 4 { 1.0 } else { 1.5 };

            samples.push(observation(&workload, "prefix", 100.0, 100.0 * factor));

            samples.push(observation(&workload, "ripple", 200.0, 200.0 * factor));
        }

        let report = cross_validate(&samples).unwrap();

        assert!(report.mean_corrected_error_percent < report.mean_baseline_error_percent);

        assert!(
            report
                .folds
                .iter()
                .any(|fold| { fold.corrected_error_percent > fold.baseline_error_percent })
        );

        assert!(!report.adoption_supported);
    }
    #[test]
    fn rejects_duplicate_observations() {
        let result = cross_validate(&[
            observation("a", "prefix", 100.0, 150.0),
            observation("a", "prefix", 100.0, 150.0),
            observation("b", "prefix", 100.0, 150.0),
        ]);

        assert!(matches!(result, Err(ExperimentError::DuplicateObservation)));
    }
}
