# Clean-checkout and cross-platform acceptance

## Local source-only reproduction

The `verify-local.ps1` script exports source to a newly created folder under `target`, checks the copy's hashes, chooses a separate `CARGO_TARGET_DIR`, and builds/tests that snapshot. It deliberately **does not** use Git to assemble the copy, and retains the normal Cargo registry cache unless `-ColdCargoHome` is requested. This checks many hidden-source-file assumptions while remaining feasible locally. It cannot reproduce Linux or a fresh Git clone and is not an independent security certification.

## Fresh hosted checkout

The GitHub workflow fetches an exact commit onto both hosted operating systems and executes the same verification gates in a pristine runner checkout. It needs network access for dependencies, a working Rust toolchain, and available hosted runner capacity. Windows/Ubuntu runtime, compiler, CPU, and native libraries may differ; do not imply that either platform passed before its corresponding Actions job reports success.

## Mandatory checks

1. Rust formatting: `cargo fmt --all -- --check`.
2. Compilation: `cargo check --workspace --locked`.
3. Full tests: `cargo test --workspace --locked`, including expensive cryptographic integration tests.
4. Lint: `cargo clippy --workspace --all-targets --locked -- -D warnings`.
5. Release binaries: `cargo build --release --locked -p fhec-cli -p fhec-bench --bins`.
6. Direct executable help, `build`, and `analyze` on a valid MUX example.
7. Locked dependency inventory; RustSec advisory check and human license review.

The tooling deliberately avoids generating or storing private keys as release artifacts. Tests may generate transient TFHE test keys internally. Direct command examples use public test programs.

## Exact commit signoff

Successful jobs must refer to the exact commit that will be tagged. If Cargo.lock or source changes after a successful run, CI must rerun. Other workflows in `.github/workflows/` must also be reviewed. Publishing tags and release artifacts remains a separate final-stage decision.
