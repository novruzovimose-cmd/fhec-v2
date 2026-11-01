# Dependency advisories, licensing, and TFHE security signoff

## Current blocker: RustSec audit not executed

The most recent Stage D terminal report explicitly stated:

- a locked `cargo metadata` inventory was generated (82 packages in that run);
- the license metadata inventory was produced, **not** a legal license clearance;
- `cargo audit` was **not run**, because `cargo-audit` was not installed.

This is a release blocker. A clean compilation or cryptographic execution test does not substitute for a current advisory review. Stage E will run `cargo audit` automatically if it is available; otherwise it records the missing tool and keeps the blocker open. If network or advisory-database retrieval fails, the status is **failed or incomplete**, never silently passed.

## Required developer-side process

```powershell
cargo --version
cargo install cargo-audit --locked
cargo audit
```

Installation can require network access and compiling Rust dependencies. Run from the exact intended release source; preserve the full output with date and advisory database provenance. The hosted Ubuntu RustSec GitHub Actions job is another independent mandatory gate; local success cannot replace it.

For each advisory: record crate name, version, advisory ID, impact, whether vulnerable code is reachable, patched version, and remediation. Do not use automatic blanket `ignore` entries. If a risk is accepted as a documented exception, explain why it does not compromise supported V1 scenarios and obtain a deliberate approval before publication.

## Third-party license and notice review

The Stage D `dependency-inventory.json` includes package names, versions, source and license expressions. Inspect `license-review.txt` for missing SPDX fields, copyleft/reciprocal licenses and Git-sourced crates. Read the licenses of actual third-party distributions, not just the workspace license. Determine whether attribution, NOTICE files or redistributable binary requirements apply; source-only distribution and binary distribution have different obligations.

The full project-level Apache-2.0 license is necessary but does not automatically satisfy every dependency's license requirements. The inventory identifies review candidates, not an automated legal compatibility decision.

## TFHE-specific security claims and non-claims

Based on the locally tested V1 architecture, the runtime actually encrypts inputs, executes Boolean circuits under a TFHE backend, and decrypts specified results. Encrypted branch conditions are lowered into MUX operations. Security depends on the TFHE library's implemented primitives, its parameters, key management, runtime environment, and the application's own information-flow assumptions.

The CLI intentionally emits decrypted outputs to the user. Earlier hardening stopped echoing plaintext inputs in ordinary diagnostics and added `--inputs-stdin`; neither prevents a local privileged process from observing input memory, terminal history, standard streams or OS process details. Logging application data, copying key material, or leaking result-dependent metadata can still undermine confidentiality. Do not claim that all side channels are eliminated.

Bootstrapping and noise risk reports are backend/model-dependent. The V1 analyzer's noise-management discussion is **not** proof of multi-scheme BFV/BGV/CKKS automatic noise scheduling. The supported V1 backend is TFHE-focused.

## Security release approval fields

- [ ] Current RustSec audit succeeded on intended lockfile, with advisories reviewed.
- [ ] Hosted RustSec GitHub Actions job succeeded on exact release commit.
- [ ] All third-party licenses, redistributable notices and potential unknowns reviewed.
- [ ] Final staged diff and intended GitHub archive inspected for credentials, local profiles, user data and backups.
- [ ] Key material is not serialized into persistent calibration/report files; relevant tests passed on final commit.
- [ ] Actual TFHE encrypted input-to-output tests and invalid-input errors passed on final commit.
- [ ] Public documentation states residual security and performance limitations without invented certification.

Until all applicable fields are reviewed and evidenced, **security release signoff remains open**.
