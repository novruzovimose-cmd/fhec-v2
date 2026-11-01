# FHEC V1 — Exact-commit release review template

This file is a template. Fill the values from evidence, not speculation. Save a copy outside generated directories when preparing the actual source release.

## Candidate identity

- Intended public version: **TBD**
- Cargo workspace package version: **verify**
- Git commit SHA: **not yet created/verified**
- Git remote and repository: **verify**
- Rust toolchain / editions: **verify**
- Release review date: **TBD**

## Correctness

- Supported language/type matrix: **evidence required**
- Compiler and optimizer equivalence tests: **evidence required**
- Encrypted conditional and function call behavior: **evidence required**
- Real TFHE encrypted-input-to-decrypted-output tests: **evidence required**
- Unsupported feature diagnostic behavior: **evidence required**
- Malformed IR and invalid circuit handling: **evidence required**

## Local reproducibility

- D source-only export directory: **TBD**
- Export file count and SHA-256 manifest: **TBD**
- Locked format/check/test/Clippy/release-build gates: **TBD**
- Direct binary workflow tests: **TBD**
- Cold Cargo registry test if selected: **optional, not a substitute for hosted CI**

## Hosted CI on exact commit

- Windows workflow run URL and job result: **TBD**
- Ubuntu workflow run URL and job result: **TBD**
- RustSec advisory job run URL and result: **TBD**
- Other existing workflows reviewed: **TBD**
- Any reruns after commit amendments: **TBD**

## Dependency and security review

- Inventory JSON saved and inspected: **TBD**
- RustSec advisories/decisions: **TBD**
- Licenses with unknown or reciprocal expressions reviewed: **TBD**
- Private input/key exposure scan and manual assessment: **TBD**
- Threat model and limitations documented accurately: **TBD**

## Release files and documentation

- `.gitignore` reviewed for generated data: **TBD**
- Staged tree and Git diff inspected, no backups/secret files: **TBD**
- `LICENSE` full Apache-2.0 and notices: **TBD**
- README and changelog exact-version agreement: **TBD**
- All advertised examples validated: **TBD**
- Executable naming/bench Cargo requirement explained: **TBD**
- Analyzer limitations, predicted cost versus measured timings explained: **TBD**
- Archive generated from exact candidate commit and contents reviewed: **TBD**
- Binary behavior and hash evidence: **TBD**

## Final decision

- Release blocking issues: **TBD**
- Decision: **NOT APPROVED until all required evidence is attached**
- Final reviewer: **TBD**

**Do not mark an item complete because an installer printed `INSTALLED`. It must have independent behavior or evidence appropriate to that item.**
