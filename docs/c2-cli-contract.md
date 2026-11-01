# FHEC V1 CLI acceptance contract (C2)

This document records what C2 verifies. It does not claim an independently tested clean installation, a published release, or support for arbitrary FHE programs.

## Executable name

The Cargo package is `fhec-cli`, and its default binary is `fhec-cli` (`fhec-cli.exe` on Windows). Help and documentation sometimes abbreviate the command to `fhec`. That abbreviation is not an installed command alias. Users must invoke `fhec-cli` or explicitly create their own shell alias. The final documentation/release review must choose one advertised name and be consistent.

## Supported commands and scope

- `fhec-cli --help`: display usage and commands.
- `fhec-cli build examples/conditional_mux.fhe`: parse, lower, construct, and verify the circuit. It prints a summary but does **not** emit a reusable compiled artifact in the implementation inspected for C2.
- `fhec-cli analyze examples/conditional_mux.fhe`: produce a candidate/cost report. Default cost predictions are not real execution timings.
- `fhec-cli run examples/conditional_mux.fhe --function pick_greater --inputs-stdin`: perform real encrypted execution and print the **decrypted** result. Standard input can reduce exposure in process argument listings but does not make the output secret.
- `fhec-cli profile show|verify|calibrate`: inspect or calibrate hardware profiles. Calibration is expensive and is **not** run by the fast C2 test gate.
- `fhec-cli bench --iterations 1 --output target/fhec-c2-example`: delegates to `cargo run` for the `fhec-bench` package. **This requires Cargo and a working FHEC source checkout as the current directory.** Copying only `fhec-cli.exe` elsewhere does not make `bench` standalone.

## C2 test layers

1. `cargo test --locked -p fhec-cli --test cli_contract_c2`: a fast executable-level contract suite that checks actual exit codes, successful `build`/`analyze` examples, input-argument errors and redaction. It never requests TFHE execution.
2. `verify.ps1`: runs the fast contract suite, produces the release CLI binary, checks its help output, runs one clear-circuit benchmark via CLI, and executes one real TFHE source-to-decrypted-result case using the known demo inputs `37,5`.
3. The full `cargo test --workspace --locked`, Clippy `-D warnings`, cross-platform CI, dependency auditing, and clean-checkout verification remain distinct later release gates.

Run the PowerShell verifier from the repository root, after installing the bundle, with `-Project (Get-Location).Path`. It streams native command output instead of hiding it behind an external process waiter. If the verifier fails, the installed C2 files are left untouched, so the failure can be inspected and fixed; the installer only removes C2 files it created if its own preflight/write fails.

## Security and usability limits

Validation errors must return a nonzero status without repeating rejected plaintext values. Success intentionally emits a decrypted output; logs containing that output should not be shared as if they were confidential. Test inputs `37` and `5` are public test fixtures, not real secrets.

The `--inputs-stdin` option accepts a bounded single line of comma-separated values, not a long-running stream. The `run` command currently creates or reuses a profile under the working directory's `target/fhec-profiles/`. A valid environment, TFHE native build dependencies, and appropriate CPU resources are required for real TFHE runs.

The C2 result supports usability and reproducibility claims only for the specific commands actually run and the host where they pass. C2 does **not** by itself close Stage C, final documentation, or GitHub publication.
