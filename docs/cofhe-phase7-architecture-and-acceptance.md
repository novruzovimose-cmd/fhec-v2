# FHEC V2 / CoFHE Phase 7 — Differential Evidence and Receipt Corroboration

Status: engineering patch. Scope: local encrypted-operation mock differential testing and optional read-only corroboration of a separately obtained SDK trace. This patch is not a claim of production-ready FHE, an independent security audit, or measured live-network performance.

## 1. Why this phase exists

Phase 6 proved broad local agreement with a plaintext modular arithmetic oracle: four unsigned widths, six operation families, encrypted boundary cases, and signer/recipient authorization negative tests. That is a meaningful verification baseline but it is not sufficient for a grant-ready performance claim. A compiler may match a reference implementation and still produce inefficient Solidity, inaccurate cost projections, or fragile evidence. Conversely, synthetic gas can appear better than handwritten code and still be irrelevant to live FHE execution. Phase 7 separates these concerns.

The first acceptance objective is equivalence across three generated/handwritten implementations on identical mathematical input pairs. The second is conservative, independent interpretation of all emitted mock transaction receipts, including regressions. The third is an optional confirmation that transaction IDs in an existing Arbitrum Sepolia SDK trace really belong to included and successful on-chain transactions. The third objective is read-only and never sends a transaction.

## 2. Boundaries

This patch intentionally adds no Rust compiler production behavior. It exercises the existing compiler's `cofhe --optimize` path and records its output. No Solidity source is hard-coded in place of the Rust-generated optimized variant during real installation. The guarded installer performs only file additions; all preexisting Phase 6 files remain as they were. A hash mismatch in the specified Phase 6 baseline prevents installation.

The operation-family matrix is exactly:

| Operation | FHE lowering target | Oracle semantics | Output type |
|---|---|---|---|
| add | `FHE.add` | `(a + b) mod 2^w` | `euintw` |
| sub | `FHE.sub` | `(a - b) mod 2^w` | `euintw` |
| mul | `FHE.mul` | `(a * b) mod 2^w` | `euintw` |
| gt | `FHE.gt` | `a > b` unsigned | `ebool` |
| max | `FHE.select(FHE.gt(a,b),a,b)` | `max(a,b)` unsigned | `euintw` |
| repeated | `mul` then `add(product, product)` | `(2ab) mod 2^w` | `euintw` |

Here `w` is one of 8, 16, 32 and 64. The reusable Phase 6 oracle is imported by Phase 7; the expected value is never computed by the contract under test. A separate mathematical oracle is essential: agreement between two wrong Solidity implementations does not prove either implementation correct.

## 3. Comparison architecture

The Phase 7 compilation/measurement flow has five source layers:

1. User's version-pinned FHEC Rust compiler processes `examples/cofhe_phase4.fhe` and emits `FHECPhase6Optimized.sol`, exactly as Phase 6 did.
2. A deterministic manual-source generator emits `FHECPhase7ManualStraight.sol` and `FHECPhase7ManualTuned.sol`; these libraries do not invoke FHEC.
3. The Phase 6 harness template is reused to emit the three wrappers `FHECPhase6Live`, `FHECPhase7Straight`, and `FHECPhase7Tuned`. Contract names and library imports differ. Dispatch, proof validation entry points, ACL calls and storage conventions match.
4. Hardhat uses its local CoFHE plugin mocks to instantiate all three harnesses in one isolated test run. Each input pair is encrypted separately for the target consuming contract, preventing illicit reuse of a proof across contracts.
5. A standalone Node verifier checks every recorded result against the deterministic oracle, validates receipt uniqueness, and computes per-family gas and latency summaries. A separate renderer only accepts the verified summary schema.

The straight reference intentionally does not share a repeated multiplication. Its `repeated` expression invokes `FHE.mul` twice. The tuned reference hoists the product and reuses it, matching the intended optimization of the Rust compiler. Both references use the same one-call implementations for simple arithmetic and the same select expression for max. No artificially expensive reference is concealed: the straight implementation is explicitly labeled and the tuned implementation is always shown. The tuned reference may beat FHEC or tie it, and that fact must remain visible in the report.

The comparison does not mean that a handwritten implementation is a verified global optimum, nor does an equal mock gas number mean true network cost equality. It is a controlled apples-to-apples local benchmark with a narrow, inspectable set of variants. The generated wrappers all include on-chain proof checking and sender-scoped FHE grants; these costs dominate many small operations and should not be silently subtracted from one variant only.

## 4. Source and artifact isolation

The installation path is `C:\Users\Moska\Documents\fhec-v2`. The user's shell may be in `Documents\fhec`; this is intentional. The installer requires the last directory component to be exactly `fhec-v2` and the Cargo workspace to exist. It verifies SHA-256 digests for the six reviewed Phase 6 baseline paths before writing anything. Every intended additive file is listed in a manifest and checked for SHA-256 and byte size. Existing Phase 7 files are accepted only when their hashes are identical. Any foreign or modified existing file stops installation.

Each verification run allocates a unique ID and writes:

- one result directory under `target/cofhe-phase7-<id>`;
- one generated Solidity directory under `integration/cofhe-hardhat/phase7/generated/<id>`.

The Hardhat config receives the exact run-specific source directory through `FHEC_PHASE7_SOURCES_DIR`. The test receives an exact matrix path through `FHEC_PHASE7_MATRIX_FILE` and an exact evidence output path through `FHEC_PHASE7_MOCK_EVIDENCE_FILE`. The verifier saves and restores all three process-local environment values in a `finally` block. No generated source, prior test, historical run or user edit needs to be overwritten to rerun Phase 7. The current run's generated source is retained so it can be examined alongside the report.

Phase 7 deliberately does not run the Phase 6 live SDK script. The optional `-VerifyLiveTrace` value must point to a preexisting, separately authorized Phase 6 trace, and `FHEC_TESTNET_RPC_URL` must be HTTPS. That action performs read-only RPC calls (`eth_chainId`, `eth_blockNumber`, `eth_getTransactionByHash`, `eth_getTransactionReceipt`, `eth_getCode`). No private key is read, injected, requested or logged by Phase 7.

## 5. Golden corpus profiles

Two canonical, pre-generated JSON corpora ship with the patch:

| Profile | Cases per family | Families | Total oracle cases | Purpose |
|---|---:|---:|---:|---|
| Small default | 2 | 24 | 48 | rapid verify/install feedback |
| Benchmark fixture | 16 | 24 | 384 | complete boundary-pair coverage per family |
| Stress fixture | 28 | 24 | 672 | all 16 boundary pairs plus 12 deterministic seeded pairs |

The benchmark and stress JSONs are independent fixed fixtures, not merely promised future commands. `verify-fixtures.mjs` reconstructs them from the Phase 6 oracle and recomputes every expected result. The installer verifier checks both before any compiler invocation. For actual Hardhat execution, `-CasesPerFamily 2`, `16`, or `28` regenerates and consumes the same canonical case order; a 672-case three-way run performs 2,016 mock transactions before additional negative tests.

The random-looking cases are deterministic pseudorandom inputs; they do not constitute adversarial fuzzing or a claim of cryptographic unpredictability. The test suite includes zero, maximum width, wraparound, equal operands, cross-half comparisons and nonboundary pairs. If a new operation or encrypted type is added later, the family inventory and schema must be upgraded rather than silently modifying this historical corpus.

## 6. Measurement definition

Each successful run case yields exactly three Solidity harness transactions plus decryption checks. The raw evidence records the transaction hash, receipt gas used and wall clock milliseconds for each variant. The timer begins before the transaction submission and ends after the local proof of authorization and decrypted result. Therefore wall-clock time includes Hardhat, the mock implementation, receipt waiting and SDK view-decryption; it is not isolated cryptographic latency.

The independent validator requires 24 complete families, an exact matrix digest, a case-by-case oracle value match, three variants per case, safe decimal-string gas/latency measurements, distinct transaction hashes, and no missing records. JSON numeric precision cannot silently corrupt measurements because receipt gas, milliseconds and large integers are parsed as strings and bounded to a safe integer scale. Boolean expected values are normalized only at the result comparison boundary; the actual oracle matrix remains typed.

For each family, it reports min, median, nearest-rank p95 and max gas per variant, plus median and nearest-rank p95 local elapsed milliseconds. Relative savings are measured as signed basis points between medians. Positive savings mean the compiler-generated variant consumed less local mock gas. Negative savings mean a regression; negative results are never clamped to zero. The benchmark HTML publishes all 24 families, including losses. An aggregate win count is supplementary, not a replacement for complete family rows.

Proof of measurement is different from proof of optimization. A large apparent reduction in local mock gas may reflect the plugin mock's instrumentation. Phase 7 must not describe a mock p95 as network latency, must not reinterpret a local gas unit as coprocessor fees and must not claim a bounded operational cost merely because a static FHEC budget check passed.

## 7. Optional live RPC corroboration

`verify-receipts.mjs` validates a Phase 6 live SDK trace before contacting the RPC provider. It requires Arbitrum Sepolia chain ID 421614, successful deployment and execution receipts, matching transaction hashes, signer identity, destination contract, consistent block metadata, nonzero gas and an adjustable confirmation depth. It also requires nonempty deployed runtime bytecode and rejects a transaction reused as another result.

The resulting evidence tier is deliberately long and explicit: `testnet-transport-receipt-verified-sdk-decryption-not-independently-attested`. RPC corroboration supports transaction inclusion and the reported gas figures. It does not inspect an independently trusted FHE coprocessor trace, prove the confidentiality of inputs, establish a cryptographic statement about decrypted results or grant mainnet deployment assurance. Use two independent RPC providers if presenting this evidence externally, and preserve their original response files and timestamps outside this patch.

No trusted off-chain prover, price oracle, task fee receipt or independent recipient decryption is implemented. Those are future research and system-integration tasks, not accomplishments of Phase 7.

## 8. Evidence locations and provenance

A successful local run leaves:

| Artifact | Meaning |
|---|---|
| `three-way-mock-raw.json` | local Hardhat mock observations |
| `three-way-verified-summary.json` | independent oracle and receipt audit, including 24 families |
| `phase7-mock-evidence-report.html` | static self-contained report; opening it has no external requests |
| `fhec-optimized-static-report.json` | FHEC static estimator output, not gas measured on-chain |
| `provenance.json` | hash ledger and scope labels for this run |
| `read-only-testnet-rpc-corroboration.json` | present only if optional read-only RPC succeeded |
| `phase7/generated/<id>/` | actual Rust-generated source, manual references and matrix used |

The two evidence tiers must be shown separately. If an optional RPC call fails, the independent report is not updated with a misleading “passed live” indicator. The local mock evidence still exists and can be inspected, but the verifier returns a failure and no all-green acceptance banner.

## 9. Acceptance criteria

A Phase 7 local acceptance result requires all of the following in one run:

- Baseline Phase 6 digests and additive patch manifest verified before installation.
- Unit tests covering Solidity generator, golden fixture reconstruction, mock metrics, HTML report and forged read-only RPC response behavior passed.
- Rust CoFHE formatting, tests and Clippy passed unless the user explicitly opted into `-SkipRustTests`; Rust compilation of optimized Solidity is always required.
- `FHECPhase6Optimized.sol` generated by actual `cargo run -p fhec-cli` with `--optimize` and used by Hardhat.
- Manual straight and tuned libraries generated independently and compiled alongside their identical harnesses.
- Exactly 24 operation-width families completed with the requested number of cases per family.
- Three decrypted results per case agreed with the independent modular arithmetic oracle.
- Three authorization and sender-isolation regressions were explicitly rejected in the local mocks.
- Independent Node audit accepted all unique mock receipts and retained per-family positive and negative deltas.
- A report with clear `LOCAL MOCKS ONLY` marking was generated successfully.
- A provenance ledger with source and evidence SHA-256 digests was written without a UTF-8 BOM.

The following claims do **not** follow from acceptance: fully complete V2, a globally optimal CoFHE backend, verified mainnet confidentiality, cryptographically attested computation, live gas reductions, independent funding readiness, or deployment security. No engineering process can honestly guarantee “undisputed 100%” solely from a passing finite test matrix.

## 10. Windows commands

From PowerShell, change to the user's requested working directory first, but target the actual V2 workspace explicitly:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
$v2 = "$env:USERPROFILE\Documents\fhec-v2"
& "$v2\tools\cofhe\verify-phase7.ps1" -Project $v2 -CasesPerFamily 2 -SkipDependencyInstall
```

For the full 384-case benchmark, use `-CasesPerFamily 16`; for the 672-case stress corpus, use `-CasesPerFamily 28`. These increase test runtime and Hardhat local workload substantially. Start with the small smoke run and inspect the full report before paying for a longer test run.

For optional read-only transaction corroboration, configure a trusted HTTPS Arbitrum Sepolia RPC endpoint in the local shell and pass a previously obtained Phase 6 live trace path. Phase 7 never needs the Phase 6 private-key environment variable:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
$v2 = "$env:USERPROFILE\Documents\fhec-v2"
$env:FHEC_TESTNET_RPC_URL = "https://YOUR_TRUSTED_ARBITRUM_SEPOLIA_RPC_ENDPOINT"
& "$v2\tools\cofhe\verify-phase7.ps1" -Project $v2 -CasesPerFamily 2 -SkipDependencyInstall -VerifyLiveTrace "C:\ABSOLUTE\PATH\TO\YOUR\PHASE6\provisional-live-sdk-trace.json" -MinimumConfirmations 2
```

Do not put testnet private keys or RPC credentials into evidence attachments. If the HTTPS URL contains an API key, redact it before sharing logs. Testnet use may incur cost even though this particular receipt corroboration only reads public chain state.

## 11. Engineering decisions deliberately deferred

The next phase should not optimize for “more files” without a practical research objective. The highest-value uncompleted work is a real testnet measurement baseline with independent external replication, contract source verification and bytecode reproducibility, account lifecycle and access-control review, formal type/soundness invariants for the emitted FHE IR, adversarial parser/optimizer fuzzing, and more robust compiler optimizer profitability data. These are distinct tracks that can be accepted independently.

A measured performance win suitable for a grant proposal should identify the exact compiler commit, target FHE contract version, library and coprocessor versions, hardware or chain environment, workload, number of repetitions, comparison baseline, result distributions and costs, and documented uncertainty. If a benchmark regresses, publish the regression and prioritize correctness or workload-specific optimizer work. This transparency is more credible than a percentage estimate without reproducible measurements.
