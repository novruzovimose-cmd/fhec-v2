# FHEC V2 / Phase 7 — Detailed Test Plan and Reproduction Contract

## Purpose

This document is the test contract for the Phase 7 CoFHE differential patch. It names the inputs, boundaries, commands, failure cases and evidence files required for a technically defensible local verification. A positive result describes one pinned project state and one local mock execution. It does not certify all FHE programs or all future versions of the plugin.

## Preparation and prerequisites

The target workspace is a complete Rust Cargo workspace at `Documents/fhec-v2`, with the previously validated CoFHE CLI, the Phase 6 oracle and Solidity generator, the Hardhat 2 CoFHE integration and a frozen `pnpm-lock.yaml`. PowerShell should have access to Node, pnpm, cargo, rustc and the Windows C++ linker needed for native Rust dependencies. Exact Phase 6 baseline file hashes are checked by the Phase 7 installer.

Before starting a full run, capture these version outputs as part of the public reproducibility record:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
$v2 = "$env:USERPROFILE\Documents\fhec-v2"
git -C $v2 status --short
git -C $v2 rev-parse HEAD
node --version
pnpm --version
cargo --version
rustc --version
pnpm --dir "$v2\integration\cofhe-hardhat" list @cofhe/sdk @cofhe/hardhat-plugin @fhenixprotocol/cofhe-contracts --depth 0
```

A failing or untracked Git state does not automatically prove the compiler is wrong, but it must be disclosed when presenting the measurements to a third party. The installer never removes existing user modifications, and a baseline mismatch is a stop condition rather than a prompt to overwrite a source file.

## Verification matrix

The test universe is the Cartesian product of:

- four unsigned types (`euint8`, `euint16`, `euint32`, `euint64`);
- six operations (`add`, `sub`, `mul`, `gt`, `max`, `repeated`);
- two, sixteen or twenty-eight canonical input pairs;
- three implementations (actual Rust-compiled optimized library, handwritten straightforward, handwritten tuned).

For `N` cases per family, the expected totals are exactly `24 × N` oracle cases and `72 × N` successful mock transactions. Three additional negative authorization checks must pass. The default N=2 gives 48 oracle cases and 144 successful mock transactions. N=16 gives 384 cases and 1,152 transactions. N=28 gives 672 cases and 2,016 transactions. The tests run three contracts serially per oracle case to avoid conflating concurrent Hardhat nonce and state changes with performance.

The first sixteen pair positions per width are deterministic boundaries: zero, maximum value, symmetric maxima, near-maximum, one-sided wraparound, half-range crossing, equal halves, small integer multiplication and mirrored pairs. Positions sixteen through twenty-seven are generated using a fixed 64-bit linear congruential sequence and stored in the stress corpus. This is an intentionally reproducible pseudorandom supplement, not coverage of every possible 64-bit integer input.

### Test design by family

| Family | Primary correctness concern | Principal boundary stress | Expected reference behavior |
|---|---|---|---|
| `add` | unsigned modular wraparound | `MAX + 1`, `MAX + MAX` | same modulus for all variants |
| `sub` | underflow under unsigned interpretation | `0 - MAX`, `0 - 1` | modular wraparound, not a Solidity unsigned revert |
| `mul` | high bits discarded under width | `MAX × MAX`, half-range products | low `w` bits only |
| `gt` | correct unsigned ordering and boolean type | equal inputs, half-range reversal | canonical boolean oracle |
| `max` | encrypted select semantics | equal inputs, reversed operands | returns one of the original operands |
| `repeated` | optimization preserves common subexpression | products requiring wraparound | `2ab mod 2^w` in all variants |

### Independent golden corpus checks

`verify-fixtures.mjs` checks that the 384- and 672-case immutable JSON fixtures are bytewise equivalent after parsing to the independently regenerated Phase 6 oracle data. It also recomputes every result using unsigned BigInt arithmetic, enforces 24 complete operation-width families, rejects duplicate IDs and verifies the count of boundaries versus seeded cases. The test suite includes counterfeit expected values and tampered scope labels. These files are intended to remain stable until a documented schema migration, not rewritten on every compiler update.

## Test categories

### A. Solidity generator and library parity

The generator unit suite confirms all 24 functions exist in both manual libraries, all three wrappers are obtained from one source template, the correct imported library name is used, ACL calls and operation dispatch match after name normalization, and unrecognized pairings are rejected. The straight `repeated` implementation must contain exactly two multiplication invocations; the tuned one must share a single product. The generator creates new files only (`wx`) and cannot silently overwrite a previous output.

This category checks source-template equivalence. It cannot independently prove generated bytecode equivalence, which would be an incorrect expectation because different libraries intentionally use different operations and compiler optimization can alter assembly. The Solidity compiler's own optimizer settings remain pinned by the existing Hardhat config.

### B. Rust compiler compatibility

The verifier runs `cargo fmt --all -- --check`, then `cargo test --locked -p fhec-backend-cofhe -p fhec-cli`, then `cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings`, unless `-SkipRustTests` was explicitly requested. It always invokes `cargo run --locked -p fhec-cli -- cofhe ... --optimize`. Skipping the regression suite is never equivalent to skipping actual code generation.

The Solidity source must be recompiled from the user's current V2 codebase. A pre-shipped sample `.sol` is not accepted as evidence of current Rust behavior. The source hash is saved in `provenance.json`; the accompanying static optimizer report cannot be substituted for a transaction gas measurement.

### C. Mock three-way encrypted execution

The Hardhat suite deploys CoFHE mocks and three harnesses. For every pair, it creates separately signed encrypted inputs for each harness address and sends the width-specific operation selector. Every transaction must succeed and produce an on-chain receipt. A stored encrypted handle is read back under the originating sender's account, and the SDK mock decryption must match the BigInt oracle's decimal string or boolean value.

The primary test case is reported only after all three variants pass. The independent validator requires the exact expected case count, complete triples and distinct receipt hashes. A transaction ID cannot appear in more than one variant or test case. A valid metric is represented as a decimal string; `NaN`, negative or unreasonably large precision-loss-prone values are rejected.

### D. Permission isolation

The suite attempts to submit a valid ciphertext proof to a different consuming contract, to use a ciphertext signed for a different sender, and to have an unrelated account decrypt a previously stored encrypted value. The first two attempts must revert. The last must be denied by the local CoFHE mock ACL. The permission tests operate on distinct contracts from the normal tests and are not counted as successful differential transactions.

These are regression checks for the integration's intended ACL and signature behavior. A mocked ACL can differ from a live environment; results do not replace a threat-model-driven contract review or a third-party audit.

### E. Independent post-execution evidence audit

The raw evidence is not trusted merely because it came from a test that passed. A separate Node process validates matrix SHA-256, exact case identity, canonical operation and width, expected plaintext-oracle result, three named variant results, gas and elapsed decimal strings, exact transaction count and receipt hash uniqueness. It calculates median and nearest-rank p95 using BigInt, with documented rounding for an even-length median. It never silently drops a family that lost against either reference.

The unit tests for this auditor inject forged expectations, missing rows, duplicated hashes, invalid decimal strings, unsafe number magnitudes, counterfeit matrix digests and negative gas reductions. Acceptance requires these attacks to be caught. An artifact without the required `LOCAL_HARDHAT_MOCK_NOT_LIVE_FHE` environment label is rejected, even if it contains superficially plausible numbers.

### F. Evidence report rendering

The HTML renderer rejects a partial summary, an invalid evidence tier, mismatched transaction count, absent width-family results or a false assertion that mock gas is live gas. It escapes all dynamic HTML strings, embeds no external script or tracker and displays all operation-width rows. If separate corroborated testnet receipts are supplied, they appear in a separate narrative and are never merged into the local mock gas table.

The report must be previewed locally after generation. A useful review reads not only the headline win count but the worst regressing families, exact source matrix SHA-256 and the measurement limitations. A funding application should link to the raw JSON and exact compiler commit, not solely a screenshot of a summary table.

### G. Optional read-only testnet RPC verification

This checker consumes an existing Phase 6 live SDK trace and queries an HTTPS Arbitrum Sepolia RPC endpoint. It requires chain ID 421614, expected signer identity, deployment transaction with null destination, the reported contract address, matching block hashes and numbers, successful receipts and runtime bytecode. All three recorded execution transactions must resolve to successful receipts with the same gas and block numbers reported by the SDK trace and reach the configured minimum confirmation depth.

Forged response tests cover wrong chain, reverted receipt, wrong signer, nonexistent runtime bytecode, inadequate confirmations, mismatched gas, duplicate transaction hashes and an RPC payload that does not contain a valid JSON-RPC result. If the live trace itself has incomplete provenance, Phase 7 rejects it before performing network calls.

An RPC provider is an external trust boundary. Its answers must be independently reproduced by another provider for high-confidence public evidence. No private key is required to verify already-mined transaction receipts.

## Commands to run

Install the additive patch once, after downloading and checking the ZIP digest that accompanies the release:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
$zip = "$env:USERPROFILE\Downloads\FHEC-V2-CoFHE-Phase7-Differential-Evidence-20261101.zip"
$staging = Join-Path $env:TEMP ("fhec-phase7-" + [guid]::NewGuid().ToString("N"))
Expand-Archive -LiteralPath $zip -DestinationPath $staging -ErrorAction Stop
& (Join-Path $staging "Install-FHECPhase7.ps1") -Project "$env:USERPROFILE\Documents\fhec-v2" -CasesPerFamily 2 -SkipDependencyInstall
```

The release SHA-256 must be checked against the published exact digest before extraction. Do not substitute a guessed hash. The installer checks all unpacked manifest entries and the reviewed Phase 6 baseline digests before any write.

After the smoke run has passed, run the full benchmark:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
$v2 = "$env:USERPROFILE\Documents\fhec-v2"
& "$v2\tools\cofhe\verify-phase7.ps1" -Project $v2 -CasesPerFamily 16 -SkipDependencyInstall
```

Then run the expanded stress suite if local runtime and CPU budget permit:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
$v2 = "$env:USERPROFILE\Documents\fhec-v2"
& "$v2\tools\cofhe\verify-phase7.ps1" -Project $v2 -CasesPerFamily 28 -SkipDependencyInstall
```

`-SkipDependencyInstall` avoids running `pnpm install`; it is appropriate only when the exact frozen Hardhat dependencies have already been installed successfully. Omit it for a clean machine with a known-good network setup. `-SkipRustTests` saves time but narrows the claim; it should not be used for a final acceptance run.

## Evidence inspection protocol

Do not send a grant reviewer only a console transcript that says “passing.” Preserve all contents of the run's `target/cofhe-phase7-<id>` directory. Attach the generated Rust `.sol` from its run-specific directory, the source `.fhe`, the two handwritten references, matrix JSON, manifest and exact pinned dependency lock file. Preserve at least one failed or regressed run if you later change the compiler, to avoid survivor bias.

Useful manual checks in PowerShell:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
$v2 = "$env:USERPROFILE\Documents\fhec-v2"
Get-ChildItem -LiteralPath (Join-Path $v2 "target") -Directory -Filter "cofhe-phase7-*" | Sort-Object LastWriteTime -Descending | Select-Object -First 3 FullName, LastWriteTime
```

Read the newest `three-way-verified-summary.json` and `provenance.json`. Compare `regression_families_vs_tuned` with the complete 24-family table rather than interpreting one headline ratio as an empirical speedup. Check that every reported `gas_median` is a decimal string and the `evidence_tier` is `verified-local-mock-only`.

## Acceptance and non-acceptance

| Observable | Valid Phase 7 interpretation | Invalid interpretation |
|---|---|---|
| 51 tests passing with N=2 | local mock three-way differential suite accepted | mainnet encrypted runtime is certified |
| 144 unique receipts | three successful simulated transactions per oracle case | 144 live FHE coprocessor executions |
| 384 or 672 deterministic values checked | fixed oracle family coverage | formal exhaustive test of 64-bit arithmetic |
| optimized `repeated` costs less mock gas | this workload/model shows mock improvement | all workloads achieve a network gas reduction |
| optimized `repeated` ties tuned reference | CSE matches this manually optimized pattern locally | compiler is globally optimal |
| optional RPC receipts accepted | SDK-reported transactions found on Arbitrum Sepolia | independent proof of confidential computation correctness |
| local clippy and tests passed | pinned source compiles and has no detected tested regression | Rust compiler has no undiscovered bug |

No finite test suite justifies “100% secure,” “production ready,” or “complete V2” as a blanket statement. The trustworthy output is a set of verified, reproducible, explicitly scoped engineering claims.

## Known sources of variance

Gas variation can arise from storage zero-versus-nonzero transitions, encrypted ACL bookkeeping, Hardhat's mock task implementation, operation-specific fake ciphertext allocation and compiler optimizer settings. Wall-clock elapsed time can vary with machine load, Node garbage collection, Windows filesystem activity, background antivirus and test batching. Rather than hiding variance, keep per-transaction raw evidence and summarize per-family distributions. Do not compare different machines' elapsed times directly without controlled methodology.

Different consuming contract addresses intentionally receive separately encrypted inputs. This is necessary for input-proof binding and also means there may be small differences in mock task state. The tests maintain a fixed variant ordering, which is reproducible but may introduce order bias. A future benchmarking phase can rotate variant order and reset equivalent storage state between repeats. For now, Phase 7 prioritizes correctness, transparent provenance and comparable wrappers over claiming a high-confidence microbenchmark.

## Exit checklist

- [ ] Installation had no Phase 6 hash mismatch.
- [ ] Golden benchmark and stress fixture validations passed.
- [ ] Rust source generated with `--optimize`; source SHA-256 recorded.
- [ ] Solidity compilation succeeded with frozen dependencies.
- [ ] All three contracts deployed against the local CoFHE mocks.
- [ ] All case-by-case decrypted outputs matched the oracle.
- [ ] Wrong consuming contract was rejected.
- [ ] Wrong signer was rejected.
- [ ] Stranger decryption was rejected.
- [ ] Independent receipt audit found exactly `72 × N` successful transactions.
- [ ] All 24 cost-family results were preserved, including negative values.
- [ ] HTML report clearly labels mock-only gas and wall-clock timings.
- [ ] Optional testnet receipts, if supplied, were corroborated read-only and labeled separately.
- [ ] Complete evidence directory and compiler source were retained.
- [ ] No user source, credential or historical evidence was overwritten.

An incomplete checklist means Phase 7 should be reported as partial, even when some console tests passed.
