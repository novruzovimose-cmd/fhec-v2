# FHEC V1 Benchmark Methodology

This document describes the V1 benchmark tools and how to interpret their output. Run commands from the root of a FHEC source checkout with Cargo installed. The benchmark package has multiple binaries; specify `--bin fhec-bench` when running it directly.

## Benchmark modes

| Mode | Example command | Measures |
| --- | --- | --- |
| Default clear circuit | `cargo run --release --locked -p fhec-bench --bin fhec-bench -- --iterations 30 --output target/fhec-benchmarks` | Source lexing/parsing/IR/circuit compilation and **unencrypted** Boolean circuit evaluation, in nanoseconds |
| Real TFHE | `cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode tfhe --iterations 3 --output target/fhec-benchmarks` | Real encrypted circuit execution, in milliseconds |
| Optimizer comparison | `cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode optimizer --iterations 3 --output target/fhec-benchmarks` | Baseline versus optimized real TFHE execution, in milliseconds |
| Candidate comparison | `cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode compare --iterations 3 --warmup 1 --profile target/fhec-profiles/tfhe-v4.json --output target/fhec-benchmarks` | Candidate timings, prediction accuracy, and measurement quality with a **pre-existing validated hardware profile** |

Clear-circuit results are **not encrypted TFHE timings**. They must not be presented as measured FHE execution performance. `fhec bench` delegates to Cargo; it is not a standalone benchmark binary that functions without the FHEC checkout and Cargo. To run the standalone benchmark package, use the commands above.

## Workloads and correctness

The default benchmark includes addition, subtraction, multiplication, equality, nested function calls, selection by encrypted comparison (`mux_u8`), nested encrypted selection, and Boolean conditional selection. The TFHE mode adds an encrypted-MUX workload and a Boolean-returning conditional workload. The optimizer mode compares baseline and optimized MUX execution. Each benchmark checks the decrypted or clear output against its expected value and terminates on a mismatch.

Boolean operators `&&`, `||`, and `^` are **not** asserted as source-language features. Boolean-heavy workloads use supported encrypted comparisons and conditional MUX selection.

Default workload inputs are fixed (37 and 5) to keep different revisions comparable. For tests with other input values, run the source-level regression suite. A benchmark on one fixed input pair does not establish correctness for all possible inputs.

## Reproducibility protocol

1. Record commit or source revision, Rust version, OS version, CPU model, available cores, RAM, and TFHE backend/library versions.
2. Record the workload name, iteration count, warmup count where supported, runtime mode, calibration profile path, and hardware profile provenance.
3. Perform a warmup before timed comparisons and repeat measurements. The `compare` mode supports configurable warmup; the basic TFHE and optimizer modes do not provide that same warmup control, so do not imply they do.
4. Avoid concurrent CPU-heavy work, thermal throttling, and power-mode changes; results can vary across machines.
5. Keep the raw CSV/JSON and use per-workload results, not just averages. Small sample sizes have substantial variance; for published numbers, report dispersion and the run environment.
6. Compare baseline and optimized results on the same hardware and configuration. A speedup below 1x is a regression, not a benefit.

The default clear benchmark reports an integer-nanosecond median: for even sample counts, it averages the two middle values with truncation below one nanosecond. Real TFHE benchmark modes report floating-point millisecond medians.

## Profile accuracy and known regression

Hardware estimates and their prediction intervals are **models**, not direct execution observations. Uncalibrated predictions must not be interpreted as zero-millisecond TFHE execution.

An earlier experimental global calibration correction reduced average prediction error on sampled workloads but **worsened multiplication prediction error**. The adoption guard rejected that global correction; it must not be claimed as adopted or as a uniform accuracy gain. Re-evaluate calibrated models on held-out workload families before changing production defaults.

TFHE gate bootstrapping and FHEC analyzer noise-pressure estimates are distinct concepts. This project does not implement general-purpose multi-scheme noise-budget scheduling for BFV/BGV/CKKS.

## Output files

- Default clear mode: `results.csv`, `results.json`, tagged as clear-circuit measurements.
- TFHE mode: `tfhe_results.csv`, `tfhe_results.json`, tagged as real TFHE execution.
- Optimizer mode: `optimizer_results.csv`.
- Candidate comparison: refer to the CSV reports and raw samples written by the comparison tool.

Use a distinct `--output` directory per experiment to avoid mixing files from different runs.
