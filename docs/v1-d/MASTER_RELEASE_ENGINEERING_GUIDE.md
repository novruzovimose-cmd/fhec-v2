# FHEC V1 — Release engineering and reproducibility guide

**Stage:** D (the preparation and verification layer between a functional compiler and a public release).

**Status at authoring:** C3 was reported verified on the developer's Windows workstation, including formatting, locked workspace build, release executables, analyzer documentation checks, and actual TFHE tests. Stage D's scripts have been assembled, but the developer has not yet run them; the GitHub repository's clean hosted builds cannot be claimed before a CI run on the release commit.

**Non-goals:** This package will not change Rust semantics; silently fix a failing compiler test; create a Git commit; push a remote; publish a demo; choose a public semantic version; generate GitHub tags; or declare a cryptographic certification. Those actions require explicit release preparation and passing final checks.

## 1. The purpose of Stage D

A workspace that builds on the machine where it was developed is not necessarily portable. Local backups might be imported accidentally, `target` might contain old artifacts, dependencies might be resolved differently on Linux, and build scripts can depend on non-source files that never get committed. For FHEC, TFHE execution and cryptographic integration tests also make it easy to mistake a fast successful compile for true end-to-end correctness.

Stage D adds independent checks without disturbing the existing implementation. There are two different levels of evidence:

1. A *local filtered-source build* proves that an explicitly enumerated set of source files can compile and run all tests from a clean Cargo target directory on the local machine. It uses installed Rust and, unless requested otherwise, the existing Cargo registry cache. It is not a fresh Git clone.
2. A *hosted checkout build* is performed by GitHub Actions on Windows and Ubuntu after a commit is pushed. It tests exactly the checked-in files, not your entire local working directory. Both operating systems must report success before the cross-platform release requirement can be signed off.

These two forms of evidence complement rather than replace each other.

## 2. Running the local clean-source gate

Start from the FHEC repository root using Windows PowerShell 5.1, and execute:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\d-release\verify-local.ps1 -Project (Get-Location).Path
```

The script prints every major operation and native Cargo output. It may need to compile many crates into a new target folder, even if your current workspace is already built. TFHE tests may take several minutes. Do not terminate the script just because an individual cryptographic test is slow; its name and final result should remain visible. You may use Ctrl+C to stop an unusually long run, but a partially completed run does not qualify as a passing gate.

Each local run gets a unique directory:

```text
target/fhec-d-validation/YYYYMMDD-HHMMSS-random/
    source/
    source-manifest.json
    fresh-cargo-target/
    reports/
        dependency-inventory.json
        license-review.txt
```

The `source` directory has no Git history, no `target`, and no local outputs. The SHA-256 manifest records the accepted file list and hashes. Compile results live in a **separate** `fresh-cargo-target` directory outside the exported source, preventing a successful test from relying on an old build output.

The script does not automatically delete this evidence. Inspect it before manually removing it to conserve disk space. The entire validation directory remains below `target` and should be ignored by Git.

### Optional strict modes

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\d-release\verify-local.ps1 -Project (Get-Location).Path -ColdCargoHome
```

`-ColdCargoHome` starts with an empty dependency registry cache. It often requires downloading a substantial amount of Rust dependencies, and therefore requires working internet access and available disk space. The installed toolchain is still the developer's toolchain, managed by rustup; a fresh Cargo home is not a fresh operating system.

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\d-release\verify-local.ps1 -Project (Get-Location).Path -RequireAudit
```

`-RequireAudit` makes the security-advisory gate mandatory. If cargo-audit is absent, the run fails instead of silently skipping the check. The default allows local build verification without this optional tool but labels the vulnerability review incomplete.

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\d-release\verify-local.ps1 -Project (Get-Location).Path -FastOnly
```

`-FastOnly` tests the CLI contract suite but skips the expensive full workspace suite. It is a development diagnostic tool, not release acceptance evidence.

## 3. Content selection and security boundary

The export script is intentionally not a general-purpose recursive copy of the repository. It selects a controlled set of inputs:

- Rust source, Cargo manifests, test modules and recognized textual fixtures inside `crates/`.
- `.fhe` examples, Markdown documentation, approved JSON documents and report specifications.
- Root Cargo files, Rust toolchain specification, README, changelog, `.gitignore`, LICENSE, and a handful of optional public project metadata files.
- CI workflow definitions and a restricted set of Stage C/D verification scripts.

It excludes patches, ZIP files, private-key suffixes, `.env` files, backup directories, generated artifacts, `.git`, and `target`. Files with unexpected extensions are not exported; if a compile error indicates an essential binary fixture or additional source asset was excluded, determine whether the asset is safe and truly required before broadening the allowlist. Never fix an isolated-build failure by copying your entire home directory or raw `target` tree.

Symlinks and reparse points inside selected directory trees are refused. This prevents source-export paths from silently pointing outside the project. A SHA-256 digest is computed for every source before/after copying. If a file changes while it is copied, the export fails.

The exporter also rejects a few recognizable private-key headers and common cloud access-key formats. It is a *heuristic*, not a comprehensive credential scanner. Private files with nonstandard formats, secrets inside images, proprietary configs, and pasted personal information can still evade it. Final release contents require a human review of actual Git staged paths and diffs.

## 4. Verification gate sequence

### Cargo version and license consistency

`version-consistency.ps1` obtains the locked workspace package metadata, verifies that all twelve workspace packages declare the same version and Apache-2.0 license, and writes `reports/version-consistency.txt`. Public tag names, README language and changelog headers still require human reconciliation.

### Formatting

`cargo fmt --all -- --check` checks that the exact exported source has canonical formatting. It must not rewrite working files. Any discrepancy means the source tree needs deliberate formatting and another local test cycle.

### Locked compilation

`cargo check --workspace --locked` checks the full workspace. `--locked` prevents accidental lockfile changes during resolution. For local source export, the Cargo registry cache may still supply previously downloaded packages; this is explicit, not hidden.

### Full tests including cryptography

`cargo test --workspace --locked` covers the standard unit and integration suites. It includes actual TFHE encryption/decryption tests and may take significantly longer than frontend tests. Tests deliberately marked `ignored` are not automatically run by this command and must be identified separately if release requirements depend on them.

The one-iteration clear-circuit benchmark from C2 was a CLI functionality check, *not* reliable evidence of general TFHE speed or calibrated prediction accuracy. Stage D makes no claim of better performance than the previously measured model.

### Zero-warning lint

`cargo clippy --workspace --all-targets --locked -- -D warnings` checks examples, binaries, libraries, integration tests, and bench targets exposed by Cargo. Unlike merely inspecting standard output, the `-D warnings` gate returns failure on warnings. Do not disable Clippy lints just to get a green build without first establishing an intentional exception.

### Release binaries

`cargo build --release --locked -p fhec-cli -p fhec-bench --bins` produces the CLI and both benchmark package binaries under the active Cargo target directory. FHEC's current executable is named `fhec-cli(.exe)`, not `fhec(.exe)`. The `bench` command may delegate to Cargo, and therefore continues to require Cargo and the repository checkout.

### Real executable smoke test

The release CLI is executed directly—not with `cargo run`—to check help, source-to-circuit `build`, and `analyze` on `examples/conditional_mux.fhe`. A nonzero return is a failure. This is a usability gate and does not supplant the real TFHE integration tests.

### Metadata and dependency scan

The inventory reports resolved packages, versions, license expressions, and source registries. If installed, `cargo audit` queries RustSec. A clean metadata report is not equivalent to license clearance; a skipped `cargo audit` is **never** a passing vulnerability scan.

## 5. Hosted Windows/Linux matrix

The workflow `.github/workflows/fhec-v1-crossplatform.yml` is deliberately additive. Each OS job:

1. Performs a fresh checkout of the triggering commit, with persisted Git credentials disabled.
2. Installs a stable Rust toolchain with rustfmt and Clippy.
3. Runs all locked workspace checks, full TFHE tests, zero-warning lint, release builds, and CLI smoke tests.

There is an independent Ubuntu RustSec job that installs cargo-audit and exits unsuccessfully if advisories or tooling errors prevent a clean audit. This makes dependency advisories a visible build gate, not a paragraph in the README.

The workflow triggers on push, pull request, and manual dispatch. **It cannot run until the file exists in a reachable GitHub repository.** It creates no tag, pushes no commit, and uploads no release assets. Before pushing, check existing workflow files too—an older `.github/workflows/ci.yml` may still be triggered and require attention.

When inspecting hosted evidence, always record the exact full commit hash. A successful run from an earlier commit is insufficient to certify a newer release candidate. Record OS runner labels and workflow run URLs separately; the local script does not know them.

## 6. Dependency security and third-party licensing

FHEC includes the TFHE Rust backend and other direct and transitive crates. Dependency security review is not a theoretical formality: resolved package versions are fixed by Cargo.lock and can have known advisories independent of source correctness.

The inventory reads the actual locked resolution with `cargo metadata --locked --format-version 1`. It lists package source and each crate's declared license expression. Entries with missing license fields, GPL/AGPL/LGPL/MPL/EPL family expressions, or git-sourced package dependencies receive attention in `license-review.txt`. These are **review candidates**—the script does not automatically decide legal compatibility.

`cargo-audit` uses the RustSec database to detect known Rust advisories. Running the audit successfully is still not a guarantee against undisclosed vulnerabilities, malicious crates, or unsafe build scripts. Record time and advisory database version when completing security signoff.

A dependency's own repository and license file can be relevant even when its Cargo metadata appears permissive. Ensure proper attribution and notices before redistributing source or binaries. Keep project-level `LICENSE` consistent with `workspace.package.license`.

## 7. Failure classes and recovery

| Failure | Interpretation | Appropriate response |
|---|---|---|
| D installer rejects an existing file | A path already exists with differing contents | Do not force overwrite; review the file and decide whether the bundle is appropriate for this workspace |
| Required C3 file missing | Source layout predates verified C3 | Restore the verified prerequisite or stop; do not fabricate it |
| Export reports symlink or reparse point | Selected tree could escape the project boundary | Inspect the path, reject if it points outside intended sources |
| Export reports suspicious credential content | A possible secret is inside release-selected text | Remove/redact the secret intentionally; do not disable the scan blindly |
| Export omits an essential include file | Whitelist is too narrow for a real source dependency | Review the asset before adding its extension/path |
| Cargo cannot download a crate | Network/cached dependency issue | Check connectivity, registry configuration and lockfile; do not delete Cargo.lock |
| Locked compilation fails | Source or dependency constraint problem | Investigate the first compiler error, fix source separately, then repeat the full gate |
| Real TFHE integration test is slow | Cryptographic tests can run for a long time | Let live output identify the test; distinguish slow progress from an actual hang |
| Clippy reports an issue | A source/test warning under `-D warnings` | Fix the smallest actual issue and rerun all affected gates |
| Linux hosted compile fails | Platform-specific bug or missing native prerequisite | Use the exact CI error; verify supported platform/toolchain, then rerun |
| RustSec reports an advisory | A locked dependency may have known vulnerability | Determine reachability, fixed versions and upgrades; never unconditionally suppress |
| License inventory has unknowns | Declared package metadata is incomplete | Inspect the package license/notice at its source |
| D local result passes but CI has not run | Evidence exists for Windows workstation only | Leave hosted matrix checkboxes open |

Nothing in Stage D automatically rewrites source or undoes a developer's ongoing changes. The exporter removes only a **new destination copy** if that export fails; the original project remains intact.

## 8. Release staging discipline

The user explicitly prefers postponing Git commits and public publication until the FHEC V1 product is ready. That is respected here. When the final release preparation begins, use careful, explicit staging rather than indiscriminate `git add .`.

Review the entire working tree, including untracked files; review every staged file and diff; run secret scanning appropriate to the organization; and verify that `.gitignore` excludes benchmark reports, `target`, backups, archives, and local TFHE profiles. Do not commit code containing private keys or non-public input values.

Only after that review should a first clean source commit be prepared. The commit must contain the **same files and semantics** that the clean source gate already validated. A release may need an explicit version choice and a README/changelog reconciliation, but the exact version has not been chosen here. Avoid describing the current `0.1.0` Cargo package version as a tagged `v1.0.0` release by implication.

Once GitHub runs the matrix on the exact candidate commit, inspect the source archive derived from that commit, validate the release build output, and record the tag. Any source change after CI success requires renewed validation.

## 9. Audit evidence to preserve

Record these facts when checks pass:

- Full `cargo --version` and `rustc --version` output, workstation OS information, and validation timestamp.
- The clean-export path and its SHA-256 manifest, including the total file count.
- Formatting, workspace check, full test, Clippy, and release-binary gate results.
- Dependency inventory and manual review notes; `cargo audit` outcome or explicit `NOT RUN`.
- The set of ignored tests and why they remain ignored.
- The eventual CI run URLs and exact GitHub commit hash for Windows, Ubuntu and RustSec.
- The final source archive contents and final version strings.

A precise evidence trail is much more useful than a rough percentage-complete estimate.

## 10. Honest interpretation of a green Stage D local gate

If the terminal shows `STAGE D LOCAL CLEAN-SOURCE VALIDATION PASSED.`, you have gained meaningful confidence that the source compiles cleanly without the prior `target` directory and with an explicit release-like file list on Windows. You have *not* proven that Linux works, that GitHub will include the same file list, that the vulnerability scan ran if cargo-audit was absent, or that the release is safe to publish. The remaining signoff checks are in `D_ACCEPTANCE.md`.

Stage D should be closed only with the combined clean-source, vulnerability/license review, final staged/committed file inspection and hosted cross-platform evidence. This avoids shipping a demo simply because one happy-path build passed.
