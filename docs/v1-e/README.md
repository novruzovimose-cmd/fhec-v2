# FHEC V1: Stage E release-candidate preparation

**This is a non-publishing release engineering package.** Its purpose is to turn the verified local Stage D source tree into a source-only, reproducible *candidate* and to expose what still prevents a public FHEC V1 release.

Stage E does not make a Git commit, push to GitHub, invent a version, create a tag, or publish a release. Stage D's last reported local clean-source validation passed, but the tool reported that hosted Windows and Linux CI had **not** passed and that the RustSec audit was **not run** because `cargo-audit` was missing. Those gates remain open regardless of how many local checks pass.

## Quick start (Windows PowerShell 5.1)

From the existing FHEC repository root, after installing this bundle:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\d-release\e-prepare.ps1 -Project (Get-Location).Path
```

The script saves a unique run under `target/fhec-e-candidate/` and prints `STAGE E CANDIDATE PREPARED; NOT CLEARED FOR PUBLICATION.` A source archive and integrity-checked file list are created there. **The archive name includes `NOT-FOR-PUBLICATION` deliberately.** Do not upload that archive to GitHub Releases.

The human-readable status report is under the printed run's `reports/E_STATUS.txt`. A machine-readable version is `reports/e-preflight.json`. Run results are evidence, not an endorsement of public release readiness.

## What is inside this package

- `tools/d-release/e-prepare.ps1`: read-only inspection of the current project plus new outputs under `target`; delegates to the Stage D allowlisted source exporter and verifies the archive.
- `tools/d-release/e-archive-verify.ps1`: checks the ZIP's exact file set, lengths and SHA-256 of every decompressed file against the D exporter manifest.
- `docs/v1-e/*`: version decision ledger, release-notes draft, security audit process, Git/CI handoff, actual acceptance ledger, and scope/limitations review.

The scripts are stored under `tools/d-release/` rather than another tool root so the existing Stage D source exporter includes them in its explicit allowlist. No existing Rust module, Cargo manifest, `.gitignore`, Git workflow, CI configuration or repository metadata is changed by the installer.

## Scope of evidence

The Stage D source exporter preserves code, examples, lockfile, documentation, select scripts, license and `.github/workflows` while excluding `target`, backups, credentials by known patterns, patch archives and local profiles. It computes a SHA-256 for each copied file. Stage E compresses that filtered tree with .NET's ZIP API, including dotfiles, then recomputes and checks SHA-256 for **every ZIP member**.

This proves the candidate ZIP is bitwise consistent with that one allowlisted source snapshot. It does **not** prove that the archive builds on Linux, that GitHub's eventual commit contains exactly the same bytes, that there are no hidden secrets, or that every open-source license obligation has been satisfied.

## The honest gate

The script is designed to succeed at **preparing** a candidate while *reporting blockers*. It never prints `E VERIFIED` or claims release approval. Missing cargo-audit does not mean the Rust dependency graph is safe. Green local Windows tests do not mean hosted Linux tests passed. Version `0.1.0` in Cargo metadata does not automatically authorize a public tag called `v1.0.0`.

The final public release must wait for the cross-platform hosted CI and RustSec jobs to pass on the **exact intended Git commit**, explicit version/release-note reconciliation, review of staged files, and archive inspection.

## Safe reruns and troubleshooting

Every run gets a new unique `target/fhec-e-candidate/<timestamp>-<random>` directory, so reruns do not replace earlier evidence. If the Stage D exporter rejects unexpected files or symlinks, inspect its diagnostic and update the source only after independent review. Do **not** copy all of `target` or `C:\Users` into the candidate as a workaround. If the source tree changes between preparation and publication, run the complete validation sequence again on the final commit.

If `cargo audit` reports a vulnerability, investigate the exact advisory and affected dependency rather than suppressing warnings to get green output. If the audit is missing, use `cargo install cargo-audit --locked` in an environment with internet access and a compatible Rust toolchain, then rerun the preflight.

If PowerShell fails to parse or a source preflight differs from the current project, stop. The package deliberately does not rewrite existing project files or try a destructive rollback.
