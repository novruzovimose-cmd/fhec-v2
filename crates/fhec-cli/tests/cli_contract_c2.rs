use std::path::PathBuf;
use std::process::{Command, Output};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("FHEC CLI crate must reside under crates/")
        .to_path_buf()
}

fn execute(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fhec-cli"))
        .current_dir(workspace_root())
        .args(args)
        .output()
        .expect("FHEC CLI process must start")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn expect_success(args: &[&str], expected: &str) {
    let result = execute(args);
    assert!(
        result.status.success(),
        "expected success for {args:?}; stdout: {}; stderr: {}",
        stdout(&result),
        stderr(&result)
    );
    assert!(
        stdout(&result).contains(expected),
        "missing {expected:?} in CLI output: {}",
        stdout(&result)
    );
}

fn expect_failure(args: &[&str], expected: &str) {
    let result = execute(args);
    assert!(
        !result.status.success(),
        "expected nonzero status for {args:?}; stdout: {}",
        stdout(&result)
    );
    assert!(
        stderr(&result).contains(expected),
        "missing {expected:?} in CLI diagnostic: {}",
        stderr(&result)
    );
}

#[test]
fn help_and_empty_invocation_succeed() {
    expect_success(&["--help"], "USAGE:");
    expect_success(&[], "FHEC");
}

#[test]
fn standalone_binary_builds_and_verifies_mux_example() {
    expect_success(
        &["build", "examples/conditional_mux.fhe"],
        "Build successful.",
    );
    expect_success(&["build", "examples/boolean_mux.fhe"], "Verification: PASS");
}

#[test]
fn standalone_binary_analyzes_mux_example() {
    expect_success(
        &["analyze", "examples/conditional_mux.fhe"],
        "Analysis complete.",
    );
}

#[test]
fn invalid_commands_and_missing_sources_exit_nonzero() {
    expect_failure(&["not-a-real-command"], "FHEC error:");
    expect_failure(&["build"], "Usage: fhec build");
    expect_failure(
        &["build", "examples/c2-does-not-exist.fhe"],
        "Source file not found",
    );
    expect_failure(
        &["analyze", "examples/c2-does-not-exist.fhe"],
        "Source file not found",
    );
    expect_failure(&["profile", "not-a-real-subcommand"], "Usage:");
}

#[test]
fn run_argument_validation_exits_nonzero_without_crypto() {
    let source = "examples/conditional_mux.fhe";
    expect_failure(
        &["run", source, "--function", "pick_greater"],
        "Missing --inputs",
    );
    expect_failure(
        &[
            "run",
            source,
            "--function",
            "pick_greater",
            "--inputs",
            "37",
        ],
        "expects 2 inputs",
    );
    expect_failure(
        &[
            "run",
            source,
            "--function",
            "pick_greater",
            "--inputs",
            "37,5",
            "--inputs-stdin",
        ],
        "Only one input source",
    );
    expect_failure(
        &[
            "run",
            source,
            "--function",
            "pick_greater",
            "--inputs",
            "37,5",
            "--unrecognized-flag",
        ],
        "Unknown run argument",
    );
}

#[test]
fn rejected_input_values_are_not_echoed_by_cli() {
    let secret = "c2_sensitive_marker_98171";
    let result = execute(&[
        "run",
        "examples/conditional_mux.fhe",
        "--function",
        "pick_greater",
        "--inputs",
        "c2_sensitive_marker_98171,5",
    ]);
    assert!(!result.status.success());
    assert!(stderr(&result).contains("Invalid encrypted<u8> value"));
    assert!(!stderr(&result).contains(secret));
    assert!(!stdout(&result).contains(secret));
}
