# Stage D signoff ledger

This ledger is intentionally conservative. **Do not precheck items because files were installed.** Copy test and CI evidence after it exists.

## Local preparation

- [ ] New Stage D workflow and scripts installed without overwriting existing code.
- [ ] Source-only export succeeds, with SHA-256 manifest inspected.
- [ ] Local clean-export `cargo fmt`, `check --locked`, full tests, Clippy and release build pass.
- [ ] Direct binary help/build/analyze pass from clean-export source.
- [ ] All 12 Cargo workspace package versions and license metadata agree; README/changelog public version reviewed.
- [ ] Cargo dependency inventory generated and licenses reviewed.
- [ ] RustSec cargo-audit run and advisories reviewed (not merely optional skipped audit).

## Publication-adjacent gates (NOT part of local Stage D completion)

- [ ] `.gitignore` and actual staged diff show no unwanted outputs/backups/private data.
- [ ] First intended source commit created and verified.
- [ ] Fresh GitHub clone of exact commit builds with `--locked`.
- [ ] Hosted Windows workflow job green on exact commit.
- [ ] Hosted Ubuntu workflow job green on exact commit.
- [ ] RustSec workflow job green on exact commit.
- [ ] Complete LICENSE, README, Cargo version and CHANGELOG reconciled.
- [ ] Final release archive inspected for generated/private content.

**Current status must remain partial until Git is set up and hosted CI is green.** This distinction honors the request not to publish any half-finished demo.
