# FHEC V1 feature contract and known limitations

This reference is an **acceptance review sheet**, not an invented expansion of the language. Use `docs/v1-c3/LANGUAGE_REFERENCE.md` and the final release commit's tests as the precise source of supported syntax and operator combinations.

## Supported platform and language subset

FHEC is a source-to-circuit compiler implemented in Rust, with an actual TFHE-backed encrypted execution path. The locally observed workflows support a restricted typed source language, encrypted scalar data types, Boolean operations, fixed-width integer arithmetic/comparisons, function calls, nested expressions and encrypted selection by MUX. The verified source-to-result test `pick_greater(37,5)` returned `37` through actual TFHE execution.

A successful `fhec build` compiles and verifies the circuit; it does not necessarily write a reusable executable circuit artifact. The built CLI executable is called `fhec-cli` under the current Cargo package setup. `fhec bench` is a Cargo-backed delegation in the checkout; the benchmark path is not fully standalone.

## Explicit non-goals for V1

- Arbitrary Rust source compatibility.
- Full general-purpose data structures, unbounded loops, dynamic allocations and general recursion.
- OpenFHE, BFV, BGV or CKKS implementations.
- Native TFHE integer/radix backend beyond the current Boolean circuit lowering.
- Automatic cross-scheme noise scheduling.
- GPU execution or an independently certified hardened cryptographic runtime.
- Universal secrecy guarantees for OS-visible inputs, decrypted outputs or timing/memory side channels.
- Validated claims of being the fastest or first FHE compiler.

## Analyzer-report interpretation

A printed `0.000 ms` latency from the **default, uncalibrated cost model** is not evidence of zero runtime or of two candidates having identical physical performance. Candidate ties under that model reflect the available estimated-cost policy. Zero predicted bootstraps/refreshes in an unknown noise profile should not be mistaken for a full cryptographic risk assessment.

A one-iteration clear-circuit benchmark verifies command execution, not a statistically robust real TFHE performance study. A hardware calibration profile may be reused when compatible; persisting a profile should store measurements, not secret keys. The previous global calibration adjustment was deliberately **rejected** because multiplication workloads regressed, even though aggregate error improved.

## Release verification priorities

- Test source -> IR -> Boolean circuit -> TFHE execution on user-facing representative examples.
- Check signed `i8` wraparound and comparisons, unsigned `u8`/`u16` bounds, Boolean conditions and nested MUX behavior.
- Check malformed input errors do not panic or disclose rejected plaintext values.
- Verify candidate correctness and selection policy under multiple plausible cost models.
- Clearly distinguish source-level clear-circuit timing, predicted cost, and measured encrypted execution latency.
- Require hosted Windows and Linux to pass before claiming cross-platform support.
