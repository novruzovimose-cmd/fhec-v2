# C3 acceptance ledger and remaining V1 release obligations

## Scope

Stage C3 is the **single final Stage C milestone** after C1 benchmark coverage and C2 CLI developer-experience acceptance. It checks whether the current source behavior matches the documentation and whether essential analyzer and example workflows still run. It does **not** claim a clean public release or universal correctness.

C3 does not create new numbered sub-sprints. If a gate fails, treat it as a C3 repair. After C3 is verified, the next phase in the user's agreed roadmap is D, then E, F, G; publication comes last.

## What the C3 acceptance verifier checks

1. The bundle is installed completely, and its installed files agree with the original SHA-256 content manifest. No partial or modified C3 document is silently trusted.
2. Existing B/C docs, example source files and critical Rust source paths are present. It doesn't replace those files.
3. The four new C3 examples compile and verify through the executable built from the current checkout.
4. Analyzer reports for the examples identify the default non-hardware-calibrated model, include candidate comparison, prediction intervals, source/confidence labels, optimization decisions and IR cost attribution.
5. Reported selected predicted cost does not exceed rejected predicted cost after allowing for decimal display rounding. Real performance need not be positive or nonzero in a default model.
6. The prior C2 executable contract suite still passes.
7. Format, workspace compile, focused optimizer/compiler/analyzer and benchmark tests, full workspace tests, Clippy warnings denied and release CLI/bench binary builds succeed.
8. No Git, tag, remote, lockfile update, or external publishing operation is performed.

`-FastOnly` deliberately omits full-workspace real TFHE tests; it produces `C3 FAST CHECKS PASSED`, not `STEP C3 VERIFIED`. A full run is needed for a C3 acceptance claim.

## Evidence already supplied (before C3)

- **Stage A:** source-correctness patch bundles A1/A2/A3 installed; full tests and zero-warning Clippy reportedly passed after the fixed A3 bundle.
- **Stage B:** B1/B2/B3 patches installed; protected input handling, profile bounds/persistence, runtime-shape and real TFHE MUX tests passed as reported.
- **C1:** test log shows successful formatting, compilation, benchmark correctness, complete workspace regression tests (including the real TFHE integration target), all-target Clippy with `-D warnings`, and release CLI/bench builds.
- **C2:** user supplied successful 6/6 CLI integration tests, direct binary help, zero-warning CLI Clippy, release build, eight-workload clear benchmark smoke execution, and real encrypted demo with input 37/5 and decrypted output 37.

This is **local host evidence**; it is not a security audit or an external reproduction.

## Stage C signoff matrix

| Checklist item | Evidence or limitation | State before C3 local verification |
| --- | --- | --- |
| Supported-language reference | `LANGUAGE_REFERENCE.md` and examples | Written; verification pending |
| Analyzer terminology/intervals/uncalibrated reporting | `ANALYZER_AND_OPTIMIZER.md` and CLI analyzer contract | Written; executable verification pending |
| Optimizer passes versus advisory hints | Explicit action table in analyzer documentation | Documented against inspected source; final source review pending |
| Benchmark reproducibility/mode labels | C1 benchmark suite and `BENCHMARKS_AND_CALIBRATION.md` | C1 measured locally; written review pending |
| Calibration multiplication regression | Explicitly rejected global correction documented | Documented; model adoption remains rejected |
| TFHE key and noise limitations | `SECURITY_AND_RUNTIME.md`, B3 review | Written; independent audit pending |
| Example build/analyze workflows | New C3 examples and existing C1 examples | C3 executable checks pending |
| CLI installation name and Cargo-backed bench | `DEVELOPER_INSTALL_AND_TROUBLESHOOT.md`, C2 suite | Documented and verified in C2 local run |
| New-user build-to-result walkthrough | Guide plus C2 real-TFHE acceptance evidence | Windows local evidence only |
| Windows/Linux clean-install reproduction | Instructions exist; not yet clean CI validated | **Open: D** |
| README/changelog manifest version agreement | Must be reconciled for final V1 publication | **Open: D-G** |
| External dependency security/license inventory | No current report verified here | **Open: D-G** |

## D-G: release engineering remains required

The user's final roadmap continues with D, E, F and G. Specific mandatory items from the original master checklist include:

- Clean temporary, stale or backup files from the source archive and Git staging.
- Review every staged file for leaked secrets and test data.
- Ensure `.gitignore` excludes `target/`, local profiles, copied backups and private benchmarks.
- Review all `Cargo.toml` versions and the `Cargo.lock` selected for V1; update README and changelog coherently.
- Perform dependency vulnerability and license inventories, including TFHE/native components.
- Make a clean local Git commit only when the full final source is ready, as requested.
- Test fresh-clone reproduction and Windows/Linux CI on the exact intended release commit.
- Validate release binaries, archive contents, license text and executable naming.
- Confirm examples and documentation work without relying on unpublished local files.
- Tag and publish the finished V1 only after those checks pass.

Stage C3 passing supports closing the scoped documentation/analyzer acceptance milestone **only**. It does not automatically mark all earlier checklist security, release engineering, clean-install, or public release items complete.

## Evidence handling

C3 stores logs in `target/fhec-c3-verification/` under a unique local run directory and does not upload data. Keep these logs for test provenance, but check them for filenames, local usernames, and decrypted demo results before sharing publicly. The C3 verifier uses synthetic public sample programs rather than private input values.
