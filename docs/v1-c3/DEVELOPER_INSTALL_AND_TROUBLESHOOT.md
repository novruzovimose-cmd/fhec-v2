# FHEC V1 developer installation, first run and troubleshooting

## 1. Target audience and release status

These instructions build the current FHEC **source checkout**. They are not the result of a completed public package install, Linux clean-machine CI, or GitHub release. Those are D-G release gates. V1 is a TFHE Boolean-circuit compiler and CLI, not a self-contained distributable app with bundled native libraries.

### Requirements

- A Rust toolchain supporting workspace edition **2024** (the inspected workspace declared Rust 1.85+ and `rust-toolchain.toml` pinned `stable`). Install `rustfmt` and `clippy` components.
- A supported native C/C++ build toolchain as required by TFHE-rs dependencies. On Windows, MSVC Build Tools and the Visual Studio Developer PowerShell or correctly configured environment may be required for `link.exe`.
- Sufficient memory and CPU capacity for real TFHE execution, which is significantly heavier than clear-circuit evaluation.
- An intact `Cargo.lock`; production and CI commands use `--locked` to avoid accidental dependency drift.

### Windows PowerShell from project root

```powershell
cd C:\path\to\fhec
cargo --version
rustc --version
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test -p fhec-cli --test cli_contract_c2 --locked
cargo build --release --locked -p fhec-cli
.\target\release\fhec-cli.exe --help
```

The Windows directory shown is a **development example**, not a required or public release path. Use your actual checkout directory. If `CARGO_TARGET_DIR` is set, find the executable inside that configured target directory rather than assuming `target/release`.

### Linux from project root

```bash
rustc --version
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test -p fhec-cli --test cli_contract_c2 --locked
cargo build --release --locked -p fhec-cli
./target/release/fhec-cli --help
```

Some Linux distributions require additional native build packages and a working C compiler/linker. Linux commands are **instructions pending clean CI verification**, not an assertion that they passed in C3 on Windows.

## 2. Exact executable name

The current Cargo package is `fhec-cli`, and on Windows its executable is `fhec-cli.exe` (on Linux usually `fhec-cli`). The CLI help displays the shorter conceptual name `fhec` in examples, but **that is not an automatically installed executable alias**. Use the executable name that `cargo build --release -p fhec-cli` actually produces, or create a clearly documented alias yourself.

## 3. First compile, analyze and encrypted run

After a release build:

```powershell
.\target\release\fhec-cli.exe build examples\conditional_mux.fhe
.\target\release\fhec-cli.exe analyze examples\conditional_mux.fhe
'37,5' | .\target\release\fhec-cli.exe run examples\conditional_mux.fhe --function pick_greater --inputs-stdin
```

`build` parses, lowers and verifies; it does not serialize a reusable compiled artifact in the inspected V1 implementation. `analyze` uses a default uncalibrated cost model; its predicted latency is not wall time. `run` generates/uses TFHE key material and returns the **decrypted** output. With public demo inputs `37` and `5`, `pick_greater` returns `37`.

You can use inline test arguments:

```powershell
.\target\release\fhec-cli.exe run examples\conditional_mux.fhe --function pick_greater --inputs 37,5
```

Avoid `--inputs` for genuinely sensitive inputs because process arguments and shell history may expose them. Even `--inputs-stdin` does not eliminate exposure from other host mechanisms.

## 4. Other example programs

```powershell
.\target\release\fhec-cli.exe build examples\c3\nested_selection.fhe
.\target\release\fhec-cli.exe build examples\c3\signed_comparison.fhe
.\target\release\fhec-cli.exe build examples\c3\wide_addition.fhe
.\target\release\fhec-cli.exe build examples\c3\boolean_selection.fhe
```

These are intended as source-language coverage examples, not proof of all possible encrypted output values. Run the full A/B/C test suites and representative real TFHE scenarios before release.

## 5. Profiles

```powershell
.\target\release\fhec-cli.exe profile show
.\target\release\fhec-cli.exe profile verify
.\target\release\fhec-cli.exe profile calibrate
```

`show` and `verify` fail with nonzero status if the profile is absent, stale, malformed or incompatible. Calibration produces fresh performance measurements on the local machine; it may take substantial time and must not be forced automatically during fast test gates. The normal profile path is relative to the checkout, under `target/fhec-profiles/tfhe-v4.json`.

## 6. Benchmarks

```powershell
.\target\release\fhec-cli.exe bench --iterations 1 --output target\bench-smoke
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode tfhe --iterations 3 --output target\bench-tfhe
```

`bench` invokes Cargo and therefore needs the checkout and toolchain; it is not a self-sufficient benchmark command from an isolated copied executable. One-iteration clear-circuit results are strictly a smoke test and not comparable to real TFHE execution latencies. For full reproducibility follow [BENCHMARKS_AND_CALIBRATION.md](BENCHMARKS_AND_CALIBRATION.md).

## 7. Fast vs expensive test layers

| Command | Purpose | Typical expectation |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Source formatting | Fast |
| `cargo check --workspace --locked` | Type checking with pinned dependencies | Usually fast after first build |
| `cargo test -p fhec-cli --test cli_contract_c2 --locked` | CLI error paths and clear build/analyze commands | Fast; no real TFHE execution |
| `cargo test -p fhec-optimizer --locked` | Optimizer regressions | Fast |
| `cargo test -p fhec-compiler --locked` | Compiler correctness and selection invariants | Fast after compilation |
| `cargo test -p fhec-bench --bin fhec-bench --locked` | Clear benchmark unit correctness | Fast |
| `cargo test --workspace --locked` | Entire test suite including real TFHE | May take several minutes |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | All-target lint signoff | Fast after compilation |
| `cargo build --release --locked -p fhec-cli -p fhec-bench --bins` | Release executables | Can take longer on clean machine |

The C1 Windows run observed two real TFHE arithmetic tests running for over 60 seconds before successfully finishing around 75 seconds for the integration executable. That is not necessarily a hang. Use live output; if necessary run `cargo test ... -- --nocapture` when debugging a specific target.

## 8. Troubleshooting

**`link.exe` not found / MSVC link failure:** The native C/C++ linker isn't on PATH. Install the matching Visual Studio C++ tools and run from a Developer PowerShell or appropriately configured Rust/MSVC environment; rerun `cargo check --workspace --locked`.

**Wrong working directory:** `examples/` and `target/fhec-profiles/` are relative paths. Navigate to the repository root before running examples, `bench`, or profile commands.

**Multiple binaries in `fhec-bench`:** Use `--bin fhec-bench` in direct Cargo commands; `cargo run -p fhec-bench` alone may be ambiguous.

**`--locked` errors:** Your `Cargo.lock` is out of sync or a new dependency was introduced. Do not delete the lockfile or run an unreviewed dependency upgrade as a workaround. Inspect the manifest changes and intentionally regenerate/review the lockfile in a separate change.

**`cargo fmt -- --check` shows diffs:** The source is not formatted. For deliberate formatting changes, save/inspect a diff, run `cargo fmt --all`, then rerun verification. Don't apply formatting without considering the repository's scope and clean-state requirements.

**PowerShell `elseif` / `else` not recognized:** Interactive PowerShell may execute individual blocks as separate statements; paste one contiguous block or run a saved `.ps1` file. The C3 verifier is saved as a file to avoid fragile interactive nesting.

**PowerShell `log file:` appears:** This is often Git's pager, not a request from FHEC. Press `Esc`, then `q` to exit; use `git -c core.pager=cat ...` when Git output should be noninteractive. C3 performs no Git operations.

**Test output appears to freeze:** Check which Cargo command is running and whether the verifier redirects output. C3 streams Cargo's output directly; the full cryptographic tests may genuinely take minutes. Don't assume that Ctrl+C performed automatic rollback.

**Hardware profile invalid/missing:** Run `profile verify`, inspect path and host compatibility, and consider a fresh calibration on trusted local hardware. Do not silently accept corrupted calibration JSON.

**Uncalibrated report prints `0.000 ms`:** This is model rounding or missing calibration provenance, not proof of zero cryptographic latency. Use real TFHE benchmarks to measure performance.

**`--inputs` invalid / wrong number of inputs:** Inspect the function signature and the declared types. Numeric and Boolean ranges are enforced. C2 tests ensure invalid user-supplied values aren't echoed in error diagnostics.

**Linux fails while Windows works:** Native dependencies, compiler toolchains, filesystem behavior and TFHE features can differ. The D-stage clean Linux CI is mandatory before describing FHEC V1 as portable.

**Clippy complains about a new test:** Fix the test rather than suppressing the warning or disabling `-D warnings`. Keep release test files formatted and checked under `--all-targets`.

## 9. Important boundaries before public release

Build and run evidence from one existing Windows checkout is not the same as a clean clone on Windows+Linux. The future D-G gates must validate the exact release commit, artifact naming, installer experience, license compliance, dependency advisories, secret scans, README and changelog versions, and release archive hygiene.
