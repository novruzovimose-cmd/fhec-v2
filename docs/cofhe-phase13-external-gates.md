# Phase 13 — External Evidence Architecture

## Purpose

The primary artifact is an evidence gatekeeper, not a new compiler backend. Phase 12 already qualified a specific local source capsule. Phase 13 connects that trusted local candidate to externally observable evidence and prevents unsupported promotion of local results into mainnet, full-API, audit or funding claims.

## Existing trusted components reused

- `phase12/verify.mjs` replays the complete local candidate's evidence, including independent oracle checks and manifest integrity.
- `phase12/inventory.mjs` identifies the exact selected source files from the original local qualification.
- `phase89/plan.mjs` validates previously reviewed test cases and approved variants.
- `phase89/verify-chain.mjs` checks actual transaction receipts, transaction calldata, source-correlated deployed runtime bytecode, sender, chain ID 421614, transaction status and fee arithmetic.
- `phase89/analyze-live.mjs` reconstructs real native transaction gas and latency statistics, with unmeasured coprocessor fees preserved as unknown.
- Existing `run-phase89-live.ps1` is the only reviewed opt-in paid run path. Phase 13 never calls it.

## Phase 13 modules

| Module | Responsibility | Trust boundary |
| --- | --- | --- |
| `io.mjs` | Bounded regular file loads, digest parsing, directory boundaries, redaction | Local files can be modified and manifests are unsigned |
| `source.mjs` | Re-evaluate Phase 12 capsule and every original source digest | Newly added Phase 13 files cannot invalidate an older immutable source review |
| `chain.mjs` | Read-only double-provider replay of existing Phase 8/9 plan, source seal, transactions, code and fees | Two RPC providers may collude or share infrastructure |
| `github.mjs` | Public API verification of a precise GitHub commit and Windows/Linux jobs | GitHub controls reported job outcome; source-to-release binding still matters |
| `gates.mjs` | Strictly typed progress assessment and $20,000 illustrative milestone accounting | Never create third-party security/fee proof from self-assertions |
| `packet.mjs` | New-folder-only evidence outputs, SHA-256 file records, independently regenerated report | Unsigned manifests do not authenticate publisher identity |
| `report.mjs` | Offline HTML and Markdown grant reports | Every claim must remain within its evidence tier |
| `cli.mjs` | Controlled offline/read-only commands | No wallet, deployment, mainnet or fee authority |

## Expected sequence

1. Install Phase 13 add-only and run Node regression tests.
2. Re-run the Phase 12 capsule verification; verify every originally fingerprinted source file. Extra Phase 13 source files are allowed because they postdate Phase 12.
3. Publish a clean commit and run the offline/Hardhat mock workflow on hosted Ubuntu and Windows runners.
4. Verify the exact public GitHub Actions run from the API and the intended commit SHA. Publishing a workflow file without a successful hosted run does not pass the gate.
5. Separately authorize the existing Phase 8/9 live runner on a dedicated small-balance testnet wallet, after preparing and reviewing exact transaction counts and fee caps. Phase 13 cannot perform this step.
6. Revisit the newly produced SDK trace from *two distinct HTTPS RPC providers* without any wallet key. The two independently observed sets of receipts must agree on block hashes, transaction hashes, calldata hash, fee and source identity.
7. Measure provider-specific coprocessor task charges from provider evidence if available; do not extrapolate from EVM gas or static operation counts.
8. Arrange external source-generation, access-control and Solidity security review; keep all identified findings and remediations attached.
9. Generate the Phase 13 HTML, Markdown and machine-readable grant packet.
10. Seek the funder's independent milestone and eligibility review. No software command can approve a $20,000 payment.

## Evidentiary tiers

### Local source qualified

Phase 12 candidate verification passed and the originally included file hashes match the current source. This shows local review reproducibility, not chain activity.

### Hosted public CI checked

The GitHub API reports a publicly readable run for the given SHA, exact workflow, push/dispatch event, repository and successful distinct Ubuntu and Windows jobs. A public release reviewer still needs to check that this SHA corresponds to the code being distributed.

### Real testnet two-provider receipt verified

The existing `verifyChain` returns two successful independent read-only attestations from distinct hostnames and receipt records agree. This supports a narrow network claim about tested contracts, observed SDK decryption, and native transaction fees. It does not prove full cryptographic soundness or coprocessor fee accounting.

### Provider-fee documented

An independently identifiable CoFHE provider statement, bill, task meter or official published fee schedule is required, with task IDs and billing units. Native gas receipts alone do not meet the requirement. Phase 13 does not automatically mark this verified.

### Independently security reviewed

A reviewer independent from the FHEC authors examines the exact source revision and testnet protocol trust boundaries, records all findings and confirms remediation. Self-generated checklists and successful fuzz tests alone do not meet the requirement.

## Safety guarantees

- The Phase 13 CLI has no command for `sendTransaction`, `deploy`, `broadcast`, wallet creation or key import.
- The read-only testnet command takes two RPC endpoints only through process environment variables. It records no endpoint strings in the public evidence packet.
- The installer never edits preexisting compiler, contract, lockfile or evidence bytes. A conflict is an error.
- The installer can remove only files it itself copied and only while their SHA-256 remains identical to the originally copied content.
- No source is erased if installation or verification fails.
- The evidence packet is generated in a fresh unique subdirectory of `target`, refusing to overwrite prior records.
- Public GitHub verification uses no personal access token. The source repository must be public.
- No developer secret should be committed, attached to a grant report or pasted into chat.

## Threat model and explicit residual risk

1. **Coordinated data forgery:** An attacker with write access to a report and its unsigned manifest can forge the entire packet. Use signed Git tags, trusted publication identities and reviewer-side reproduction for stronger authenticity.
2. **Provider collusion:** Distinct RPC hostnames do not prove independent operator ownership or canonical chain consensus. Prefer providers operated by different organizations and compare public explorers.
3. **Reorganizations:** Confirmations can change on a reorganizing chain. A previously verified transaction might later be orphaned. Reverify shortly before submission.
4. **SDK trust:** Matching decrypted test vectors is useful evidence of application semantics; it is not a zero-knowledge proof or cryptographic soundness certificate for the remote coprocessor.
5. **Performance scope:** One Arbitrum Sepolia run under current conditions says nothing certain about mainnet gas markets or all off-chain service charges.
6. **API drift:** The historical source snapshot uses SDK/Hardhat plugin `^0.7.1` and CoFHE contracts `0.2.0`. Newer upstream versions require a separate compatibility review; this release intentionally avoids silent upgrades.
7. **Security independence:** A local rule can detect missing or malformed evidence but cannot authenticate the independence or professional qualifications of a reviewer.
8. **Funding externality:** Fhenix, fellowship operators, grant programs, or sponsors decide eligibility and awards. Funding status remains unverified until a separate genuine award notice exists.

## Acceptance checklist

- [ ] Phase 13 local tests pass on the user's project.
- [ ] Phase 12 candidate file integrity passes against the actual selected capsule.
- [ ] All original fingerprinted source files remain unchanged.
- [ ] Hosted Ubuntu job passed on the exact intended SHA.
- [ ] Hosted Windows job passed on the exact intended SHA.
- [ ] A reviewer checked the source SHA against the distributed release.
- [ ] Explicitly approved Phase 8/9 run has verified transaction receipts.
- [ ] Two independent read-only RPC providers agree on all compared receipts.
- [ ] On-chain native fees and off-chain protocol charges are separated.
- [ ] Independent security review and remediation evidence are available.
- [ ] Grant application program and terms have been checked separately.

## Separate future coding milestones

For full developer-facing CoFHE support, establish a versioned API compatibility matrix rather than a subjective percentage. Include arithmetic beyond the existing selected tests, boolean operations, encrypted equality and ordering, bitwise and shift operations where offered, casting, wider types, permission and ACL lifecycle, client proof conversion, asynchronous failures, retries, task settlement, and generator diagnostics for unsupported constructs. Only mark a row supported when it has parser/typechecker tests, emitted Solidity tests, and both negative/positive integration evidence for the pinned library version.
