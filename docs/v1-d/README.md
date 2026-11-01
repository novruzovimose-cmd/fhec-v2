# Stage D — clean-source builds, platform CI, and release audits

**Status:** Tools configured; hosted CI is not proven until a push triggers the workflow and both operating-system jobs pass on the exact release commit.

Stage D is intentionally **not a publication step**. The repository remains local. The scripts do not call `git add`, `git commit`, `git push`, tag a release, modify Cargo.toml, or upload source archives.

## Commands on the Windows development workstation

At the PowerShell prompt, from the repository root:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\d-release\verify-local.ps1 -Project (Get-Location).Path
```

The command exports a filtered copy to a timestamped folder under `target/fhec-d-validation`, verifies every copied file with SHA-256, and runs the full Cargo test/build/release gates with a **fresh Cargo target directory**. Progress from Cargo is displayed live, including real TFHE tests, which can be slow. It exports source only: not Git history, target folders, private profiles, source backups, or benchmark artifacts. The current user's Cargo registry cache and installed Rust toolchain are reused by default; this is not a clean-room operating-system test.

To additionally start with an empty Cargo registry cache, use `-ColdCargoHome`. This may download many dependencies and requires network access. To require an installed `cargo-audit` to succeed, add `-RequireAudit`. The default audit mode reports *not run* when cargo-audit is missing; it is never presented as a clean security audit.

To run fast gates while debugging use `-FastOnly`; **that does not qualify as full local Stage D evidence**.

### Expected local result

`STAGE D LOCAL CLEAN-SOURCE VALIDATION PASSED.`

That confirms only the local Windows snapshot gate. It does not prove that the existing Git staging area is clean, that Windows and Linux GitHub-hosted CI pass, or that the exact eventual commit builds. These remain release blockers.

## What the source-only export includes

- Root Cargo manifests and lockfile, Rust toolchain specification, Apache-2.0 license, README, changelog, and `.gitignore`.
- Rust crates' source and test files, Cargo manifests, approved textual test fixtures.
- Markdown/JSON documentation, source examples, GitHub workflow definitions, and a limited set of release/verification scripts.

The exporter refuses symlinks and selected files containing recognizable private-key headers or known credential-token formats. It excludes directories commonly used for generated output, temporary archives, backups, and private state. **This automated scan cannot prove the absence of all secrets.** Review the manifest and eventual Git staged diff manually.

Look at `target/fhec-d-validation/<run-id>/source-manifest.json` and `.../reports/dependency-inventory.json` after a successful run.

## GitHub-hosted checks after the repository is ready

`.github/workflows/fhec-v1-crossplatform.yml` is an additive workflow. Once pushed, it runs fresh `actions/checkout` operations on `windows-latest` and `ubuntu-latest`. Each runs formatting, workspace check, full TFHE-inclusive tests, zero-warning Clippy, release builds, and direct binary smoke tests. A third Ubuntu job installs cargo-audit and checks the RustSec advisory database. Failure in any job is a release blocker pending investigation.

Other existing workflows (such as `.github/workflows/ci.yml`) still run independently and must be examined in the release review. Installing this new file does not prove that a workflow is passing; GitHub must report success for the exact release commit.

The workflow does **not** publish crates, push commits, create tags, upload release binaries, or expose credentials. Its source checkout disables stored credentials.

## Remaining D/E/F/G release blockers

- Complete the advisory audit and manually review any non-permissive or unknown dependency licenses.
- Review current `.gitignore`, untracked files, generated artifacts, and the **actual staged diff** before committing.
- Produce the first clean Git commit and confirm it matches the source-only inventory.
- Push and obtain successful Windows and Linux GitHub Actions results.
- Repeat exact-commit release validation; reconcile README/changelog/manifests and assemble final artifacts only at publication stage.

See [MASTER_RELEASE_ENGINEERING_GUIDE.md](MASTER_RELEASE_ENGINEERING_GUIDE.md) for the complete runbook, [RELEASE_REVIEW_TEMPLATE.md](RELEASE_REVIEW_TEMPLATE.md) for exact-commit documentation, [D_ACCEPTANCE.md](D_ACCEPTANCE.md) for evidence and signoff boxes, [RELEASE_CONTENTS.md](RELEASE_CONTENTS.md) for the source-export policy, [DEPENDENCY_AUDIT.md](DEPENDENCY_AUDIT.md) for review rules, and [CI_AND_REPRODUCIBILITY.md](CI_AND_REPRODUCIBILITY.md) for environment boundaries.
