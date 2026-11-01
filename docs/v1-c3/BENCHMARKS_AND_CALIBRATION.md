# FHEC benchmark methodology and persistent calibration

## 1. Use the correct measurement category

FHEC has at least three distinct forms of performance information:

1. **Clear-circuit benchmark:** compilation and **unencrypted Boolean evaluation**, typically reported in nanoseconds. It is useful for circuit construction and baseline behavior, but it is NOT FHE throughput.
2. **Real TFHE benchmark:** actual encrypted circuit evaluation using TFHE Boolean gates, typically reported in milliseconds. These samples are hardware-sensitive and include cryptographic computation.
3. **Analyzer predictions:** modeled latency, crypto counts, memory and uncertainty. They are not timings and are not guaranteed to agree with measurement.

Never put numbers from these three categories into one chart without explicit labels. A 1,800 ns clear-circuit evaluation must not be called a 1.8 microsecond encrypted TFHE execution. Similarly, `0.000 ms` in an uncalibrated analyzer display is not proof of zero real execution time.

## 2. Reproducible commands

Run from the repository root with Cargo installed. Because `fhec-bench` has multiple binaries, explicitly choose `--bin fhec-bench` when invoking Cargo directly.

```powershell
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --iterations 30 --output target/bench-clear-c3
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode tfhe --iterations 3 --output target/bench-tfhe-c3
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode optimizer --iterations 3 --output target/bench-opt-c3
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode compare --iterations 3 --warmup 1 --profile target/fhec-profiles/tfhe-v4.json --output target/bench-compare-c3
```

For a one-iteration **smoke test only**:

```powershell
cargo run --release --locked -p fhec-cli -- bench --iterations 1 --output target/bench-smoke-c3
```

The CLI `bench` command delegates to Cargo; it requires the repository and Rust toolchain rather than functioning independently when the binary is copied elsewhere. When a benchmark mode requires a persisted profile, validate that profile before claiming hardware-calibrated results.

## 3. Reference workload families

Current C1 default clear mode includes 8 named workloads: `add_u8`, `sub_u8`, `mul_u8`, `eq_u8`, `nested_calls_u8`, `mux_u8`, `nested_mux_u8`, and `comparison_mux_bool`. TFHE and optimizer modes add relevant encrypted condition and MUX checks. Candidate-comparison mode includes an expanded workload set with arithmetic, comparisons and multiplication.

Additional examples cover signed `i8`, `u16` addition, Boolean selection, nested arithmetic and comparison. It is valid to use these for compiler correctness even if they are not all represented in every benchmark mode. Do not claim complete real-TFHE multi-width benchmark coverage without running it.

The C2 reported a successful clear-circuit smoke run using one iteration and output files for all eight workloads. **Those individual one-iteration numbers are not publication-quality performance evidence.**

## 4. Timing protocol

For credible results:

- Record date, source revision, toolchain, CPU, worker threads, memory, OS, TFHE library version, cache/power conditions, source file, data types, and workload IDs.
- Choose a fixed configuration and include warmups when the mode supports it. Do not pretend every mode has the same warmup controls.
- Run enough repetitions to characterize variability; report counts and dispersion instead of presenting one timing as a stable median.
- Preserve per-run raw data and separately summarize per-workload medians/dispersion. Check for workload-specific regressions.
- Use unique `--output` directories to avoid overwriting prior results.
- For optimizer comparisons, evaluate both versions under the same backend, input values and load; a predicted improvement should be treated as a hypothesis until empirical TFHE measurements validate it.
- For best/worst-case sensitivity, test cold and warm profile states, varied input widths and deep vs wide circuits; do not conflate one-host evidence with portability.

A zero-iteration test or unsupported benchmark mode must fail explicitly rather than generate plausible-looking empty charts.

## 5. Medians and precision

The benchmark's even-sized integer-nanosecond median is the integer average of the two middle samples (fractional nanoseconds truncated). With an odd sample count, it selects the middle sorted value. The C1 regression suite verifies even and odd behavior. A median from one observation is the observation itself and says little about variability. Real TFHE modes can use floating-point millisecond medians and may differ in summary fields.

## 6. Artifact formats and interpretation

- Clear default output: `results.csv` and `results.json`, explicitly marked clear/non-TFHE.
- Real TFHE mode: `tfhe_results.csv`, `tfhe_results.json` where produced.
- Optimizer real TFHE comparison: `optimizer_results.csv`.
- Candidate comparison mode: profile-specific comparison and raw-measurement files; inspect emitted names rather than assuming a universal schema.

When preparing a research figure or release table, include: category, time units, sample count, variability, hardware provenance, circuit source, profile file source, and the baseline. Code compilation time and encrypted execution time measure different operations.

## 7. Calibration workflow and failure handling

```powershell
cargo run --release --locked -p fhec-cli -- profile show
cargo run --release --locked -p fhec-cli -- profile verify
cargo run --release --locked -p fhec-cli -- profile calibrate
```

`show` and `verify` may return nonzero when the profile is missing or invalid; this is expected. `calibrate` performs real cryptographic measurements and can be slow. Calibration profiles live by default under the source root at `target/fhec-profiles/tfhe-v4.json`, not in a global operating-system secret store.

Profile validation from Stage B includes structural scaling checks, invalid/oversized JSON rejection, incompatible or stale metadata, and bounds for declared widths. Profile persistence stages data before replacement. **On Windows the backup swap is not fully crash-atomic**; after a power loss the operator may need to recover or recalibrate manually. Profiles are performance metadata, not cryptographic key material.

A hardware profile is usable only if it matches the backend/profile version and relevant hardware fingerprint requirements. Copying a profile from another machine may produce rejection, as it should.

## 8. Multiplication-model regression, not an adopted optimization

In earlier experiments, a global correction reduced average prediction error from roughly 37.71% to 14.45% on a seven-workload sample, but **multiplication error worsened** from around 19.05% to 34.93%. A 13-workload extension improved the aggregate but still regressed multiplication. The adoption guard **rejected** the global correction. Do not claim the optimized production predictor adopts those new corrections or that accuracy improved uniformly.

A model should only be promoted after held-out validation shows no unacceptable per-operation-family regressions and complete model/hardware provenance is saved. Absolute timing from one workstation does not generalize to other machines.

## 9. Limits of predictions and noise metrics

Latency estimates may undercount runtime overhead, scheduler effects, real TFHE implementation choices, allocator overhead and platform differences. Peak ciphertext memory estimates model object lifetimes and declared sizes; they are not measured process peak RSS. Bootstrap and key-switch counts are backend-model assumptions. Abstract noise-pressure reports are not multi-scheme decryption-failure guarantees.

Uncalibrated reports should be explicitly labeled uncalibrated. A report that merely prints zero milliseconds is ambiguous unless its source and confidence labels explain why. C3 checks that the CLI announces default, non-hardware-calibrated analysis.

## 10. Acceptance and release readiness

C1 validated benchmark correctness, real TFHE candidate coverage, and successful release compilation. C2 validated the direct binary and a one-iteration clear benchmark through `fhec bench`. C3 validates report interpretation and linked documentation; it does not repeat an entire statistically meaningful performance study. D-G must verify fresh-checkout reproducibility and CI on Linux and Windows before public release.
