# FHEC Phase 7 — Threat Model, Evidence Integrity and Research Limits

## 1. Assets and trust boundaries

This patch evaluates software that generates Solidity for encrypted arithmetic. The primary assets are the integrity of generated contract source; correctness of encrypted computation relative to declared language semantics; confidentiality of encrypted user inputs; valid proof consumption under the intended contract and signer; correctness of stored-result ACL; and integrity of published performance evidence. Attackers may be malicious callers, cross-contract proof replayers, unauthorized decryptors, compromised dependency sources, users submitting adversarial program inputs, or researchers unintentionally misinterpreting mock measurements.

There are several distinct trust domains. The Rust FHEC compiler is trusted to implement its frontend validation and optimized lowering. The Phase 6 BigInt oracle is an independently implemented but not formally verified source of expected arithmetic values. Solidity compilation and Hardhat depend on the pinned CoFHE contract version and plugin mocks. The browser-side or Node CoFHE SDK prepares encrypted inputs and signs proof records; it is not replaced by this compiler. A live Arbitrum Sepolia RPC endpoint reports chain data but is not a cryptographic attestation of correct FHE execution. The evidence summarizer runs after tests and must treat raw fixture and measurement JSON as untrusted input.

What is inside the Phase 7 patch is deliberately narrow: source code for two manual Solidity references, one harness generator, a Hardhat differential test suite, deterministic fixtures, a strict independent measurement auditor, static HTML report creation, and optional read-only transaction receipt corroboration. It does not change FHE protocol cryptography, key management, on-chain task verification, Fhenix or CoFHE trusted parameters, or the Rust compiler's internal dataflow representation.

## 2. Assets in detail

### A. Arithmetic correctness

Unsigned arithmetic may be subtly wrong at modular boundaries if generated code widens values and truncates at the wrong time, if signedness is inferred incorrectly, or if a common-subexpression pass removes a task that mutates ciphertext metadata. Phase 7's core arithmetic comparator tests three implementations against a standalone BigInt oracle. The oracle uses bit masks and string conversions instead of floating-point math, including at width 64. This reduces, but does not eliminate, common-mode bugs. Because all implementations use the same CoFHE API, a shared library-level error might affect every variant.

The `gt` family is boolean. It is intentionally verified as a typed boolean expected value in the matrix but rendered as a string at the boundary where SDK decryption output is compared. This avoids coercing `true` to `1` or accidentally treating JavaScript's truthy string values as a cryptographic result. Arithmetic families use decimal-string numbers; every 64-bit input is converted to BigInt without passing through an unsafe JavaScript Number.

### B. Input-proof binding

A ciphertext should not be accepted by an unrelated contract merely because the encrypted payload is syntactically valid. The test harness creates encrypted inputs bound to the actual consuming contract address for each variant. A targeted negative test deliberately presents an input prepared for another harness and requires rejection. This checks the local SDK/mock proof binding and the way the wrapper invokes `FHE.asEuint{w}`. It is not a proof that every possible CoFHE proof-verification path is secure in production, nor does it cover cross-chain replay by itself.

### C. Signer authorization

The harness stores results under `msg.sender`, calls `FHE.allowThis` and `FHE.allowSender`, and exposes read-only encrypted handles by account. A negative test encrypts with one signer then attempts execution from another signer. A separate test requires that a stranger cannot use `decryptForView` to decrypt the owner's handle. These checks validate the local mock flow and provide regression coverage for refactors of the wrapper. They do not replace a review of the underlying ACL contract, SDK user identity assumptions, key material storage or live-contract upgrade governance.

### D. Source and package integrity

The original project can contain valuable user modifications. Installing a large file dump by overwriting existing files is a particularly important threat here. Phase 7 addresses that risk by pinning specific Phase 6 SHA-256 digests, accepting only a manifest-listed set of additive destinations, rejecting any existing destination that differs, copying in small increments and rolling back only unchanged files created by the attempted installation. A preexisting source file is never treated as expendable. The rollback is consciously conservative: if someone modifies a newly installed source during a failed verification, it is preserved rather than deleted.

The manifest binds paths, file lengths and hashes, but it is not digitally signed. An attacker who changes both the archive and its manifest could create a self-consistent malicious package. Therefore the user must verify the exact ZIP SHA-256 out of band and ideally source the archive from a trusted conversation or release channel. Dependencies fetched from public registries retain their normal upstream supply-chain risks; `pnpm --frozen-lockfile` reduces unplanned version changes but does not establish dependency trust by itself.

### E. Benchmark evidence integrity

Benchmark cherry-picking is a research integrity risk. An apparently useful compiler might only beat a naive handwritten reference because the handwritten code was intentionally inefficient. Phase 7 therefore compares both a straightforward and a separately tuned reference, displays all 24 families and retains negative improvements. The tuned reference explicitly shares the repeated multiplication, while the straightforward reference does not. Both are legitimate source patterns, but neither is presented as the unique optimal implementation.

The raw evidence format contains one record per oracle case and three entries per record. The auditor rejects missing variants, reused receipt transaction hashes, out-of-range gas values, counterfeit matrix hashes and mismatched outputs. The report renderer consumes only independently validated JSON. The HTML has no remote dependencies, making it suitable for offline review and avoiding accidental leakage of local paths or metric data to third-party analytics endpoints.

Gas measurements still include the plugin mock implementation, which is not the same execution model as live CoFHE. Even a perfectly validated mock number should not be used as a network billing estimate. The report's mock-only evidence tier is enforced by schema validation to prevent accidental escalation to a live-performance claim.

## 3. Adversary table

| Actor or failure mode | Capability | Phase 7 control | Residual risk |
|---|---|---|---|
| Malicious contract caller | supplies a proof for the wrong consuming contract | cross-harness binding negative test | behavior may differ on live version |
| Wrong signer | reuses encrypted inputs signed under another account | signer rejection negative test | unrelated SDK identity bugs may remain |
| Unauthorized observer | reads a public encrypted handle and requests decryption | account-scoped decrypt denial test | key custody is outside this patch |
| Source archive tamperer | alters a file or hash in transit | explicit SHA-256 manifest + external ZIP digest | not an authenticated signature |
| Local workspace modification | edits baseline or target files | fail-closed baseline/target hash checks | user must resolve legitimate edits explicitly |
| Forged benchmark record | omits failing operation or swaps expected result | canonical matrix and 24-family independent auditor | raw receipt identities not tied to an external chain |
| False efficiency claim | compares FHEC only to naive code | both straight and tuned references, all regressions shown | measured workload selection still narrow |
| Compromised RPC endpoint | fabricates transaction inclusion | HTTPS, chain ID, receipt, signer and confirmations checks | independent RPC provider recommended |
| Testnet chain reorganization | removes shallowly confirmed transaction | minimum confirmations check | deeper reorgs can still occur |
| Untrusted dependency | changes compiler or mock semantics | pinned lockfile and source provenance | no reproducible dependency build attestation |

## 4. Questions not answered by this phase

Whether the compiler prevents all secret-dependent branch leakage requires stronger semantics than a local differential test. The absence of obvious control-flow leaks from the six tested families does not establish noninterference for arbitrary source programs. A formal information-flow analysis, security lattice for encrypted/plaintext values, and proof of optimizer preservation are future work.

Whether a CoFHE task is actually computed under the promised cryptographic security level requires independent protocol and implementation evidence. The API returning a ciphertext handle does not by itself prove the ciphertext's derivation, noise budget or key policy. Phase 7 neither implements a FHE library nor reproduces the coprocessor computation independently; it uses existing CoFHE tooling and local mocks.

Whether mainnet contracts are safely deployable depends on upgradeability, operator control, fee semantics, state lifetime, replay domains, external-call behavior and actual task settlement. Phase 7's example wrappers are designed for exercising an FHE backend. They are not audited production application contracts. A grant demonstration can use them as research evidence but should not hold user assets or confidential production data.

Whether relative gas costs generalize depends on real task scheduling, deployment gas, storage warm/cold effects, encrypted-operation fees, network overhead and workload distributions. The Phase 7 table compares identical mock wrapper paths but does not isolate each constituent cost. It would be misleading to claim a universal percentage improvement for all CoFHE programs from these six source operations.

Whether a single RPC provider is honest is outside the trust model of an HTTP response. Phase 7 checks consistency of data returned by the provider and the existing SDK trace but not an independent light-client proof. For important public claims, corroborate with a second provider and a block explorer, preserve transaction IDs, and document confirmation depth and chain context.

## 5. Failure and rollback behavior

The preflight checks run before adding Phase 7 source files. If the directory is not named `fhec-v2`, if the Cargo manifest is missing, if a pinned Phase 6 file has moved, or if any expected payload fails its checksum, installation aborts with no file changes. This is a feature, not a malfunction. The installer does not attempt to infer that two differently hashed source files are semantically equivalent.

During installation, the script tracks only the exact paths newly added. If verification fails, it deletes those exact new paths only when their current SHA-256 still matches the installed payload. If a file was modified by another process, it is retained and explicitly named in the error. Existing identical Phase 7 files are never included in the deletion list. The project can retain a newly created evidence directory after failure because it may contain useful diagnostic output; the script does not delete past evidence or cryptographic traces.

During a direct re-run of the verifier, source generation uses a new timestamp-plus-random run ID. The generated sources and evidence files are intentionally retained for comparison across runs. This is a safe alternative to silently replacing `FHECPhase6Optimized.sol` from an earlier validation. Avoid concurrent runs against a shared Hardhat `artifacts/phase7` cache unless operating-system filesystem isolation is added; independent run IDs currently isolate generated Solidity and evidence but not every Hardhat cache file.

Network receipt verification has an explicit fail-closed contract. If the trace path does not exist, if an HTTPS RPC URL was not configured, if a receipt is missing, if confirmation depth is insufficient, if a transaction was reverted, or if the contract address is unexpected, the verifier must fail. It does not generate an “RPC pass” entry after a partial read. It never searches for a private key and never uses `eth_sendRawTransaction` or another transaction-sending JSON-RPC method.

## 6. Independent test attack cases

The Module `verify-receipts.test.mjs` uses an in-memory synthetic RPC response fixture so no network is needed for its unit tests. It covers:

1. a chain ID other than 421614;
2. unsuccessful and incomplete receipts;
3. a signer mismatch between the transaction and the SDK trace;
4. mismatched destination contract;
5. missing deployed runtime bytecode;
6. transaction hash duplication;
7. insufficient confirmation depth;
8. successful corroboration where all fields match;
9. malformed transport metadata and rejected HTTPS URL configuration.

The module `metrics.test.mjs` similarly mutates otherwise valid mock evidence. It checks missing coverage, incorrect oracle expectation, missing third variant, repeated transaction hash, corrupt decimal metric, unexpected environment label, unsafe number scale, mismatched matrix digest and explicit performance regression. One particularly important property is that a negative result in a family does not cause the overall mock execution to fail: correctness and optimization are separate dimensions, and a measured regression is a valid scientific observation.

The module `report-html.test.mjs` checks rejection of incomplete reports, explicit tier separation, correct handling of trusted receipt summaries and escaping of externally supplied text. The module `generate-manual.test.mjs` checks that the two references differ only in the deliberately highlighted repeated subexpression, not in authorization logic or source width coverage.

## 7. What reviewers can independently reproduce

A technically equipped reviewer can inspect the ZIP SHA-256 and manifest, regenerate the golden JSON with the Phase 6 oracle, run all standalone Node unit tests without Hardhat, compile the Rust source and three Solidity implementations with the pinned dependency lock, execute the 48-, 384- or 672-case local mock matrices, independently run the report verifier against raw JSON, and inspect the static HTML. They can compare hashes of actual Rust-generated source across runs and see the full per-family regressions.

With access to an authorized previous Phase 6 live testnet trace and a trustworthy HTTPS RPC endpoint, they can separately reproduce read-only transaction inclusion corroboration. They cannot conclude a broad cryptographic assurance level from these artifacts alone. To claim that the Solidity output faithfully enforces encrypted semantics in an adversarial production environment, they must also inspect the underlying SDK, contracts, protocol and live deployment configuration.

## 8. Responsible public claim templates

Appropriate claim: “FHEC generates CoFHE-compatible Solidity from a typed encrypted source program. In a pinned Hardhat CoFHE mock environment, its optimized output matched a modular BigInt oracle and two independently authored Solidity reference implementations across four integer widths and six operation families. We published raw per-transaction mock gas and latency plus a separate, independently checked summary.”

Appropriate qualified live claim, only after optional corroboration: “A previously authorized SDK trace was cross-checked against confirmed Arbitrum Sepolia receipts and deployed runtime code using a read-only JSON-RPC verifier. The SDK decryption and coprocessor correctness were not independently attested.”

Inappropriate claim: “Our compiler is 100% secure, produces globally optimal FHE, and reduces real CoFHE gas fees by X% because 51 mock tests passed.” The evidence is too narrow to support that assertion, and publishing it would weaken rather than strengthen a research/grant application.

## 9. Exit and follow-on research

The Phase 7 exit artifact should be a reproducible engineering dossier: source commit, dependency versions, full raw receipts, exact matrix, verified summary, clearly labeled HTML report, negative authorization test output and SHA-256 provenance. A formal compiler soundness claim requires proof obligations on encrypted control flow, value typing and optimizer equivalence. A production performance claim requires live network measurement and cost accounting for FHE tasks. An audit-readiness claim requires deployment threat modeling, key and role custody, dependency review, contract static analysis and independent security evaluation.

Passing Phase 7 is a useful milestone toward those goals. It is not interchangeable with their completion.
