# Live safety, evidence integrity and $20,000 grant preparation

Version: Phase 8 + Phase 9 additive verification extension, 2026-11-01

## 1. What these phases can legitimately add

For FHEC, the highest-value evidence gap after extensive Rust unit tests and CoFHE local mocks is an independently inspectable real-network demonstration with a bounded workload and reproducible source chain. Phase 8 adds a mechanism for executing the selected compiler-generated and reference-backed CoFHE Solidity on the approved Arbitrum Sepolia testnet after explicit approval. Phase 9 provides a read-only independent corroboration layer for receipts and native transaction fees. Together, they create a clearer funding case because a reviewer can distinguish software correctness, local test harness behavior, confirmed on-chain activity and observed fee data.

Neither stage secures a grant by itself. A $20,000 grant depends on the specific funding program, eligibility, demonstrated ecosystem value, live demand, builder credibility, milestones, and an external evaluator's judgment. The patch only strengthens the technical evidence and transparency of a potential application. It does not create an awarded grant, assure the user that funding is available, or prove enterprise readiness.

## 2. Exact risk statement

The user is considering a series of live Arbitrum Sepolia transactions. They can cost native testnet gas and potentially incur other protocol or service-side costs. This patch enforces user-entered limits for the native gas paid by transactions it initiates. It does **not** enforce a universal cap on CoFHE coprocessor fees or subscription/service charges because those were not independently metered or modeled here. Never advertise an all-in cost limit based solely on this runner.

The real network may be unavailable, a RPC provider may be stale or malicious, the SDK may change independently, network congestion may change the estimate, a transaction may revert, and asynchronous FHE decryption may not complete during a finite test window. The patch is designed to fail closed and preserve evidence in these conditions. It does not guarantee every testnet action will succeed or that a previously spent fee will be refunded.

For any real run, use a dedicated testnet wallet whose balance is deliberately small and whose key is not used for production funds. Use a reputable HTTPS RPC endpoint selected by the operator. Never supply a wallet key to a shared chat, pasted log, screenshot, git repository, ZIP attachment, HTML evidence report or funding portal. Prefer a new key whose only authorization is the intended testnet environment.

## 3. Three separate commands and their authority

### Preparation: no payment

`prepare-phase89.ps1` builds exact generated Solidity, checks canonical plan fixtures and test harness integrity, and runs the selected suite against local CoFHE mocks. It does not require a wallet private key, consent phrase or live RPC. Its evidence is local. The default installer invokes this step after add-only installation. Passing this step does not mean live Phase 8 or independent Phase 9 has occurred.

The preparation writes a `prepared-phase89.json` file. The operator must keep this file and its generated source/seal sibling directory unmodified. The generated directory has a unique name, avoiding accidental reuse of an older compiled artifact. The prepared file records the exact number of live transactions required for the chosen profile; it is a proposal, not an authorization.

### Optional execution: deliberately paid

`run-phase89-live.ps1` is a separate command that does not run by default. It requires all three explicit native spend limits: `MaxGasPriceWei`, `MaxNativeFeesWei` and `MaxTransactions`. It also requires an exact opt-in consent phrase and the `AllowLiveTransactions` switch. The script validates that the prepared evidence was produced by a prior successful preflight in the intended `fhec-v2` project, that generated source directories are under the approved project path, and that sealed plan and Solidity have not changed. A mismatch stops before the transactions.

The operator must separately set a trusted HTTPS Arbitrum Sepolia endpoint in `FHEC_TESTNET_RPC_URL` and a dedicated key in `FHEC_TESTNET_PRIVATE_KEY`. They should review the plan and fee caps independently before calling the script. The runner will also check chain ID 421614 and local account balance, avoiding accidental use of another network.

No private key or sample plaintext input is encrypted with an unspecified production setting. All demo vectors are deterministic and public. This is a test compiler integration, not a confidentiality guarantee for the input vectors. It uses the project's pinned CoFHE SDK and existing wrapper assumptions.

### Independent audit: read-only

`audit-phase89.ps1` runs after a complete provisional SDK trace has been written. It can be rerun without access to the wallet key and without sending a new transaction. It requires a HTTPS RPC endpoint for the same testnet and exact prepared plan/seal digests. It reconstructs the transaction list from the original trace, checks all receipts and deployed code, then calculates real native-chain fee and gas comparisons.

If the paid command succeeds but audit fails because a RPC provider is unavailable, use the read-only audit again later. **Do not rerun the paid command merely because the audit failed.** Replaying could redeploy contracts and repeat every fee-bearing computation.

## 4. Cost budget calculation

The budgeting unit is wei. The operator chooses a maximum EIP-1559 gas price in wei per gas, and a total native chain fee ceiling in wei. For each transaction, the runner asks the signer/provider for a gas estimate and adds a margin of 20 percent plus 10,000 gas. It then computes a conservative fee reservation:

`estimated_max_native_fee = (estimated_gas + estimated_gas / 5 + 10000) * MaxGasPriceWei`

The runner allows submission only if `native_fees_paid_so_far + estimated_max_native_fee <= MaxNativeFeesWei` and the next transaction is within the authorized `MaxTransactions`. EIP-1559 transaction overrides specify both `maxFeePerGas` and `maxPriorityFeePerGas`, with the former set at the operator's maximum. The wallet balance must be sufficient for the entire explicitly configured chain-fee authorization budget.

This reservation is conservative at the point of each submission. It may cause a run to stop even when the actual fee would have been lower, which is safer than silently increasing authorization. Actual receipt fees are later computed from `gasUsed * effectiveGasPrice`. If the receipt is missing or reverted, the run is incomplete even if the transaction fee has already been incurred.

Transactions on this chain may have pricing components distinct from other networks. The receipt-derived effective gas price represents the native transaction fee as reported by that chain RPC; it does not measure all CoFHE protocol service costs. Keep cost-model assumptions separate and do not multiply static operation counts by an invented fiat exchange rate to claim savings.

If running the wider three-way comparison plan, make sure the count includes the 3 deployments plus 216 executions, a total of 219 transactions. Starting with smoke6/optimized-tuned requires 2 deployments and 12 executions, 14 total. The operator should first inspect testnet account balance and expected fees using the chosen wallet/provider, then set the smallest reasonable limits. The patch does not supply hardcoded defaults that could be mistaken for universally safe fee quotes.

## 5. Interruption and journal recovery

The transaction journal starts with an identity entry for the exact plan and source seal, network and signer public address. A new line is appended as soon as a transaction hash is available. The next line records either receipt verification and details or an error state. A crash can leave a journal with some transaction hash submissions and no complete trace. This is not an idempotent operation and the script does not assume it can replay safely.

In a partial run:

1. Stop issuing new transactions.
2. Preserve the entire `target/cofhe-phase89-live-<run-id>` directory, including the journal and any partial logs.
3. Inspect each `transaction_sent` hash in a block explorer or through a read-only RPC method to determine whether it was mined.
4. If the failure was in SDK decryption, retain the relevant contract address and handle; do not assume the chain transaction reverted.
5. Classify the result as incomplete rather than passing Phase 8.
6. Determine the reason for failure and whether a separate, newly authorized run is warranted. Do not reuse the old trace filename or source directory.

There is no automatic cleanup transaction or contract self-destruct. The deployment remains on the testnet after the command terminates, whether the benchmark completed or not. The file installer never attempts to reverse network-side effects.

## 6. Trust levels of decryption evidence

The SDK decryption comparison establishes a specific kind of observable correctness: for a known public vector encrypted for a known consuming contract and executed using a known generated function, the SDK-observed decrypted output equals the independent plaintext oracle. It is useful for finding generation mistakes, wrong width handling, signedness drift, selector mistakes and MUX regressions.

It is weaker than a cryptographic proof that a remote coprocessor executed the entire computation correctly. The independent RPC verifier corroborates the chain portion of the operation, not secret-key decryption or coprocessor state. It cannot reconstruct the confidential computation from the chain transaction alone. This caveat is visible in machine-readable and HTML evidence reports.

Distinguish three claims:

- `local-mock-passed`: the prepared test environment and mock CoFHE contracts accepted selected vectors.
- `sdk-live-provisional`: the real testnet SDK flow reported matching results and saved transaction hashes, pending independent corroboration.
- `rpc-attested-chain`: read-only node RPC confirmed recorded transactions, gas charges and runtime source hashes. SDK decryption remains an observed result, not an independent cryptographic proof.

Every grant screenshot, summary and presentation should label which tier it depicts. The strongest tier in this patch is still narrower than a production security audit.

## 7. Shareable evidence folder structure

A grant application should present a compact overview and link to a detailed append-only supporting archive. Suggested structure:

```
evidence/
  manifest.json
  source-revision.txt
  methodology.md
  generated-source/
    FHECPhase6Optimized.sol
    FHECPhase6Live.sol
    FHECPhase7Straight.sol
    FHECPhase7Tuned.sol
    phase89-plan.json
    phase89-source-seal.json
  local-preflight/
    prepared-phase89.json
    rust-and-hardhat-test-output.txt
  live-testnet/
    provisional-sdk-live-trace.json
    append-only-transaction-journal.jsonl
  independent-verification/
    independently-corroborated-rpc-receipts.json
    phase9-real-native-chain-comparison.json
    phase9-offline-testnet-evidence.html
  limitations-and-future-work.md
```

Only include `live-testnet` and `independent-verification` as passed results if the corresponding actual run completed successfully. Retain both success and failure data. Public evidence must be redacted for credentials, API keys, sensitive local user paths and unrelated information, while preserving enough hashes and public transaction identifiers for reproducibility. The wallet public address may be shown as part of verification; its private key must never be included.

## 8. What reviewers should independently reproduce

Give reviewers the exact Git commit ID or equivalent immutable SHA-256 source inventory, versioned dependency lockfile, Rust compiler and SDK versions, Solidity compilation settings, chain ID, code seal, plan fixture and receipts. If the project lacks a public repository, acknowledge that hosted CI reproducibility and source trust are currently more limited.

Reviewers can regenerate the canonical plan using `node tools/cofhe/phase89/verify-plans.mjs` and run Node unit tests without a wallet. Those with the source environment can regenerate the optimized Solidity from the FHEC CLI, compare emitted contract bytes, and run the local Hardhat differential mock preflight. Those with a public Arbitrum Sepolia endpoint can run the read-only Phase 9 verifier against the published plan and trace. None of these operations should require access to the original developer's private key.

For high confidence, have two independent RPC providers corroborate the same transaction/block records and record any disagreement. This patch queries one configured provider per audit run and can be rerun against another, but does not automatically produce a multi-provider quorum or prove provider honesty. A stronger future audit could compare state roots, event logs and public protocol task registries where available.

## 9. Cost table presentation and honest comparisons

For a $20,000 tooling proposal, present at least four separate rows: local compiler static encrypted operation estimate; mock CoFHE gas (clearly marked non-live); actual Arbitrum Sepolia native gas; and CoFHE coprocessor/service fees (unknown unless independently measured). Do not place those columns on a single unlabeled cost axis.

For each measured family, show all variants and the number of observations. Document the exact input widths, vector seed or sample indices, chain and block range, contract addresses and fee units. If a family has three samples, show the median and individual values rather than a p95 claim. A 219-transaction workload may be expensive and still insufficient for broad statistical certainty; funding reviewers prefer an honest limited pilot over an unsupported claim of universal speedup.

If the optimized generated variant is more expensive on live native gas than a handwritten reference, report that regression. If an optimization improves static encrypted operation count but leaves on-chain gas unchanged, distinguish compiler task count from network billing. Such honest comparisons are actionable and establish that optimization reporting is not merely promotional.

Potential next economics work includes tracking protocol-specific task metering, separate coprocessor cost invoices or receipts, SDK encryption/decryption timings on independent machines, compile time and binary size, sustained workload reliability, and a documented operational cost model for contract consumers.

## 10. Suggested $20,000 milestones, conditional on external approval

These are illustrative engineering deliverables for a funding proposal, not a statement that any program currently offers these amounts:

| Milestone | Demonstration | Technical acceptance | Suggested budget |
| --- | --- | --- | --- |
| 1. Reproducible CoFHE pipeline | Public FHEC CLI + Solidity demo | Clean clone generates same library, Windows/Linux CI passing | $4,000 |
| 2. Verified real testnet samples | Phase 8 opt-in evidence | Public transaction hashes, supported widths/ops, valid SDK-observed outputs | $5,000 |
| 3. Independent billing evidence | Phase 9 chain receipt audit | Receipts, gas, native fee comparisons and documented service-fee limitations | $4,000 |
| 4. Developer experience | Stable CLI, fixtures, docs and tutorials | End-to-end sample setup and reproducible reports for third parties | $4,000 |
| 5. Security and release readiness | External review / hardening sprint | Dependency audit, threat model, CI gates, clear limitations | $3,000 |

The amounts total $20,000 and illustrate how to make deliverables measurable. They are not an application-specific recommendation unless the grant's rules, eligible expenses, geographic restrictions and funding scope are confirmed. A funding application should propose only work not already completed and should not promise guarantees outside the team's control.

## 11. Remaining work after both phases

Even a complete Phase 8/9 live demonstration leaves substantial V2 engineering:

- Production-quality on-chain operation/task accounting, including independent CoFHE coprocessor fee measurement.
- Hosted CI on immutable code revisions for Rust and Solidity integration across Windows and Linux.
- Robust compatibility tracking for SDK and contract dependency changes, especially upstream major versions.
- Threat model and review of proof validation, key management, replay protection and task confidentiality.
- Fuzzing and property-based differential testing across entire unsigned input domains, where feasible.
- Independent third-party compiler reference tests and reproducibility on more than one workstation.
- A complete benchmark design with repeated randomized cases, network-condition controls and reproducible statistics.
- Documentation of gas versus asynchronous FHE task cost and latency, including failure and timeout behavior.
- End-user documentation, example applications, versioned release notes and license compliance.
- Separate security review before claiming production deployment readiness.

The appropriate current claim, if all available local tests pass, is that the compiler has a strong local CoFHE integration harness with guarded instrumentation for future live validation. After separately successful Phase 8 plus Phase 9 runs, it may claim a verified testnet demonstration with independently corroborated chain receipts and actual native gas charges. Neither condition mathematically translates to a fixed percentage of total V2 production readiness or a guaranteed grant size.
