use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use fhec_analyzer::hardware_profile::{
    HardwareFingerprint, HardwareProfileFile, HardwareProfileMetadata, HardwareProfileSource,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Sample {
    measured_ns: u64,
}

fn test_path(tag: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fhec-b3-{tag}-{}-{unique}.json",
        std::process::id()
    ))
}

fn sample(measured_ns: u64) -> HardwareProfileFile<Sample> {
    HardwareProfileFile::new(
        HardwareProfileMetadata::new(
            "tfhe-boolean-v4",
            HardwareFingerprint::current(),
            HardwareProfileSource::FreshCalibration,
        ),
        Sample { measured_ns },
    )
}

#[test]
fn overwrites_a_complete_profile_and_can_reload_it() {
    let path = test_path("replace");
    sample(10).save_json(&path).unwrap();
    sample(25).save_json(&path).unwrap();
    let reloaded = HardwareProfileFile::<Sample>::load_json(&path).unwrap();
    assert_eq!(reloaded.payload.measured_ns, 25);
    fs::remove_file(&path).unwrap();
}

#[test]
fn refusing_to_replace_a_directory_preserves_its_contents() {
    let path = test_path("directory");
    fs::create_dir(&path).unwrap();
    let child = path.join("keep.txt");
    fs::write(&child, "untouched").unwrap();
    assert!(sample(30).save_json(&path).is_err());
    assert_eq!(fs::read_to_string(&child).unwrap(), "untouched");
    fs::remove_file(child).unwrap();
    fs::remove_dir(path).unwrap();
}

#[cfg(unix)]
#[test]
fn refuses_to_replace_a_symlink_target() {
    let real = test_path("target");
    let alias = test_path("symlink");
    fs::write(&real, "unchanged").unwrap();
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    assert!(sample(50).save_json(&alias).is_err());
    assert_eq!(fs::read_to_string(&real).unwrap(), "unchanged");
    fs::remove_file(alias).unwrap();
    fs::remove_file(real).unwrap();
}
