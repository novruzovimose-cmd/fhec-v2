# Release contents and accidental disclosure review

## Why this matters

The current Windows development workspace has historically contained `.backup.rs` files, temporary patch scripts, compiler output, untracked logs, and calibration data. These are not safe to ship indiscriminately. Do not use `git add .` as the release staging strategy.

## Export policy

`tools/d-release/export-source.ps1` builds a new source tree from an allowlist. It does not delete or rewrite the originals. The selected sources must be files inside approved directories and must not be reparse points/symbolic links. Included files are hashed before and after copying. A manifest is placed *outside* the exported source root.

Approved top-level inputs: Cargo.toml, Cargo.lock, rust-toolchain.toml, LICENSE, README.md, CHANGELOG.md, `.gitignore`, and a small group of optional community/configuration metadata. `crates/**` is restricted to Rust/Cargo/textual code and test fixtures. `docs/**`, `examples/**`, `.github/workflows/**`, and `tools/d-release/**` have narrower extension rules. Other scripts under `tools/` are not included except the explicit C3 verifier and manifest.

Rejected or excluded paths include `.git`, `target`, `.env` files, private-key extensions, archives, `*.log`, `*.patch`, `*.backup*`, `*.bak*`, corrupted sources, and backup directories.

## Limits of automated review

Pattern matching catches some private-key headers, GitHub tokens, and AWS-style access keys. It is not a comprehensive secret detector. It does not examine opaque file formats or run a malware scanner. The user must manually review the final Git staged paths and staged diff before a commit. Paths excluded by the exporter are **not automatically excluded by Git**; check `.gitignore` and staging separately.

## Final staged-content criteria

- No real profiles or keys, credentials, environment files, private inputs, or personal data.
- No local logs, benchmark generated data, `target`, `.zip`, backup files, or patches.
- Full LICENSE and matching Cargo metadata.
- README, changelog, examples, docs, workflow, and source crate tree present.
- No obsolete placeholder code or broken test fixtures knowingly shipped.
- Actual archive generated from the final *commit*, not a random working directory snapshot.

The Stage D exporter is deliberately not an official Git source archive. Its purpose is to prove that a known source file set builds cleanly without local target output.
