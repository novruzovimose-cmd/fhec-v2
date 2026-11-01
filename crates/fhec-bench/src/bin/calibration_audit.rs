use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use fhec_bench::calibration_experiment::{Observation, cross_validate};

const HEADER: &str = "program,implementation,iteration,measured_ms,predicted_ms";

#[derive(Debug)]
struct Sample {
    predicted_ms: f64,
    measured_ms: f64,
}

type Key = (String, String);
type Samples = BTreeMap<Key, Vec<Sample>>;

fn read_session(path: &Path) -> Result<Samples, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();

    let header = lines
        .next()
        .ok_or("empty benchmark CSV")?
        .trim_start_matches('\u{feff}')
        .trim();

    if header != HEADER {
        return Err(format!("{}: unsupported CSV header", path.display()).into());
    }

    let mut groups: Samples = BTreeMap::new();
    let mut seen = HashSet::new();

    for (index, line) in lines.enumerate() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let fields = line.split(',').collect::<Vec<_>>();

        if fields.len() != 5 {
            return Err(format!(
                "{}:{}: expected five CSV columns",
                path.display(),
                index + 2
            )
            .into());
        }

        let program = fields[0].trim();
        let implementation = fields[1].trim();

        if program.is_empty() || implementation.is_empty() {
            return Err("empty workload or implementation".into());
        }

        let iteration: usize = fields[2].trim().parse()?;
        let measured_ms: f64 = fields[3].trim().parse()?;
        let predicted_ms: f64 = fields[4].trim().parse()?;

        if iteration == 0
            || !measured_ms.is_finite()
            || measured_ms <= 0.0
            || !predicted_ms.is_finite()
            || predicted_ms <= 0.0
        {
            return Err(
                format!("{}:{}: invalid benchmark values", path.display(), index + 2).into(),
            );
        }

        let key = (program.to_string(), implementation.to_string());

        if !seen.insert((key.clone(), iteration)) {
            return Err(format!(
                "{}:{}: duplicate sample iteration",
                path.display(),
                index + 2
            )
            .into());
        }

        groups.entry(key).or_default().push(Sample {
            measured_ms,
            predicted_ms,
        });
    }

    if groups.is_empty() {
        return Err("benchmark CSV contains no samples".into());
    }

    Ok(groups)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);

    let middle = values.len() / 2;

    if values.len() % 2 == 0 {
        values[middle - 1] / 2.0 + values[middle] / 2.0
    } else {
        values[middle]
    }
}

fn aggregate(first: Samples, second: Samples) -> Result<Vec<Observation>, Box<dyn Error>> {
    let first_keys = first.keys().collect::<BTreeSet<_>>();
    let second_keys = second.keys().collect::<BTreeSet<_>>();

    if first_keys != second_keys {
        return Err("benchmark sessions have different candidate sets".into());
    }

    let mut observations = Vec::new();

    for (key, first_samples) in first {
        let second_samples = second.get(&key).ok_or("missing second-session candidate")?;

        let mut measurements = Vec::new();
        let mut predictions = Vec::new();

        for sample in first_samples.iter().chain(second_samples.iter()) {
            measurements.push(sample.measured_ms);
            predictions.push(sample.predicted_ms);
        }

        if measurements.len() < 4 {
            return Err(format!("insufficient measurements for {} / {}", key.0, key.1).into());
        }

        let reference_prediction = predictions[0];

        for prediction in &predictions {
            let difference = (prediction - reference_prediction).abs();
            let tolerance = reference_prediction * 1e-9;

            if difference > tolerance {
                return Err(format!("unstable prediction for {} / {}", key.0, key.1).into());
            }
        }

        let measured_ms = median(&mut measurements);

        observations.push(Observation {
            workload: key.0,
            implementation: key.1,
            predicted_ns: reference_prediction * 1_000_000.0,
            measured_ns: measured_ms * 1_000_000.0,
        });
    }

    Ok(observations)
}

fn workload_family(workload: &str) -> &'static str {
    if workload == "addition_u8" || workload.starts_with("nested_addition_") {
        "addition"
    } else if workload == "subtraction_u8" || workload.starts_with("nested_subtraction_") {
        "subtraction"
    } else if workload.starts_with("multiplication_") {
        "multiplication"
    } else if workload.starts_with("greater_than_") || workload.starts_with("equality_") {
        "comparison"
    } else {
        "unclassified"
    }
}

fn print_family_report(folds: &[fhec_bench::calibration_experiment::ValidationResult]) {
    let mut families: BTreeMap<
        &'static str,
        Vec<&fhec_bench::calibration_experiment::ValidationResult>,
    > = BTreeMap::new();

    for fold in folds {
        families
            .entry(workload_family(&fold.workload))
            .or_default()
            .push(fold);
    }

    println!("\nOPERATION-FAMILY ANALYSIS");

    for (family, members) in families {
        let count = members.len();
        let baseline = members
            .iter()
            .map(|fold| fold.baseline_error_percent)
            .sum::<f64>()
            / count as f64;

        let corrected = members
            .iter()
            .map(|fold| fold.corrected_error_percent)
            .sum::<f64>()
            / count as f64;

        let regressions = members
            .iter()
            .filter(|fold| fold.corrected_error_percent > fold.baseline_error_percent + 1e-9)
            .count();

        println!(
            "{family}: workloads={count}, baseline={baseline:.2}%, \
             corrected={corrected:.2}%, regressions={regressions}"
        );

        if count < 3 {
            println!(
                "  WARNING: fewer than 3 distinct workloads; \
                 insufficient family-level validation coverage"
            );
        }

        if family == "unclassified" {
            println!(
                "  WARNING: explicitly classify these workloads \
                 before interpreting family-level results"
            );
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();

    if args.len() != 3 {
        return Err("Usage: calibration_audit <session-a.csv> <session-b.csv>".into());
    }

    let first = read_session(Path::new(&args[1]))?;
    let second = read_session(Path::new(&args[2]))?;

    let observations = aggregate(first, second)?;

    let report = cross_validate(&observations)?;

    println!("\nFHEC CALIBRATION AUDIT");
    println!("======================");
    println!("Observations: {}", observations.len());
    println!("Validation workloads: {}", report.folds.len());

    println!("\nCROSS-VALIDATION");

    for fold in &report.folds {
        println!(
            "{}: baseline={:.2}% corrected={:.2}% \
             baseline_penalty={:.2}% corrected_penalty={:.2}%",
            fold.workload,
            fold.baseline_error_percent,
            fold.corrected_error_percent,
            fold.baseline_penalty_percent,
            fold.corrected_penalty_percent
        );
    }

    print_family_report(&report.folds);
    println!("\nSUMMARY");
    println!(
        "Baseline mean error: {:.2}%",
        report.mean_baseline_error_percent
    );
    println!(
        "Corrected mean error: {:.2}%",
        report.mean_corrected_error_percent
    );

    println!(
        "Calibration adoption supported: {}",
        report.adoption_supported
    );

    if !report.adoption_supported {
        println!("Decision: retain the verified hardware profile.");
        println!("Further independent workload evidence is required.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_known_operation_families() {
        assert_eq!(workload_family("addition_u8"), "addition");
        assert_eq!(workload_family("nested_addition_u8"), "addition");
        assert_eq!(workload_family("subtraction_u8"), "subtraction");
        assert_eq!(workload_family("nested_subtraction_u8"), "subtraction");
        assert_eq!(workload_family("multiplication_u8"), "multiplication");
        assert_eq!(workload_family("greater_than_u8"), "comparison");
        assert_eq!(workload_family("equality_u8"), "comparison");
    }

    #[test]
    fn unknown_workloads_are_not_misclassified() {
        assert_eq!(workload_family("custom_circuit"), "unclassified");
        assert_eq!(workload_family("addition_u16"), "unclassified");
    }
    #[test]
    fn median_handles_even_and_odd_samples() {
        assert_eq!(median(&mut [4.0, 1.0, 3.0]), 3.0);
        assert_eq!(median(&mut [4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn rejects_mismatched_candidate_sets() {
        let mut a = Samples::new();
        let mut b = Samples::new();

        let sample = || Sample {
            predicted_ms: 100.0,
            measured_ms: 150.0,
        };

        a.insert(
            ("first".into(), "prefix".into()),
            (0..4).map(|_| sample()).collect(),
        );

        b.insert(
            ("second".into(), "prefix".into()),
            (0..4).map(|_| sample()).collect(),
        );

        assert!(aggregate(a, b).is_err());
    }

    #[test]
    fn rejects_prediction_drift() {
        let key = ("addition".into(), "prefix".into());

        let mut a = Samples::new();
        let mut b = Samples::new();

        a.insert(
            key.clone(),
            vec![
                Sample {
                    predicted_ms: 100.0,
                    measured_ms: 150.0,
                },
                Sample {
                    predicted_ms: 100.0,
                    measured_ms: 151.0,
                },
            ],
        );

        b.insert(
            key,
            vec![
                Sample {
                    predicted_ms: 120.0,
                    measured_ms: 152.0,
                },
                Sample {
                    predicted_ms: 120.0,
                    measured_ms: 153.0,
                },
            ],
        );

        assert!(aggregate(a, b).is_err());
    }
}
