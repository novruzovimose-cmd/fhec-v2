# FHEC V2 / CoFHE Phase 12 — Local Release Qualification

## Decision

Phase 12 is the final **local verification and release-candidate packaging** phase in this track. It does not make FHEC V2 production-complete and does not close real-chain, cryptographic-audit, operational, or grant-award requirements. This distinction remains explicit in all generated artifacts.

## Predecessor trust chain

1. Existing FHEC V2 compiler and CoFHE Solidity emitter produce six generated/handwritten cross-checked libraries.
2. Phase 8/9 source-seal record anchors seven prepared source files.
3. Phase 10 compares 24 arithmetic and comparison families at widths 8, 16, 32, 64 under real encrypted **Hardhat mocks**, across optimized, tuned, and handwritten straight-line variants.
4. Phase 11 independently reconstructs arithmetic, checks the raw mock trace and generated source seal, regenerates the offline report, and pins 13 source/evidence files with SHA-256.
5. Phase 12 extracts the precise generated-source seal from the user's **independently validated** Phase 11 dossier manifest, then checks that it matches the source-seal file bytes and independently reconstructed results. If a separate externally recorded hash is supplied, Phase 12 also compares it to that digest. It then checks the 24-family/three-variant matrix, runs twelve isolated tampering attacks, and generates a portable local release capsule. A hardcoded hash from another machine's fixture must never be required.

## Architecture

The Phase 12 package introduces separated modules under `tools/cofhe/phase12`:

- `constants.mjs`: reviewed source anchor, fixed unsigned widths, functions, variants and evidence paths.
- `io.mjs`: bounded regular-file and canonical SHA-256 primitives.
- `coverage.mjs`: 24-family three-way matrix and negative ciphertext-domain proof requirements.
- `qualification.mjs`: reviewed anchor, Phase 11 independent replay and fail-closed readiness policy.
- `inventory.mjs`: bounded selected-source SHA-256 inventory, excluding generated outputs, dependencies, and credentials.
- `candidate.mjs`: atomic copy of verified evidence plus source inventory and offline report.
- `verify.mjs`: standalone verification of candidate bytes, manifest, source seal, outputs, and falsified claims.
- `adversarial.mjs`: isolated 12-attack corruption replay that must reject every forgery.
- `repro.mjs`: second-run semantic comparison over identical oracle inputs and decrypted outputs, deliberately ignoring nondeterministic mock gas and clock durations.
- `stress-fixtures.mjs`: 12 fixed-seed, 16-cases-per-family suites; an independently implemented BigInt model validates 4,608 deterministic **plaintext reference vectors**, explicitly not additional CoFHE mock executions.
- `report.mjs` and `handoff.mjs`: human-readable static HTML and grant-evidence handoff.
- `cli.mjs`: bounded offline qualification, candidate creation and verification operations.

## Why add a source inventory?

The Phase 11 package protects evidence bytes, but it does not identify which selected project source files were present during local qualification. Phase 12 fingerprints the Rust crates, FHE examples, CoFHE tools, reviewed integration scripts, documentation, dependency lockfiles and any matching CI workflows. It excludes `target`, `node_modules`, caches, generated Solidity, and suspicious credential-like paths. The inventory is informational, not a digital signature, ownership proof or independent audit.

## Scope and threat limits

An attacker capable of changing both an artifact and its unsigned SHA-256 manifest can forge an unsigned package. This release capsule detects isolated changes and independently reconstructs the arithmetic model, but stronger authenticity requires a trusted distribution identity and eventually an external reviewer. The per-dossier source-seal digest is bound across independently checked evidence files. Where a trusted pre-recorded source digest exists, it may be supplied as a separate optional pin to prevent substitution of an entirely reconstructed dossier.

All command-line release candidate writes must be into a newly named subdirectory of the target project's `target` folder. The existing compiler sources, contracts, package-locks, deployments and evidence folders remain untouched. The installer adds files only, detects conflicting existing files and rolls back only newly copied files whose hashes are unchanged.

## Accept / reject

Accept this phase as *locally qualified* only when all prescribed Node regression tests pass, 4,608 standalone arithmetic reference vectors validate, 12 attacks are rejected, the chosen actual Phase 11 dossier independently verifies, optional second-run comparison passes if supplied, the Rust workspace tests pass (unless visibly skipped), and the new candidate verifies both on disk and after independent movement.

Reject on any missing source anchor, modified source, missing family, absent straight variant, incorrect semantic output, altered HTML, evidence-promotion claim, duplicate or unexpected file, symlink, malformed manifest, unsupported archive path, or mismatch of SHA-256 bytes.

## Outstanding nonlocal acceptance

`LIVE_TESTNET`: explicitly approved, separately executed live testnet traces with independent receipt verification.

`COPROCESSOR_FEES`: actual provider and off-chain FHE task costs, not Solidity EVM mock transaction gas.

`HOSTED_CI`: immutable hosted Windows and Linux successful jobs against a public reviewed commit. This phase ships a workflow definition; a YAML file alone is not evidence that it passed.

`INDEPENDENT_SECURITY_REVIEW`: externally performed smart-contract, FHE backend, cryptographic semantics and deployment review with remediation evidence.

`GRANT_REVIEW`: out of developer control, regardless of code quality. The requested $20,000 is a funding goal, not a verified payment.
