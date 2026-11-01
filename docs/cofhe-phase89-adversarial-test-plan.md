# Phase 8 + Phase 9: executable and adversarial test matrix

## 1. Test policy

Run the least expensive, least privileged check first. The default installation and preparation must use no live testnet wallet and must never trigger a real transaction. Testing should be repeatable from a clean, reviewed FHEC source revision. Preserve all output artifacts and failures instead of rewriting reports to create a synthetic pass.

The local verification sequence is: Node unit tests → canonical plan fixture tests → Rust formatting and backend/CLI tests → Clippy → actual FHEC CoFHE output generation → Phase 7 independent Solidity generation → source-plan seal → isolated Hardhat compile → prepared CoFHE mock calls → seal recheck → prepared JSON evidence. Each stage stops on failure, without moving on to a more expensive stage.

The real network sequence is separate and cannot run as a side effect of the local sequence. The opt-in stage checks fee and transaction count authorization, validates network and account, then deploys and runs a deterministic batch. The read-only audit is separate again, so an interrupted audit never needs a transaction replay.

## 2. Test families and proof obligations

| Test family | Input class | Expected behavior | Negative or boundary obligation |
| --- | --- | --- | --- |
| Canonical plan | Six profiles/modes | Exact reference case identity and expected output | Changed vector or operation rejected |
| Plan arithmetic | All six cases/24 cases/72 cases | `deployments + executions = total` | Transaction ceiling below total rejected |
| Width correctness | u8, u16, u32, u64 | Modular integer oracle matches SDK decryption | No JavaScript number precision loss for u64 |
| Addition | Zero, regular values, modular overflow | Output wraps modulo 2^width | No widening, no signed interpretation |
| Subtraction | Equal, descending, modular underflow | Output wraps modulo 2^width | No negative plaintext result |
| Multiplication | Zero, one, regular, overflow | Output wraps modulo 2^width | Multiplication-by-zero optimization must retain semantic behavior |
| Greater-than | Less, equal, greater | Encrypted bool equals unsigned oracle | Boolean must not be reinterpreted as uint |
| Maximum | Less, equal, greater | Encrypted MUX result is maximum | Both branch values have same width |
| Repeated subexpression | Shared encrypted operands | Equal optimized, reference values | Savings claims based on actual work, not source text alone |
| Mock encryption domain | A proof for a different consuming contract | CoFHE mock rejects cross-contract use | Fail before interpreting decrypted result |
| Source integrity | Seven pinned source/plan files | Exact byte length and SHA-256 for each | One-byte edit or symlink rejected |
| Baseline integrity | Five reviewed Phase 7 prerequisites | SHA-256 matches original reviewed uploads | Local modifications preserved, install refused |
| Live guard | Missing consent, bad network, wrong key | Abort before live transaction | No implicit retry after crash |
| Live budget | Gas price cap, fee cap, transaction cap | Each tx fits remaining reservation | Budget overshoot and depleted wallet reject |
| Journal | Every transaction sent | Hash is appended immediately | Incomplete run leaves journal retained |
| RPC identity | Testnet chain 421614 | Correct chain before validation | Wrong chain rejected |
| Receipt inclusion | Deployments and executions | Unique tx receipt, successful status | Missing, duplicated, reverted receipt rejected |
| Receipt linkage | From/to, code address, block hash | Every receipt matches claimed sender and contract | Wrong sender/destination/block rejected |
| On-chain gas | Gas used and effective gas price | Exact equality to claimed trace, wei math | Overflows or invented gas fee rejected |
| Runtime bytecode | Deployed Solidity runtime code | SHA-256 equals compiled and observed runtime | Changed on-chain runtime rejected |
| Confirmation depth | Latest chain height and tx block | At least configured confirmations | Recent/shallow receipt rejected |
| Analysis arithmetic | Native wei totals and medians | BigInt recomputation, separate deploy/exec | Invalid or orphan receipt rejected |
| HTML output | Independent summary | Offline, escaped, limitations included | Cannot call a mock observation 'live' |

## 3. Verification profiles

### Smoke6

Six operation families, width u8, one canonical case per family. Two deployment variants by default: optimized and tuned. This is the least-cost live option with 14 planned transactions. The same profile in three-way mode adds a straight reference variant and requires 21 transactions. Its primary use is integration validation, not broad performance inference.

### Coverage24

Four widths times six operation families, one canonical case per family, for 24 oracle cases. Two variants require 50 transactions; three variants require 75. It is the recommended wider local mock regression profile, because it extends the smoke exercise to all supported unsigned CoFHE widths while keeping local turnaround manageable.

### Comparison72

Four widths, six operation families and three deterministic input vectors per family, for 72 oracle cases. Two variants require 146 transactions and three variants require 219. This is a better basis for preliminary comparisons of operation gas, but still has only three samples per family/variant, so a 95th percentile or statistically meaningful confidence interval must not be claimed.

The deterministic case order may confound a network benchmark because variants are always executed in fixed order. This limitation must be disclosed. A future phase can add randomized balanced ordering, multiple repeated batches and an independent measurement host after the fee budget is understood.

## 4. Node preflight tests

Use Node's built-in `node:test` runner to keep the source-only test harness independent of the Hardhat test process. The installer invokes the following reviewed test files before running the compiler:

1. `plan.test.mjs` checks exact profiles, integer families, variant selection, sample count and expected deterministic values.
2. `verify-plans.test.mjs` confirms six persisted fixtures and rejects a changed stored vector.
3. `source-seal.test.mjs` confirms the seven-file seal rejects tampering, file replacement and malformed state.
4. `rpc.test.mjs` rejects unsafe transport, unapproved RPC methods, malformed node output and incompatible identifiers.
5. `verify-chain.test.mjs` uses synthesized receipts with a controllable RPC transport to exercise both successful verification and malicious network responses.
6. `analyze-live.test.mjs` independently tests fee summation and comparisons across validated trace records.
7. `report-html.test.mjs` validates the standalone output and safe rendering properties.

These are offline or synthetic verification tests. They are designed to verify software invariants and error handling, not to represent a successful real network deployment. An attacker could still compromise the local machine or RPC provider, which requires a separate threat assessment.

## 5. Required adversarial receipt matrix

An independent reviewer should deliberately mutate a copy of the provisional evidence in a temporary directory and show that Phase 9 refuses it. Recommended mutations:

1. Change `chain_id` from 421614 to another integer. Expect refusal before accepting an attestation.
2. Change `plan_sha256` or `source_seal_sha256`. Expect exact digest mismatch.
3. Change `profile` from smoke to coverage without replacing cases. Expect canonical plan mismatch.
4. Remove the first deployment record. Expect incomplete evidence.
5. Duplicate a deployment hash as an execution hash. Expect repeated receipt rejection.
6. Change the signer address. Expect transaction `from` mismatch.
7. Change the contract address of any execution. Expect `to` mismatch.
8. Set a transaction receipt status to zero. Expect refusal.
9. Change `blockNumber` in one receipt. Expect refusal.
10. Change `blockHash` while retaining transaction hash. Expect refusal.
11. Change `gasUsed` by one gas unit. Expect refusal.
12. Change `effectiveGasPrice` by one wei. Expect refusal.
13. Change the claimed native fee while preserving gas and price. Expect arithmetic rejection.
14. Set a receipt to `null`. Expect refusal.
15. Have the node return wrong `eth_chainId`. Expect refusal.
16. Reduce `eth_blockNumber` so a selected receipt is insufficiently confirmed. Expect refusal.
17. Return `0x` as deployed contract runtime code. Expect refusal.
18. Return a code byte string differing in one byte. Expect runtime SHA-256 refusal.
19. Change an SDK plaintext result from a correct value to an incorrect value. Expect oracle disagreement.
20. Remove the SDK-observed result flag. Expect refusal.
21. Change an unsigned uint64 value by converting it to floating point. Expect plan comparison rejection.
22. Change `coprocessor_fees_measured` to true without service accounting. Expect refusal.
23. Change an arbitrary transaction time from a finite millisecond value to infinity or negative. Expect refusal.
24. Include a new variant absent from the plan. Expect exact variant-set rejection.

Mutations of archived evidence should happen only in copies, never in the original source-sealed directory or originals from the user. Preserve original test logs and the exact test harness revision.

## 6. Credentials and payment tests

Execute missing-consent tests before any funded wallet or endpoint is configured. A direct attempt to execute `run-live.ts` without the required environment consent must fail promptly, even if a config network exists. The PowerShell wrapper must also reject no `-AllowLiveTransactions`, wrong consent phrase, missing MaxGasPriceWei, missing MaxNativeFeesWei, missing MaxTransactions, invalid wallet key shape or non-HTTPS endpoint.

Never use the test for an actual fund transfer when exercising negative paths. Do not put real private keys in test fixtures. The only place a dedicated testnet key may appear is the local process environment on the operator's computer during a separately authorized run. The key should never be written to an HTML report, JSON artifact, console success output or archive.

Check the entire planned transaction count before running: two variants and six cases require 14 transactions, not six; three variants and 72 cases require 219, not 72. The native transaction budget is denominated in wei. An example value without unit conversion can be off by nine or eighteen orders of magnitude and is unacceptable. The operator must explicitly select both maximum gas price and total native fees in wei after consulting a trusted external wallet or fee view.

One extra limit is economic scope: budget enforcement inside this patch covers transactions generated by its own runner and directly paid native-chain transaction fees. It does not control unrelated transactions from the same wallet and does not enforce an upper bound on CoFHE coprocessor, relayer, subscription or third-party charges. A separate product integration might be required for complete accounting.

## 7. Protocol and SDK acceptance

The project's pinned Hardhat, ethers and CoFHE SDK versions are used as-is. The patch does not silently upgrade dependencies or regenerate the lockfile. Any later SDK migration requires an explicit compatibility sprint because encryption input packaging, proofs, contract bindings, decryption function signatures or signer permissions may change.

For the currently pinned SDK path, input encryption is bound to the consuming contract address, and the live runner decrypts with the signer-specific result-handling method. The runner records the SDK-observed result next to the oracle expectation. This is evidence of an integration path functioning for selected samples, not proof that every possible homomorphic computation is correct or confidential.

If decryption returns `undefined`, throws repeatedly, or disagrees with the oracle, the runner fails and records an incomplete journal. It must not rewrite the expected value to manufacture equality. Decryption failure may reflect waiting for asynchronous FHE task completion rather than a bad compiler, but the correct outcome is still not to mark the benchmark as complete.

## 8. Negative rollback and preservation

The installer is deliberately add-only. Before copying, it hashes and measures every file in the supplied payload and checks five reviewed Phase 7 baseline files. If a destination already exists with identical bytes, it is accepted as idempotent; if different, installation stops before changing anything. A local preflight failure removes only files introduced by this attempt that still match the original payload bytes. If the user edited one of those new files after installation began, it is preserved and named in the failure.

Generated preflight or mock evidence under `target` and isolated `phase89/generated` are not deleted by rollback. Their state may explain a failure. Neither previous phase sources nor user-created changes are overwritten or rolled back. This is intentionally conservative for a live-transaction project.

Verify two cases in a temporary copy of the repository: a clean install passes, and an intentional modification to one reviewed baseline file refuses with no changes. Also test a conflicting destination path with different content; the installer should not overwrite it. Do not perform these adversarial changes in the user's only working copy.

## 9. Real testnet validation record

After operator consent and a successful real run, save:

- The exact SHA-256 and original generated Solidity source, including the Rust compiler output and reference libraries.
- The plan fixture identity, case count and full source seal file.
- The approved chain ID, network name, SDK/toolchain versions and tested wallet public address.
- Transaction hashes for every deployment and execution, not just successes selected for a screenshot.
- Exact receipt status, gas used, effective gas price, block number, block hash and confirmation depth for every transaction.
- Runtime bytecode identity for every deployed wrapper.
- SDK-decrypted value and oracle expectation for every case/variant.
- Full failures, timing outliers and retry counts, including partial runs and incomplete journals.
- Separate totals for native deployment fees, native execution fees, and unmeasured CoFHE service fees.
- A count of all workload samples and the distinction between local mocks and real-network receipts.

Reviewer scripts should be able to perform the Phase 9 read-only audit with just the public RPC endpoint, original trace, deterministic plan and its source seal, without access to the wallet key.

## 10. Stop conditions

Stop and do not move to the next stage if a prerequisite hash differs, the local mock suite fails, a source seal changes after compilation, the testnet differs from chain 421614, wallet balance or fee ceilings are insufficient, contract runtime code differs from compiled output, SDK decryption disagrees with the oracle, any receipt is missing or reverted, or independent RPC corroboration fails. Do not treat an error from the independent receipt audit as permission to rerun deployments. Preserve the existing evidence and troubleshoot read-only first.

The grant-ready acceptance statement is narrow: "For an identified source revision and deterministic workload, FHEC generated CoFHE Solidity, passed local differential mocks, and [if separately executed and verified] recorded independently corroborated Arbitrum Sepolia successful receipts, gas use and native transaction fees." Until the bracketed condition is proven, do not remove that qualification.
