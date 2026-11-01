# Dependency advisory and licensing audit

## Inventory

`tools/d-release/dependency-inventory.ps1` runs `cargo metadata --locked --format-version 1` to list direct and transitive Cargo package versions, licenses recorded by crates, and package sources. It writes `dependency-inventory.json` plus `license-review.txt` into Stage D's evidence directory.

A package's metadata license field is evidence of its *declared* license, not a legal assessment of whether all its obligations are satisfied. Unknown, copyleft, and reciprocal-license expressions receive attention in `license-review.txt`. Even apparently permissive entries need appropriate notice/attribution review. The project's Apache-2.0 license does not automatically establish every dependency's compatibility.

## Known-vulnerability scan

Install the audit program separately, from the developer's terminal if desired:

```powershell
cargo install cargo-audit --locked
cargo audit
```

The Stage D local verifier automatically runs `cargo audit` when installed. Pass `-RequireAudit` to make its absence a failure, rather than a clearly labeled pending result. GitHub CI includes a separate job that installs `cargo-audit` and requires its success.

The RustSec database changes over time. A clean result reflects the advisory database available **when that command ran**, not a permanent claim of zero security problems. Any advisory, withdrawn/crate-unmaintained warning, or audit-tool failure must be reviewed before release; do not silently suppress warnings or automatically upgrade lockfile packages.

## Manual review still required

Review license texts/NOTICE requirements, unusual transitive dependencies, git-sourced packages, platform-specific packages, and advisories not automatically detectable by RustSec. Record decisions in the release review. Only the release commit's exact Cargo.lock matters for the final security signoff.
