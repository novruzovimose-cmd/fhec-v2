# FHEC V1 Stage E acceptance ledger

**Stage E is the release-candidate preparation and pre-publication review, not the public release itself.** The project has completed A1–A3, B1–B3, C1–C3 locally as user-reported/verified in prior logs. Stage D's local clean-source gate passed, but hosted CI and RustSec were incomplete. This ledger must not be marked entirely complete from an E script alone.

## Stage E tasks

- [ ] Installer added only intended new documentation and verification scripts, with no existing Rust files overwritten.
- [ ] SHA-256 source manifest created by the allowlisted D exporter.
- [ ] Source-only candidate ZIP produced and fully verified against every file hash and byte count.
- [ ] Cargo workspace versions and Apache-2.0 license metadata agree; public version intentionally chosen.
- [ ] README, Cargo manifests and CHANGELOG reconciled to the public version.
- [ ] Release notes reviewed against supported features and actual user-visible commands.
- [ ] Full RustSec `cargo audit` succeeded on the intended locked graph.
- [ ] Dependency license notices and unknown-license cases manually reviewed.
- [ ] No secrets, key material, local profiles, backups or generated data in candidate or staged source.
- [ ] Stage D clean-checkout checks rerun after any release-source changes.

## External gates required for final public publication

- [ ] Exact release commit exists, and its committed content was reviewed.
- [ ] Fresh checkout passes `--locked` build, full tests, zero-warning Clippy and release binaries.
- [ ] Hosted Windows CI succeeds on exact release commit.
- [ ] Hosted Ubuntu CI succeeds on exact release commit.
- [ ] Hosted RustSec dependency audit succeeds on exact release commit.
- [ ] Intended tag and GitHub generated release archive reviewed and match the exact commit.
- [ ] Final user approval given before public publication.

## Current observations

- Stage D reported `STAGE D LOCAL CLEAN-SOURCE VALIDATION PASSED` on the Windows workstation.
- Stage D reported `AUDIT: NOT RUN, cargo-audit is not installed` and `HOSTED WINDOWS/LINUX CI IS NOT YET VERIFIED`.
- Therefore **Stage D is locally verified but not globally signed off, and Stage E cannot authorize publication**.
- Replace these observations with the actual evidence paths, timestamps, full hashes and workflow run URLs when known.

## Meaning of a green E preparation run

`PASS: E source-only candidate archive and SHA-256 checks` means the candidate ZIP matches the newly exported source files. It does not verify Cargo tests inside that new snapshot, a clean Git commit, the user-selected public version, the current RustSec database or two operating systems. The intended success message is:

`STAGE E CANDIDATE PREPARED; NOT CLEARED FOR PUBLICATION.`

There must be no automated `E VERIFIED` marker until every required signoff is independently evidenced.
