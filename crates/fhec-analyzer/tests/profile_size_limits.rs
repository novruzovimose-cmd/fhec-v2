use std::{
    fs,
    io::ErrorKind,
    time::{SystemTime, UNIX_EPOCH},
};

use fhec_analyzer::hardware_profile::{HardwareProfileFile, HardwareProfileIoError};
use serde::Deserialize;

#[derive(Deserialize)]
struct TestPayload {
    #[allow(dead_code)]
    value: usize,
}

fn unique_path() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("fhec-b1-size-{}-{stamp}.json", std::process::id()))
}

#[test]
fn oversized_profile_is_rejected_before_json_parsing() {
    let path = unique_path();
    let oversized = vec![b' '; 1_048_577];
    fs::write(&path, oversized).unwrap();

    let result = HardwareProfileFile::<TestPayload>::load_json(&path);
    fs::remove_file(&path).unwrap();

    assert!(matches!(
        result,
        Err(HardwareProfileIoError::Io(error)) if error.kind() == ErrorKind::InvalidData
    ));
}

#[test]
fn small_malformed_profile_still_reports_json_error() {
    let path = unique_path();
    fs::write(&path, b"{ this is not JSON").unwrap();

    let result = HardwareProfileFile::<TestPayload>::load_json(&path);
    fs::remove_file(&path).unwrap();

    assert!(matches!(result, Err(HardwareProfileIoError::Json(_))));
}
