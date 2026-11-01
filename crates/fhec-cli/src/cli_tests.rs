use super::dispatch;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct SourceFixture {
    path: PathBuf,
}

impl SourceFixture {
    fn new(source: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        let path =
            std::env::temp_dir().join(format!("fhec-cli-test-{}-{id}.fhe", std::process::id()));

        fs::write(&path, source).expect("write test source");

        Self { path }
    }

    fn path(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

impl Drop for SourceFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn invoke(args: &[&str]) -> Result<(), String> {
    let owned = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
    dispatch(&owned).map_err(|error| error.to_string())
}

#[test]
fn help_commands_succeed() {
    assert!(invoke(&[]).is_ok());
    assert!(invoke(&["--help"]).is_ok());
    assert!(invoke(&["-h"]).is_ok());
    assert!(invoke(&["help"]).is_ok());
}

#[test]
fn missing_source_arguments_report_usage() {
    let build = invoke(&["build"]).unwrap_err();
    assert!(build.contains("Usage: fhec build"));

    let analyze = invoke(&["analyze"]).unwrap_err();
    assert!(analyze.contains("Usage: fhec analyze"));

    let run = invoke(&["run"]).unwrap_err();
    assert!(run.contains("Usage: fhec run"));
}

#[test]
fn extra_source_arguments_are_rejected() {
    assert!(invoke(&["build", "a.fhe", "b.fhe"]).is_err());
    assert!(invoke(&["analyze", "a.fhe", "b.fhe"]).is_err());
}

#[test]
fn unknown_commands_report_command_name() {
    let error = invoke(&["frobnicate"]).unwrap_err();
    assert!(error.contains("frobnicate"));
    assert!(error.contains("--help"));
}

#[test]
fn build_compiles_and_verifies_valid_source() {
    let fixture = SourceFixture::new(
        "fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }",
    );

    assert!(invoke(&["build", &fixture.path()]).is_ok());
}

#[test]
fn analyze_accepts_valid_source() {
    let fixture = SourceFixture::new(
        "fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }",
    );

    assert!(invoke(&["analyze", &fixture.path()]).is_ok());
}

#[test]
fn nonexistent_source_reports_filename() {
    let fixture = SourceFixture::new(
        "fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }",
    );

    let path = fixture.path();
    drop(fixture);

    let error = invoke(&["build", &path]).unwrap_err();

    assert!(error.contains("Source file not found"));
    assert!(error.contains(&path));
}

#[test]
fn malformed_source_is_rejected() {
    let fixture = SourceFixture::new("fn broken(");

    assert!(invoke(&["build", &fixture.path()]).is_err());
}
