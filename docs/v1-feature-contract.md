# FHEC V1 feature contract â€” draft for source verification

**Status:** release candidate. This document identifies claimed behavior to confirm against the current compiler source and tests before publication.

## Build and execution

- `fhec-cli build <file>` parses, lowers, and verifies circuits. In the inspected source, it does **not** emit a reusable compiled binary or serialized circuit artifact.
- `fhec-cli analyze <file>` reports candidate circuits and analyzer estimates; estimates from the uncalibrated default model are not measured latency.
- `fhec-cli run <file> --function <name> --inputs <values>` encrypts typed inputs, executes TFHE, and decrypts the output.
- `fhec-cli profile` and `fhec-cli bench` provide calibration and benchmarking workflows; confirm their current CLI subcommands before final publication.

## Source-language scope to verify

- Encrypted input/output types exercised by CLI encoding: `encrypted<u8>`, `encrypted<u16>`, `encrypted<i8>`, `encrypted<bool>`.
- Arithmetic: addition, subtraction, multiplication.
- Comparison: equality, inequality, less-than/greater-than and inclusive forms.
- Conditional expressions/branches and function calls are implemented in the inspected compiler and have integration tests.
- Overflow behavior and signed behavior must be verified with the exact tests and per-type operator contract.

These are **not** claims of complete support for every operator/type combination. Confirm the compatibility matrix and errors through tests.

## Optimization contract

- FHEC generates ripple-carry and parallel-prefix candidates and evaluates optimized variants.
- Analyzer hints can include actions that are not implemented transformations. Do not advertise every hint as an automatic pass.
- Hardware-aware prediction is approximate. Empirical calibration improved average error on one dataset but regressed multiplication; do not deploy the rejected global correction.

## Unsupported or not established in V1

- Arbitrary Rust-language programs, rich arrays/structs, unrestricted loops, and general standard library.
- OpenFHE/BFV/BGV/CKKS or GPU execution.
- General multi-scheme automatic noise-budget scheduling.
- Independently audited cryptographic security.

## Release sign-off tasks

- [ ] Run representative positive and negative tests for each supported type/operator combination.
- [ ] Confirm exact CLI syntax and exit behavior from a clean checkout.
- [ ] Review optimizer actions versus implemented transformations.
- [ ] Ensure README uses the same limitation statements.
