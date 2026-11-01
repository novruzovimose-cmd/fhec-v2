# FHEC V1 complete guide - release-candidate documentation

> **Status:** This is the documentation and analyzer-signoff guide for the **planned FHEC V1**. The source has passed substantial local regression checks, but no final clean Windows/Linux CI and public release are claimed. Some older documentation calls an internal analyzer report `V5.6`; that is not the Cargo package/release version. This guide deliberately avoids describing FHEC as a full Rust compiler, a universal FHE frontend, or an independently certified cryptosystem.

## 1. Executive technical overview

FHEC (FHE-Native Circuit Compiler) is a Rust workspace for describing selected typed operations on encrypted values and lowering them into Boolean circuits suitable for TFHE-backed execution. A user writes a `.fhe` source file, checks it with `build`, inspects candidates with `analyze`, and invokes `run` to encrypt inputs, evaluate a circuit with real TFHE Boolean operations, and decrypt an output. Benchmark utilities compare circuit construction, unencrypted Boolean evaluation and encrypted execution, while an analyzer estimates depth, gate counts, latency, memory, cryptographic primitive work, noise-pressure risk and uncertainty.

The engineering value lies in the **complete stack**, not a claim to have invented TFHE or fully homomorphic encryption: source parsing, FHE-aware type rules, typed IR, circuit lowering, optimizer candidate generation, choice under a cost model, execution strategies, hardware profiles and diagnostics. The supported language remains deliberately narrow enough to test.

### What works in the verified local flow

The earlier C2 run successfully demonstrated all six executable CLI contract tests, Clippy with warnings denied, release CLI compilation, direct executable help, a clear-circuit benchmark command covering eight workloads, and **real encrypted execution** of `pick_greater` with public inputs `37` and `5`, producing decrypted output `37`. C1 provided a full workspace test pass with real TFHE regression tests, Clippy on all targets and release-mode builds. Stages A and B added correctness and profile/runtime hardening. These are **local results**, not certification or cross-platform release validation.

### What is explicitly outside V1

No arbitrary Rust compatibility; no loops, arrays or structs; no unsupported Boolean source operators such as `&&`/`||`/`^`; no BFV/BGV/CKKS/OpenFHE backend; no GPU; no multi-party key custody; no automated general FHE scheme choice; no independently proven noise schedule for other schemes; no network service; no formal zero-knowledge framework; and no universal performance predictor.

## 2. Reader map

- Want syntax and exact supported combinations? [Language reference](LANGUAGE_REFERENCE.md).
- Want to understand optimization and cost reports? [Analyzer and optimizer](ANALYZER_AND_OPTIMIZER.md).
- Want to publish benchmark numbers responsibly? [Benchmarks and calibration](BENCHMARKS_AND_CALIBRATION.md).
- Want to assess threat model and key exposure? [Security and runtime](SECURITY_AND_RUNTIME.md).
- Need to install on Windows or Linux? [Developer installation and troubleshooting](DEVELOPER_INSTALL_AND_TROUBLESHOOT.md).
- Need subsystem data flow? [Architecture and data flow](ARCHITECTURE_AND_DATA_FLOW.md).
- Need release checkboxes? [C3 acceptance and remaining](C3_ACCEPTANCE_AND_REMAINING.md).

The repository also has historical design documents and C1/C2/B3 acceptance notes. Treat an older plan's description of a future optimization as an **aspiration**, not a working feature, unless the current code/tests verify it.

## 3. V1 supported type contract

The executable CLI accepts functions with supported encrypted inputs and outputs including `encrypted<u8>`, `encrypted<i8>`, `encrypted<u16>` and `encrypted<bool>`. Supported operations include integer addition, subtraction and multiplication in tested width combinations; comparisons returning encrypted Boolean results; typed function calls; and encrypted conditionals that lower to MUX. Neither the existence of an AST type variant nor the parser's ability to lex a token establishes executable support for every expression.

Source types such as `u32`, `i64` or `plaintext<T>` may be syntactically representable but should not be advertised as generally executable TFHE input types without tests on the actual backend. A type checker can reject incompatible encrypted/plaintext arithmetic or comparison. Source-level division or Boolean `&&` are not supported just because the IR has Boolean gates.

### Minimal source example

```fhe
fn pick_greater(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> {
    return if a > b {
        a
    } else {
        b
    };
}
```

This source demonstrates a homomorphic selection: the encrypted Boolean produced by `a > b` controls a circuit MUX. The compiler does not decrypt that Boolean in order to choose a branch. The local client decrypts only the chosen output upon request.

### Fixed-width arithmetic behavior

Within supported width/type combinations, an encrypted integer is a vector of bits. An 8-bit result must stay within 8 bits, so arithmetic wraps when its mathematical result exceeds the representable range. Signed 8-bit values use two's-complement interpretation. Constant folding must honor these same semantics. The A3 regression suite checked wrapping integer behavior and prevented optimizer rewrites from using definitions introduced later in the instruction sequence.

### Function calls and conditionals

Calls can be inlined through supported statically known functions; forward references and nested calls have passing tests. Recursion is rejected. Branch results in an `if` expression must have compatible types. Unsupported bare/partial return paths should fail meaningfully rather than silently produce malformed circuitry. Nested encrypted operations are possible, but pathological circuit size can make actual TFHE execution expensive.

## 4. Work from source to result

From the source checkout root, after a release build:

```powershell
cargo build --release --locked -p fhec-cli
.\target\release\fhec-cli.exe --help
.\target\release\fhec-cli.exe build examples\conditional_mux.fhe
.\target\release\fhec-cli.exe analyze examples\conditional_mux.fhe
'37,5' | .\target\release\fhec-cli.exe run examples\conditional_mux.fhe --function pick_greater --inputs-stdin
```

On Linux use `./target/release/fhec-cli` and slash-separated paths. If the workspace has `CARGO_TARGET_DIR`, use that configured target directory. The executable is named `fhec-cli` rather than automatically receiving the alias `fhec` that appears in help text.

**Build:** lex, parse, lower, assemble and verify in memory. In the inspected implementation it does not write a reusable output circuit file. Avoid implying that `build` produces a distributable compiled artifact.

**Analyze:** produces a candidate comparison and structural/cost report from the *default non-hardware-calibrated model* in the CLI. The output is not direct TFHE execution timing.

**Run:** validates typed inputs, encrypts them, executes the Boolean circuit with TFHE and intentionally prints a decrypted result. Stage B2 supports `--inputs-stdin` to avoid command-line argument exposure, but local OS tooling and the output itself can still reveal data.

**Bench:** currently runs Cargo internally to invoke the benchmark package. It requires the project checkout and development environment even if the CLI binary was copied elsewhere.

**Profile:** examines or generates a hardware-specific performance profile, normally at `target/fhec-profiles/tfhe-v4.json` relative to the working directory.

## 5. Parsing, type checking and IR

The frontend's lexer recognizes function declarations, typed parameters, `let`, `return`, `if`/`else`, integer/Boolean literals, calls, arithmetic and comparison tokens. The parser creates expressions including binary arithmetic/comparison, call and conditional nodes. Type checking verifies operator compatibility, function parameters and returns, identifier resolution and encrypted/plaintext constraints.

The compiler's typed IR acts as a boundary between source-level meaning and circuit-level instruction scheduling. Correctness requires that each IR operand be defined before its use, that values are not multiply redefined in a way that breaks SSA-like assumptions, and that the selected circuit has valid outputs and wires. A-stage patches addressed duplicate value definitions, overlapping Boolean rewrites, dominance, signed wrapping and candidate selection regressions. These tests matter because a circuit can have valid syntax but represent the **wrong** function if an optimization is unsound.

From IR, the circuit builder assembles Boolean gates and per-bit representations for integers, comparisons and MUX selection. Alternative adders give it choices between ripple-carry and parallel-prefix implementations. The analyzer compares predicted cost under a declared model; the selected variant should not have higher predicted cost than a rejected variant under that same comparison policy.

## 6. Optimizer: an honest capability statement

Implemented, tested transformations include constant folding, Boolean simplification and supported patterns of depth reduction, parallelism exposure, expensive-gate lowering and cryptographic-operation reduction. A pass must preserve semantics; safe patterns cannot be generalized to arbitrary circuit DAGs without proof. The optimizer also receives analyzer hints for memory-lifetime, synchronization and refresh-pressure issues. Some are **advisory only** in the inspected code. A hint existing in the report does not mean it ran and improved the circuit.

The optimization plan may retain the original candidate if an attempted rewrite has no safe implementation, violates order, produces a bad circuit or would lose according to modeled cost. A model-policy safety check is not an assurance of speed on every real CPU. Actual performance measurements must decide whether a transformation offers empirical benefit on the target workload and configuration.

## 7. Cost report anatomy

The analyzer's main sections (in the inspected V1 implementation) include `OPTIMIZATION`, `CANDIDATES`, `DECISION` and `IR COST ATTRIBUTION`. Individual candidate fields include gates, compute gates, depth, proposed runtime strategy, predicted latency, prediction interval and confidence/source, produced and peak live ciphertext estimates, crypto-operation counts, noise behavior and hints.

Use a candidate report in this sequence:

1. Verify function input/output bit widths match the expected source signature.
2. Locate **cost model provenance**. `default (not hardware-calibrated)` means the output is illustrative and not tied to measured TFHE operations.
3. Compare baseline and optimized candidate gate counts and depths. A candidate can be shallower but larger, or smaller but deeper.
4. Read selected vs rejected **predicted** latency. Compare consistently in ns or ms and allow for display rounding when values are tiny.
5. Inspect the interval source and confidence. An interval represents model uncertainty, not a formal confidence bound on a cryptographic secret or any actual runtime.
6. Review IR cost attribution and hints as research targets, not definite bottlenecks proven by sampling.
7. Confirm a real speedup with repeated TFHE benchmarks if performance matters.

### The `0.000 ms` trap

The default model can display a rounded zero-millisecond predicted figure. This should be read as **no meaningful calibrated timing estimate at the printed precision**, not literal instantaneous encrypted execution. FHEC's crypto backend is computationally nontrivial. The C3 verifier checks the uncalibrated provenance line, and this guide deliberately separates such predictions from TFHE benchmark measurements.

### Analyzer V5/V5.6 naming

Some generated reports use `FHEC COMPILER V5.6`. This label originates in the analyzer/compiler research reporting code. The Rust manifests can still be `0.1.0` and the planned public release name V1. Release engineering must choose consistent user-visible versioning before publishing; analyzer display version strings should not be used as the release tag or crate version without a deliberate versioning change.

## 8. Calibration, profiles and model accuracy

TFHE Boolean-gate costs depend strongly on CPU, concurrency and backend parameters. FHEC persists sampled hardware timing metadata so later executions can reuse a valid calibration. The profile includes backend/hardware metadata, declared ciphertext sizes, gate measurements and scaling data. It should not serialize a client/server secret key. B-stage tests checked profile width bounds, invalid/outdated/incompatible profiles, oversized JSON rejection and safer persistence.

On Windows, profile replacement uses a backup strategy that is **not fully crash-atomic**. In the event of a power interruption a backup can remain and manual recovery/recalibration may be necessary. Profiles should be stored in a trusted, single-writer directory. A profile is performance data, not an authenticated cryptographic policy.

The previous calibration study produced a crucial negative result: a global correction improved overall average prediction error but worsened multiplication prediction. The calibration adoption guard rejected the global correction rather than promoting a misleading universal improvement. The FHEC user guide must report that limitation plainly.

An uncalibrated analysis is not fixed by simply having an old profile file in `target/`; the inspected `analyze` handler explicitly constructs a default model. Real TFHE `run` can load existing valid profile information and report reuse. These are different code paths.

## 9. Measuring performance responsibly

The benchmark package has multiple binaries and must be invoked with `--bin fhec-bench` for direct Cargo usage. Default mode times compilation and clear Boolean evaluation (not TFHE); other modes measure real encrypted TFHE operations, optimizer before/after behavior, or candidate comparisons against a persisted profile.

```powershell
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --iterations 30 --output target/bench-clear
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode tfhe --iterations 3 --output target/bench-tfhe
cargo run --release --locked -p fhec-bench --bin fhec-bench -- --mode optimizer --iterations 3 --output target/bench-opt
```

For a trustworthy performance claim, record CPU, OS, Rust/TFHE versions, sample counts, warmups, input widths, cost-model provenance, selected candidate, per-workload medians and dispersion. Use the same host and setup for baseline comparisons. Do not extrapolate from one iteration or conflate a 1,800 ns *clear-circuit* result with 1,800 ns *encrypted TFHE* performance.

C1 verified the benchmark's even/odd median handling and enriched MUX/Boolean workloads. C2 confirmed a one-iteration clear-circuit CLI smoke run wrote CSV/JSON reports. C3's checks emphasize documentation accuracy; large statistical benchmark campaigns remain independent evidence.

## 10. TFHE runtime and security boundaries

The TFHE backend performs real encryption, Boolean gate operations and decryption. The backend's cryptographic code and client/server keys run within the same local environment in the supported V1 workflow. A local host compromise can expose secrets. There is no remote isolation guarantee, zeroized secure heap, encrypted private key store or authenticated metadata service implemented by these patches.

Encrypted `if` maps to circuit MUX; that means evaluation does not decrypt the condition just to choose a branch. B3 tested a TFHE MUX truth table. However, code correctness tests cannot prove constant-time behavior of the operating system, CPU, memory allocator, thread scheduler and all dependencies under every threat model.

Error handling should prevent rejected plaintext values from appearing in CLI diagnostics. The `--inputs-stdin` interface is an exposure reduction, not a secret-channel guarantee; terminal history, process memory, host telemetry and decrypted outputs are separate concerns. Do not place real personal input values in release logs.

The analyzer's `Bootstraps`, `Refreshes` and `Noise risk` describe a model of TFHE-backed Boolean operations; FHEC does not implement an automatic generic CKKS/BFV/BGV noise-budget scheduler. Documentation that suggests otherwise would be scientifically inaccurate.

## 11. Test strategy, fast and expensive

FHEC needs both quick deterministic checks and real encrypted integration coverage. A test that evaluates a Boolean circuit without encryption can catch circuit logic failures, but it does not prove TFHE key generation, ciphertext interoperability or decrypt correctness. Conversely, one successful encrypted `37,5` demo cannot prove all widths and operators.

**Fast checks:** `cargo fmt --all -- --check`, `cargo check --workspace --locked`, focused compiler/optimizer/analyzer CLI tests, example `build`, example `analyze`, C2 contract tests, and Clippy all targets with `-D warnings`.

**Full checks:** `cargo test --workspace --locked`, including real TFHE integration matrices; release compilation and direct binary checks. Real TFHE arithmetic boundaries can take more than one minute each on some hardware. The verifier must print test output live rather than silently wait on a redirected child process.

**External release checks (D-G):** fresh clone without hidden local files; Linux and Windows CI; exact commit-level verification; dependency/license audit; secret scanning; archive and license inspection; final changelog/version agreement; tag and public release. These checks are not replaced by a successful C3 local test run.

## 12. Practical commands reference

| Objective | From source root |
| --- | --- |
| CLI help | `cargo run --release --locked -p fhec-cli -- --help` |
| Verify source | `cargo run --release --locked -p fhec-cli -- build examples/conditional_mux.fhe` |
| Analyze source | `cargo run --release --locked -p fhec-cli -- analyze examples/conditional_mux.fhe` |
| Real TFHE demo | `cargo run --release --locked -p fhec-cli -- run examples/conditional_mux.fhe --function pick_greater --inputs 37,5` |
| Check hardware profile | `cargo run --release --locked -p fhec-cli -- profile verify` |
| Calibrate hardware | `cargo run --release --locked -p fhec-cli -- profile calibrate` |
| Clear benchmark | `cargo run --release --locked -p fhec-bench --bin fhec-bench -- --iterations 30` |
| Full workspace tests | `cargo test --workspace --locked` |
| Full Clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Build release executables | `cargo build --release --locked -p fhec-cli -p fhec-bench --bins` |

Call `fhec-cli.exe` on Windows or `fhec-cli` on Linux when launching the binary directly. Don't assume the program is installed globally as `fhec`.

## 13. Common developer pitfalls

**Problem:** Cargo cannot find the native linker on Windows. **Action:** Install correct MSVC C++ tools and use a configured developer environment; verify `rustc`/`cargo` version first.

**Problem:** `cargo run -p fhec-bench` cannot pick a binary. **Action:** Add `--bin fhec-bench`.

**Problem:** `profile verify` fails after changing machines. **Action:** Hardware compatibility is checked; recalibrate locally rather than trusting a copied profile.

**Problem:** `analyze` displays zero milliseconds. **Action:** Check the uncalibrated cost-model label; use real TFHE benchmarking for timing.

**Problem:** An encrypted input value appears to be missing. **Action:** Check the selected function signature and comma-separated input count; typed input decoding is strict.

**Problem:** PowerShell reports `elseif` as unknown. **Action:** Run a self-contained `.ps1` instead of executing separated interactive control-flow fragments.

**Problem:** Tests run silently. **Action:** Use direct Cargo execution with live output; the B/C verifier scripts are designed not to hide long-running TFHE tests.

**Problem:** A patch applies to an old source snapshot but fails on current files. **Action:** Do not force a textual patch; inspect the current file, use strict preflight and preserve backups. C3 avoids editing Rust source altogether.

**Problem:** A benchmark shows a dramatic speedup from one input pair. **Action:** Evaluate multiple widths/workloads, sufficient repeats and held-out samples; don't report one-iteration smoke results as a stable result.

## 14. Stage C3 acceptance contract

The C3 installer only adds this documentation set and verified examples plus the reusable CLI-analyzer verifier. It checks destination collisions and SHA-256 file digests before writing. It doesn't overwrite the user's Rust code, change dependencies or call Git.

The full verifier confirms the installed file hashes, builds a release CLI, builds and analyzes C3 example programs, checks the analyzer model label and report sections, checks candidate selection under the reported cost policy, runs C2 integration tests, focused crate regressions, complete workspace tests, all-target Clippy and release builds. A `-FastOnly` mode skips the full workspace cryptographic test suite and prints a different success marker, so it cannot be confused with a completed C3 gate.

No cross-platform public-release conclusion follows automatically. The final release gate still requires the exact Git commit to be tested from a clean clone in Windows and Linux and verified against the published archive.

## 15. V1 publication principles

1. Publish only supported, tested features; label unavailable constructs explicitly.
2. Separate clear-circuit performance from real TFHE performance and modeled latency.
3. Do not describe Analyzer V5.6 as the public release version if manifests/tag say otherwise.
4. Treat negative calibration results as evidence, not a problem to hide.
5. Keep keys and real decrypted data out of release examples, logs, archives and benchmarks.
6. Do not suppress Clippy warnings or bypass failed regression tests in order to reach a release marker.
7. Require a new contributor to build, analyze and execute the documented examples without editing library source files.
8. Check Windows and Linux from clean environments; one successful developer machine is not a substitute.
9. Verify dependency advisories and license inventory before posting any security-related guarantee.
10. Create Git commits and publish **after**, not before, completing the user's agreed finalization checklist.

## 16. Evidence ledger for the final release review

Maintain distinct proof for each of the following rather than using a single undifferentiated "tests passed" line:

- Compiler grammar/operator matrix: frontend, lowering and encrypted behavior tests.
- Optimizer semantics: exhaustive tiny truth tables, randomized differential comparisons, malformed IR tests and model-choice invariants.
- Runtime error paths: injected failures, incorrect input widths, invalid output references, profile incompatibility and corrupted JSON.
- Real encrypted evaluation: all supported primitive families, MUX, nested calls, signed/unsigned boundaries and arithmetic wrap.
- Analyzer: report labels, candidate comparisons, interval provenance, memory units and uncalibrated behavior.
- Performance: raw per-workload measurements with repetitions, baseline and profile provenance.
- CLI: binary name, command success and failure exit codes, redaction, stdin behavior and direct executable use.
- Documentation: working code examples, installed toolchain requirements, known limitations, version agreement and changelog.
- Security/release: dependency advisory/license reviews, secret scans, clean-clone evidence, two-OS CI and public archive inspection.

C3 contributes documentation and report signoff evidence. The other rows are not waived; keep unresolved release items open until their own evidence exists.

## 17. Maintenance policy

When a source-level API or report field changes, update the corresponding document and acceptance test in the **same change**. When a new optimizer action becomes genuinely executable, change its status from advisory only after passing equivalence and cost-policy tests. When a default analyzer starts using hardware calibration, revise the explicit uncalibrated label and add a trustworthy profile provenance line. When a compiled artifact writer is implemented, specify the file format, versioning and verifier contract before stating that `build` emits artifacts.

For future improvements, prefer source-grounded, bounded patches; do not assume an old ZIP still exactly matches the user's current working tree. Refuse ambiguous or destructive edits, back up affected paths before writing, and run the original regression tests with warnings denied.

**Bottom line:** FHEC V1 is an ambitious but scoped TFHE-backed compiler with a demonstrably working local workflow. The documentation must make its boundaries as reproducible as its strengths. Stage C3 is one milestone; the remaining D-G release engineering is the last step before a genuinely finalized public V1.
