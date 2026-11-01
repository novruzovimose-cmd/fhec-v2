#[derive(Debug, Clone)]
pub struct CandidateTiming {
    pub name: String,
    pub predicted_ns: f64,
    pub measured_ns: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccuracyError {
    InsufficientCandidates,
    InvalidName(usize),
    DuplicateName(String),
    InvalidPrediction(usize),
    InvalidMeasurement(usize),
}

impl std::fmt::Display for AccuracyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AccuracyError {}

#[derive(Debug, Clone)]
pub struct CandidateAccuracy {
    pub name: String,
    pub predicted_ns: f64,
    pub measured_ns: f64,
    pub absolute_error_ns: f64,
    pub percentage_error: f64,
}

#[derive(Debug, Clone)]
pub struct AccuracyReport {
    pub candidates: Vec<CandidateAccuracy>,
    pub mean_percentage_error: f64,
    pub max_percentage_error: f64,
    pub predicted_winner: String,
    pub measured_winner: String,
    pub ranking_correct: bool,
    pub selection_penalty_percent: f64,
}

pub fn analyze_prediction_accuracy(
    timings: &[CandidateTiming],
) -> Result<AccuracyReport, AccuracyError> {
    if timings.len() < 2 {
        return Err(AccuracyError::InsufficientCandidates);
    }

    let mut names = std::collections::HashSet::new();
    let mut candidates = Vec::with_capacity(timings.len());

    for (index, timing) in timings.iter().enumerate() {
        if timing.name.trim().is_empty() {
            return Err(AccuracyError::InvalidName(index));
        }

        if !names.insert(timing.name.as_str()) {
            return Err(AccuracyError::DuplicateName(timing.name.clone()));
        }

        if !timing.predicted_ns.is_finite() || timing.predicted_ns <= 0.0 {
            return Err(AccuracyError::InvalidPrediction(index));
        }

        if !timing.measured_ns.is_finite() || timing.measured_ns <= 0.0 {
            return Err(AccuracyError::InvalidMeasurement(index));
        }

        let absolute_error_ns = (timing.predicted_ns - timing.measured_ns).abs();

        let percentage_error = 100.0 * (absolute_error_ns / timing.measured_ns);

        candidates.push(CandidateAccuracy {
            name: timing.name.clone(),
            predicted_ns: timing.predicted_ns,
            measured_ns: timing.measured_ns,
            absolute_error_ns,
            percentage_error,
        });
    }

    let predicted = timings
        .iter()
        .min_by(|a, b| a.predicted_ns.total_cmp(&b.predicted_ns))
        .expect("validated candidate list is nonempty");

    let measured = timings
        .iter()
        .min_by(|a, b| a.measured_ns.total_cmp(&b.measured_ns))
        .expect("validated candidate list is nonempty");

    let mean_percentage_error =
        candidates.iter().map(|c| c.percentage_error).sum::<f64>() / candidates.len() as f64;

    let max_percentage_error = candidates
        .iter()
        .map(|c| c.percentage_error)
        .fold(0.0_f64, f64::max);

    let best_measured_ns = measured.measured_ns;

    let tolerance_ns = best_measured_ns * 1e-9;

    let ranking_correct = predicted.measured_ns <= best_measured_ns + tolerance_ns;

    let selection_penalty_percent =
        100.0 * ((predicted.measured_ns / best_measured_ns) - 1.0).max(0.0);

    Ok(AccuracyReport {
        candidates,
        mean_percentage_error,
        max_percentage_error,
        predicted_winner: predicted.name.clone(),
        measured_winner: measured.name.clone(),
        ranking_correct,
        selection_penalty_percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(name: &str, predicted: f64, measured: f64) -> CandidateTiming {
        CandidateTiming {
            name: name.to_string(),
            predicted_ns: predicted,
            measured_ns: measured,
        }
    }

    #[test]
    fn perfect_predictions_have_zero_error() {
        let report = analyze_prediction_accuracy(&[
            timing("prefix", 100.0, 100.0),
            timing("ripple", 200.0, 200.0),
        ])
        .unwrap();

        assert_eq!(report.mean_percentage_error, 0.0);
        assert_eq!(report.max_percentage_error, 0.0);
        assert_eq!(report.selection_penalty_percent, 0.0);
        assert!(report.ranking_correct);
        assert_eq!(report.predicted_winner, "prefix");
        assert_eq!(report.measured_winner, "prefix");
    }

    #[test]
    fn detects_incorrect_candidate_ranking() {
        let report = analyze_prediction_accuracy(&[
            timing("prefix", 100.0, 300.0),
            timing("ripple", 200.0, 100.0),
        ])
        .unwrap();

        assert!(!report.ranking_correct);
        assert_eq!(report.predicted_winner, "prefix");
        assert_eq!(report.measured_winner, "ripple");
        assert!((report.selection_penalty_percent - 200.0).abs() < 1e-9);
    }

    #[test]
    fn computes_prediction_error_correctly() {
        let report =
            analyze_prediction_accuracy(&[timing("a", 110.0, 100.0), timing("b", 180.0, 200.0)])
                .unwrap();

        assert!((report.mean_percentage_error - 10.0).abs() < 1e-9);
        assert!((report.max_percentage_error - 10.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_invalid_benchmark_data() {
        assert_eq!(
            analyze_prediction_accuracy(&[]).unwrap_err(),
            AccuracyError::InsufficientCandidates
        );

        assert_eq!(
            analyze_prediction_accuracy(
                &[timing("a", f64::NAN, 100.0), timing("b", 200.0, 200.0),]
            )
            .unwrap_err(),
            AccuracyError::InvalidPrediction(0)
        );

        assert_eq!(
            analyze_prediction_accuracy(&[timing("a", 100.0, 0.0), timing("b", 200.0, 200.0),])
                .unwrap_err(),
            AccuracyError::InvalidMeasurement(0)
        );

        assert_eq!(
            analyze_prediction_accuracy(&[
                timing("same", 100.0, 100.0),
                timing("same", 200.0, 200.0),
            ])
            .unwrap_err(),
            AccuracyError::DuplicateName("same".to_string())
        );
    }
}
