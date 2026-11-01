# FHEC Analyzer, optimizer, and cost-report interpretation

## 1. Versioning and purpose

FHEC has a planned **V1** public release. Some cost reports print `FHEC COMPILER V5.6`, and earlier technical work uses `Analyzer V5` / `V5.6`. These report-internal version labels do **not** mean the distributed Cargo crate, compiler product, or public release is version 5.6. The checked-out Cargo package versions are authoritative for the build under test.

The analyzer estimates candidate circuit costs; it is **not** a real-time profiler of every TFHE primitive. Its output is an optimization decision aid, not a cryptographic security certification or an always-correct performance predictor.

## 2. Default CLI analysis is uncalibrated

The inspected CLI `analyze <source.fhe>` constructs `V4CostModel::default()` and explicitly prints `Cost model: default (not hardware-calibrated)`. This is distinct from B-stage execution, which can load and reuse a real persistent TFHE profile. Running `profile calibrate` does not silently make `analyze` use calibrated timings in that inspected CLI implementation.

**Critical:** If a default report displays `0.000 ms`, that can be rounding or the absence of a calibrated time model. It does **not** mean that encryption, bootstrapping, execution, or decryption is instantaneous. Treat an uncalibrated latency as an illustrative modeled figure only.

The C3 verifier asserts that the current `analyze` output labels its default model uncalibrated. If the CLI evolves to load persistent profiles, change this contract and add a proof that profile provenance is printed.

## 3. Circuit candidate selection

The compiler generates structurally different circuit candidates (parallel-prefix and ripple-carry addition variants), and it may also analyze optimized variants of each. Each candidate is lowered and its circuit is checked before selected execution paths. Cost comparison uses predicted backend execution cost, not a blind minimization of gate count or circuit depth.

You may see:

- `OPTIMIZATION`: whether an optimized candidate was selected; the best baseline and predicted improvement.
- `CANDIDATES`: each candidate name, gate count, compute-gate count, depth, execution strategy, predicted latency and interval.
- `DECISION`: selected and rejected options, predicted speedup, latency delta, gate/depth differences, and a narrative reason.
- `IR COST ATTRIBUTION`: gate-work shares and which IR operation dominates estimated work.

An optimizer may increase total gates to reduce depth, or reduce gates while preserving depth. A negative gate difference does not automatically imply a faster real TFHE run. Likewise, a modeled speedup may disappear or reverse on different hardware.

The A-stage selection-invariant tests compare candidate costs and prevent choosing a candidate with a higher predicted latency under the same model. That is a **model-policy guarantee**, not proof the selected candidate is faster on all real machines.

## 4. Prediction fields

| Field | How to read it | What it does **not** prove |
| --- | --- | --- |
| `Gates` | Count of circuit gates in the candidate | Equal cryptographic work across gate types |
| `Compute gates` | Nontrivial logical gate operations | Complete wall-time profile |
| `Depth` | Longest dependency chain in the Boolean graph | Number of homomorphic bootstraps or clock cycles |
| `Runtime strategy` | Model's sequential/parallel recommendation | That thread scheduling achieves this theoretical advantage |
| `Predicted latency` | Cost-model estimate for a candidate | Actual measured execution time |
| `Prediction interval` | Model-derived uncertainty bounds | Formal statistical confidence interval with universal coverage |
| `Interval margin` | Relative model margin | Fixed error guarantee |
| `Calibration variability` | Empirical variability incorporated where available | Proof of correct calibration on every machine |
| `Interval confidence` | Analyzer confidence category | Cryptographic security confidence |
| `Interval source` | Provenance of the model/uncertainty | That a real benchmark was performed for this report |
| `Produced ciphertexts` | Modeled ciphertext objects generated | Observed allocator bytes |
| `Peak live ciphertexts` | Modeled live-object high water mark | Exact peak memory used by Rust/TFHE process |
| `Peak ciphertext memory` | Size estimate when ciphertext sizes are known | Precise RSS or GPU usage |
| `Bootstraps`, `Key switches`, `Refreshes` | Backend-cost model's declared counts | Independent instrumentation of TFHE internals |
| `Noise risk` | Abstract analyzer risk estimate | A formal decryption-failure bound for every FHE scheme |
| `IR COST ATTRIBUTION` | Relative estimated gate work per IR region | Causality or complete source-level profiling |

For a candidate selection, compare both the chosen predicted latency and the structural tradeoff. Preserve raw reports when discussing improvements, and rerun actual TFHE benchmarks to confirm empirical speedups.

## 5. Optimizer hints vs implemented actions

The analyzer generates prioritized suggestions. An optimizer plan converts some hints into executable passes; it can carry others as **unsupported** advisory actions. The distinction is essential.

The inspected optimizer plan includes:

| Action | V1 status | Qualification |
| --- | --- | --- |
| Constant folding | Implemented | Fixed-width integer folding must preserve wrapping width |
| Boolean simplification | Implemented | Includes supported Boolean/MUX identities; definition ordering matters |
| Depth reduction | Implemented in supported patterns | Not a general algebraic optimizer for arbitrary circuits |
| Parallelism exposure | Implemented in supported patterns | Benefit depends on cost model and dependencies |
| Crypto-operation reduction | Implemented in supported patterns | Only transformations proved safe for supported IR |
| Expensive-gate lowering | Implemented in supported patterns | Not universal replacement for every expensive gate |
| Memory lifetime reduction | Advisory / not implemented as a working pass in inspected plan | Do not claim automatic memory compaction |
| Synchronization reduction | Advisory / not implemented as a working pass in inspected plan | Model suggestions may have no rewrite |
| Refresh-pressure reduction | Advisory / not implemented as a working pass in inspected plan | Not a multi-scheme noise scheduler |

The A-stage work added overlap prevention, definition-order guards, duplicate-ID rejection, fixed-width folding, and exhaustively compared selected Boolean circuits. Those patches strengthen the existing optimizer but do **not** constitute a global proof of semantic equivalence for all possible programs.

Priority in an optimization plan is not a promise that every requested action is executed. If a hint is unsupported or does not match a safe rewrite pattern, the optimizer should preserve the original circuit and report the limitation.

## 6. Noise and bootstrapping

FHEC V1 uses a TFHE Boolean backend. TFHE Boolean gates generally use backend-managed bootstrapping; the analyzer's `Noise risk`, `Refreshes`, and `Max unrefreshed depth` are an **analytical model** with backend-specific assumptions. They do not simulate generic BFV/BGV/CKKS modulus consumption, nor prove a particular encryption-noise budget after arbitrary encrypted arithmetic.

No general-purpose automatic noise-budget scheduler or backend selector across OpenFHE schemes is present in the supported V1 subset. Never describe the analyzer output as a cross-scheme validated noise certificate.

## 7. Calibration-source discipline

- `profile show` displays persisted TFHE timing/hardware metadata when present and compatible.
- `profile verify` rejects missing, incompatible and invalid profiles.
- `profile calibrate` can be computationally costly and creates or replaces the local profile.
- Default `analyze` is **not** calibrated in the inspected CLI; do not imply otherwise.
- Real TFHE `run` can load an existing profile and report whether calibration was reused.
- The benchmark `compare` mode requires a usable profile; it should not invent an accurate timing model when one is missing.

Calibration samples are hardware-sensitive and imperfect. Previously observed experimental global corrections improved an average error metric but **made multiplication estimates worse**; therefore the global correction was rejected. A calibrated model remains a prediction and should be cross-validated per operation family.

## 8. How to inspect a report step by step

1. Run `fhec-cli analyze examples/conditional_mux.fhe` from the source root.
2. Confirm the report includes its cost-model provenance and function input/output bit widths.
3. Compare baseline and optimized candidates by both depth and gate count.
4. Read `Selected` and `Rejected` plus predicted cost differences. The selected predicted cost should be no larger than the rejected one up to report rounding.
5. Inspect interval source and confidence; do not interpret `0.000 ms` as execution evidence.
6. Review IR attribution to find dominant regions and to identify where hints arose.
7. For an empirical performance claim, use real TFHE timing with repeated samples on consistent hardware; clearly separate prediction from measurement.

The acceptance suite checks these report sections and selection-policy outputs without requiring a nonzero calibrated prediction from the default model.

## 9. Boundaries of final signoff

C3 covers accurate descriptions and executable-level report regressions. D-G must still validate a clean release checkout, Windows/Linux CI, dependency vulnerabilities, license list, public version, and final build artifact contents. Changes in the analyzer report format require synchronized updates to documentation and tests rather than suppressing failing assertions.
