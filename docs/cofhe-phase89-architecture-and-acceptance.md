# FHEC V2 CoFHE: Phase 8 + Phase 9 architecture and acceptance

Revision 1 — 2026-11-01

## 1. Executive architecture decision

Phase 8 and Phase 9 address two independent evidence questions. Phase 8 asks whether FHEC-generated Solidity can be built, deployed and operated in a narrow real CoFHE testnet context, with a reference oracle as a correctness cross-check. Phase 9 asks whether a second process can use read-only node RPC responses to corroborate the chain receipts, deployed runtime code and actual native gas charges claimed by Phase 8. Both are necessary for credible testnet evidence; neither is sufficient to prove formal compiler correctness, protocol security, confidential coprocessor execution or economically complete pricing.

The phases share a deterministic plan and a source seal. The testnet action is never an implicit follow-up to a compile, test or install command. A user has to choose exactly when to run it, which limited account to use, how many transactions to authorize, and how much native transaction fee exposure to allow. The tool refuses any chain other than Arbitrum Sepolia chain ID 421614.

The desired pipeline is:

1. Rebuild the compiler output from the current Rust workspace against its existing CoFHE grammar and backend.
2. Generate the existing Phase 7 independent handwritten comparison libraries and Solidity wrapper contracts.
3. Write a deterministic test plan from the reviewed Phase 6 plaintext oracle, sealed to exact bytes.
4. Compile the exact sealed prepared sources with an isolated Hardhat config, then exercise the plan against local CoFHE mocks.
5. Issue a prepared report with paths, exact SHA-256 digests, profile, tested case count and chain restriction. Mark that report `live_transactions_sent=false`.
6. Upon separate explicit authorization, deploy selected Solidity variants and run encrypted test vectors on the approved testnet. Record every transaction hash promptly and always preserve the journal.
7. Write an SDK-observed results trace only after the whole deterministic plan completes and each SDK-decrypted output equals the independent modular plaintext oracle.
8. Independently verify all referenced transaction hashes, block confirmations, native gas charges and deployed bytecode hashes against an HTTPS JSON-RPC endpoint. Abort on any discrepancy.
9. Analyze validated data and generate JSON and offline HTML reporting, keeping native-chain costs, latency observations and unknown coprocessor fees separate.

No step claims `100%` completeness or grant acceptance. Each step has precisely scoped acceptance conditions and can fail independently.

## 2. Existing project boundaries

The package is an add-only extension to the already installed Phase 7. It intentionally avoids modifying the Rust grammar, backend emitter, optimizer, cost model, Cargo.lock, CoFHE package manifests, TypeScript SDK dependencies or existing Hardhat fixtures. Its compatibility premise is that Phase 7 passed locally on the current `fhec-v2` repository and that the SHA-256 baseline of five reviewed Phase 7 source/config files has not changed.

The PowerShell installer checks these exact Phase 7 files and refuses a different baseline. This protects against accidental application of a patch to unrelated source variants. A failure means the patch must be reviewed and rebased, not that the user should overwrite local source to coerce the installer.

Phase 8/9 compiles from `examples/cofhe_phase4.fhe`, which is already part of the current V2 progression. `cargo run --locked -p fhec-cli -- cofhe ... --optimize` generates `FHECPhase6Optimized.sol` and a static optimization report. The Phase 7 generator then creates `FHECPhase6Live.sol`, `FHECPhase7ManualStraight.sol`, `FHECPhase7ManualTuned.sol`, `FHECPhase7Straight.sol` and `FHECPhase7Tuned.sol`. The Phase 8/9 plan itself is another sealed input. The exact 7 names are checked without relying on an untrusted wildcard.

The `FHECPhase6Live` Solidity wrapper exercises the optimized output. `FHECPhase7Tuned` exercises the second reference-oriented path. Optionally the `FHECPhase7Straight` path supplies a third comparison. All emitters should accept the same encrypted operation-selector interface, so every transaction measures an equivalent operation on the same encrypted-width family, but this does not guarantee equal gas behavior.

## 3. Module ownership

| Module | Responsibility | Must never do |
| --- | --- | --- |
| `plan.mjs` | Build canonical vectors, variants, transaction count and disclosure | Send an RPC request |
| `verify-plans.mjs` | Regenerate and check six immutable fixture snapshots | Change existing fixtures |
| `source-seal.mjs` | Bind plan and six Solidity files to exact SHA-256 bytes and sizes | Accept changed source after a successful compile |
| `prepare-phase89.ps1` | Run Rust checks, generate Solidity, test local mocks, save prepared report | Read wallet key, deploy or send paid transactions |
| `phase89/hardhat.phase89.config.ts` | Isolate Solidity artifacts/cache for the new phase | Mutate older Phase 6 or Phase 7 sources |
| `FHECPhase89Prepared.test.ts` | Drive selected deterministic mock encrypt/execute/decrypt paths | Prove real network economics |
| `run-phase89-live.ps1` | Enforce human opt-in, check prepared record, configure actual testnet invocation | Automatically retry after partial failure |
| `phase89/run-live.ts` | Check chain/wallet/fees, deploy, execute, preserve journal and provisional trace | Claim independent cryptographic attestation |
| `rpc.mjs` | Restrict requests to known read-only JSON-RPC methods | Sign or submit a transaction |
| `verify-chain.mjs` | Corroborate source trace and every on-chain receipt and runtime code hash | Treat SDK strings as independent FHE proofs |
| `analyze-live.mjs` | Compute observed native fees/gas by operation family and variant | Substitute mock gas or guess missing service fees |
| `report-html.mjs` | Produce a standalone verifiable evidence summary | Hide measurement limitations |
| `audit-phase89.ps1` | Combine read-only verification and analysis, save evidence in a new directory | Access a private key |

## 4. Deterministic plan profiles

Profiles vary the number of independent operation samples. The same oracle supplies every sample, including 64-bit values represented as decimal strings and `gt` as a boolean result, without coercing 64-bit integers through JavaScript `number`.

| Plan | Widths | Families | Samples per family | Variants | Deployments | Executions | Total transactions |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `smoke6` / `optimized-tuned` | u8 | add, sub, mul, gt, max, repeated | 1 | 2 | 2 | 12 | 14 |
| `smoke6` / `three-way` | u8 | six | 1 | 3 | 3 | 18 | 21 |
| `coverage24` / `optimized-tuned` | u8/u16/u32/u64 | six | 1 | 2 | 2 | 48 | 50 |
| `coverage24` / `three-way` | u8/u16/u32/u64 | six | 1 | 3 | 3 | 72 | 75 |
| `comparison72` / `optimized-tuned` | u8/u16/u32/u64 | six | 3 | 2 | 2 | 144 | 146 |
| `comparison72` / `three-way` | u8/u16/u32/u64 | six | 3 | 3 | 3 | 216 | 219 |

`comparison72` means 72 case vectors, not 72 paid transactions. Explicitly budget the entire table entry, including deployments. Larger plans improve sample diversity; they do not establish statistically confident performance distributions without repeated trials, randomization or independent measurements. Real fee numbers can vary with live network conditions, nonce state, block conditions and protocol revisions.

Each fixture is checked against the deterministic oracle at runtime before a prepared plan can pass the source seal. Every profile explicitly declares the chain ID and notes that the native transaction fee cap does not cover external FHE task charges.

## 5. Trust boundaries and state transitions

The prepared source state is not an authorization to spend money. It includes the source directory, plan/seal path, their cryptographic digests, exact profile and transaction count, and flags recording what local checks completed. Any change to those files invalidates later execution. We reject plans whose variants are unknown, whose call-count arithmetic differs, or whose complete sealed bytes differ.

The preflight compiles into an isolated per-run directory in `integration/cofhe-hardhat/phase89/generated/<run-id>` and uses a separate test evidence directory under `target/cofhe-phase89-preflight-<run-id>`. New names are generated with a timestamp plus random suffix. Existing generated paths are never silently reused. The retained prepared record enables the operator to review exactly what would be deployed.

Live execution requires all of the following: the PowerShell `-AllowLiveTransactions` switch, an exact separate consent phrase, a valid dedicated testnet private key from the local process environment, a configured HTTPS RPC endpoint, an explicit `MaxGasPriceWei`, an explicit `MaxNativeFeesWei`, an explicit `MaxTransactions` that covers the entire deterministic plan, chain-ID match, and available account balance. The Hardhat runner independently checks these conditions before its first transaction.

The runner uses EIP-1559 fee overrides and estimated gas with a finite margin. For the next transaction it checks whether `feesPaidWei + approvedGasLimit * maxGasPriceWei` fits the configured native fee budget. It rejects another submission when the estimate cannot fit. This is a chain-fee authorization boundary for the transactions generated by this script, not a general custody guarantee. The wallet may have unrelated pending transactions. CoFHE off-chain metering and service charges have not been fully characterized and therefore cannot be represented as capped by this tool.

On every deployment or execution submission, an append-only journal receives the transaction hash before the runner proceeds. After confirmations and local checks, an additional journal entry captures the result. If the process dies halfway, some transactions may have executed. The correct recovery is audit and investigation, not blind replay.

## 6. Independent verifier scope

The Phase 9 verifier accepts two distinct inputs: a canonical deterministic plan and a provisional SDK trace. It first validates all trace records against the plan (case identity, width, operation, expected result, exact variant set and ordering), checks that transaction hashes do not repeat, and recomputes native paid fees with exact decimal-string `BigInt` arithmetic.

Only after local trace validation passes does the verifier contact the configured HTTPS RPC. It is deliberately limited to `eth_chainId`, `eth_blockNumber`, `eth_getTransactionReceipt`, `eth_getTransactionByHash` and `eth_getCode`. The testnet's chain identifier must equal `421614`. Each transaction must be in a successful receipt, with a matching hash, block number, block hash, sender, destination, gas used, effective gas price and fee. Deployments must be contract-creation transactions and produce the recorded contract address. At a minimum the operator can request two confirmations, and can require more.

For each deployed contract the verifier fetches its current runtime bytecode and compares its SHA-256 hash to the trace recorded by the runner after comparing deployment code to the compiled artifact. This is an after-the-fact code identity check, not a formal source equivalence proof of arbitrary proxy systems. The exact wrappers are used without proxy upgrade semantics.

The verifier then issues a separate attestable JSON report. The report intentionally states that the SDK decryption is not independently cryptographically verified. A matching RPC receipt does not prove the FHE coprocessor produced an honest result. Our observation is that the SDK returned a value and that the local independent oracle expected the same value. A stronger proof would require independent FHE execution/attestation and cross-provider validation beyond the scope of this patch.

## 7. Phase 9 arithmetic and comparability

The analyzer never uses CoFHE mock gas as a live native gas figure. It aggregates actual confirmed receipt gas usage and actual effective gas prices for the selected plan. It reports deployment gas and fees separately from execution gas and fees. For every variant and operation/width family, it emits count and validated totals, median values and comparison to the benchmark variant.

The same deterministic vector is run per variant in a fixed order, so fair comparison is limited. Time-variation across the network can influence fees. Gas used is typically less volatile than wei cost. Gas and wei are separate units. The report must not interpret a reduction in mock instruction count as an equal reduction in live gas or off-chain FHE task fees.

Total native wei paid is the sum of `receipt.gasUsed * receipt.effectiveGasPrice`. The SDK trace supplies matching fields but the independent verifier corroborates them via JSON-RPC. Because JavaScript numbers cannot exactly represent all wei amounts, wei is stored and analyzed as decimal strings and converted to `BigInt` before arithmetic. A safe integer bound is used only for samples and local latency fields.

If a family has fewer than five independent samples, its percentile or confidence claims are absent rather than fabricated. This intentionally makes a six-case smoke test small but honest. The larger plan is preferable for comparative evidence, subject to explicit spending authorization and disclosure.

## 8. Reporting format

The default outputs are:

- `target/cofhe-phase89-preflight-<run-id>/prepared-phase89.json`: compile and local mock evidence, paths, digests and selected test profile.
- `target/cofhe-phase89-live-<run-id>/append-only-transaction-journal.jsonl`: every testnet submission and its observed state. The journal is retained after failure.
- `target/cofhe-phase89-live-<run-id>/provisional-sdk-live-trace.json`: only written on complete SDK-observed success.
- `target/cofhe-phase89-audit-<run-id>/independently-corroborated-rpc-receipts.json`: only written on successful read-only corroboration.
- `target/cofhe-phase89-audit-<run-id>/phase9-real-native-chain-comparison.json`: fee/gas/latency results grounded in confirmed receipts.
- `target/cofhe-phase89-audit-<run-id>/phase9-offline-testnet-evidence.html`: self-contained shareable demonstration, with limitations.

The HTML report can be viewed without any RPC credential. It is a summary of evidence rather than a signed audit artifact. For any grant reviewer, preserve the raw plan, source seal, generated Solidity, original receipts and all digests. The user should avoid publishing the private wallet key, RPC API token, unredacted log headers and unrelated local information.

## 9. Acceptance criteria and release gates

**A8.1**: The six deterministic plan fixtures all regenerate byte-for-byte logically from the current oracle, with exact counts and no unexplained vector drift. **A8.2**: A tampered fixture or changed generated Solidity byte causes fail closed. **A8.3**: `cargo fmt`, targeted Rust backend/CLI tests and Clippy all pass unless explicitly opted out and recorded as skipped. **A8.4**: Compilation of the exact sealed generated Solidity passes and the selected mock suite returns correct decryption/selection behavior for its profile.

The local preflight also performs a strict scoped TypeScript check using `phase89/tsconfig.phase89.json`. The scope matters because the existing Hardhat workspace has older unrelated test files that use untyped dynamic ethers contract methods. The new Phase 8/9 source is typechecked independently rather than making its result contingent on historical TypeScript errors in earlier project tests. This scoped pass does not certify the entire historical Hardhat tree as type-error-free.

**A8.5**: Without both consent mechanisms, no real chain request that broadcasts a transaction can occur. **A8.6**: Wrong chain, missing dedicated wallet, unsupported RPC, bad source plan, insufficient fee or transaction allowance fail before a paid submission. **A8.7**: Every transaction sent is preserved in the journal and the run never automatically retries a failed or interrupted batch. **A8.8**: Complete SDK trace contains every deployment and operation result, the original oracle expectation, receipt data and null coprocessor fee fields.

**A9.1**: The independent verifier can reject forged and duplicated receipt hashes, missing receipts, unexpected from/to address, wrong block or chain, reverted status, insufficient confirmation and altered code identity. **A9.2**: Only permitted read-only RPC methods are available, never `eth_sendRawTransaction`. **A9.3**: Fee calculations preserve exact wei arithmetic and reject values presented outside the audited trace. **A9.4**: The comparison report clearly differentiates actual testnet gas, actual native on-chain fees, SDK-observed latency, unknown off-chain service fees and non-proven execution security.

**A9.5**: A grant evidence package must include independent external reproducibility instructions, an exact code revision, source digest, toolchain versions, workload profile, full environment, transaction hashes and records, failures/nonresponses, and both successes and limitations. **A9.6**: The project may claim a completed live testnet demonstration only after the operator separately runs and passes the live phase and independently corroborates it; successful local mock tests alone do not satisfy this criterion.

## 10. Out-of-scope and next gates

The patch does not create mainnet support, production key management, a general compiler language expansion, cryptographic correctness proofs, an independent coprocessor, end-user billing infrastructure, economic guarantees, a formal audit, a third-party bug bounty, external grant approval, operational observability across an entire chain, or automatic long-running monitoring. Those would be distinct milestones with funding and security review.

For a public release, additionally require hosted Linux and Windows CI on an immutable source revision, complete dependency and license audit, a published threat model with test evidence, reproducible release artifacts, and a public repository with no private credentials. This patch improves the evidence layer; it does not make the entire V2 program universally complete.
