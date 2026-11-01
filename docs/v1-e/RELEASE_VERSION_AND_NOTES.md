# Release version and changelog decision record

## Unresolved: choose the public version deliberately

The last verified D version checker describes the workspace as `0.1.0` and Apache-2.0, with 12 workspace packages. The final user-facing release title may describe **FHEC Compiler V1** while the Cargo package version remains `0.1.0`, but that distinction needs to be intentional, documented and consistent with packaging policies. No automation in this package rewrites manifests or declares a public `v1.0.0` tag.

**Record these facts before publication:**

| Item | Candidate decision | Evidence to attach |
|---|---|---|
| Public version/tag | **TBD** | Approved tag, e.g. `v1.0.0` or `v0.1.0` |
| Workspace package version | Read from current `cargo metadata --locked` | `reports/version-consistency.txt` |
| Executable name | `fhec-cli` on the observed V1 checkout | Clean build artifact names |
| Package identity | Rust workspace with 12 crates | `Cargo.toml` / metadata |
| Project license | Apache-2.0 | Full root `LICENSE`, manifests |
| README release/version terminology | Review, update if necessary | Exact README revision |
| CHANGELOG release section | Review, update if necessary | Exact changelog revision |
| Release-note title and links | **TBD** | Approved artifact text |
| Release tag commit | **TBD** | 40-character commit SHA |

If you choose to publish `v1.0.0`, change and validate the Rust manifests, lockfile, README, and changelog as required *before the final clean-source run*, rather than publishing a tag that misleads readers about the actual package version. Never casually rewrite `Cargo.lock` or guess that a printed analyzer version is the release version.

## Important distinction: Analyzer V5.6

The CLI's `FHEC COMPILER V5.6` or Analyzer V5.6 report banner describes an internal report/model generation. It is **not** proof that the distributable Cargo compiler is at semantic version 5.6. Public documentation should explicitly distinguish the analyzer/report banner from the package/release version.

## Proposed user-facing release note (draft; verify against release commit)

> FHEC Compiler V1 introduces a Rust-based, TFHE-backed compiler toolchain for a deliberately limited encrypted-language subset. It includes typed source analysis, branch-to-MUX lowering, encrypted Boolean and fixed-width integer circuits, alternative candidate implementations, optimizer passes, cost-analysis reports, hardware-profile calibration, benchmarking and real TFHE encrypted execution. It is not a general-purpose Rust compiler, not an OpenFHE backend and not independently certified cryptographic software.
>
> Supported workflows include `build` (compile/verify, not a standalone binary artifact generator), `analyze`, and `run`, plus profile/benchmark commands requiring the appropriate Rust/Cargo environment. The executable is presently named `fhec-cli`, and `bench` may require a source checkout. Performance predictions without an available calibrated hardware profile can display `0.000 ms`; this is unknown or uncalibrated reporting, not evidence of instantaneous encryption.
>
> The bounded V1 language supports selected encrypted scalar types and operations; consult `docs/v1-c3/LANGUAGE_REFERENCE.md` for the authoritative limitations. FHEC does not claim universal FHE noise management or cross-platform guarantees until the hosted workflow checks actually pass on the release commit.

Replace the quote above with release-commit-verified language, link actual documentation, and add the exact version and commit. Do not claim that FHEC is the first or fastest of its kind unless an independent reproducible comparison has demonstrated that claim.

## Release archive names

Stage E's local candidate archive is **not** a GitHub release artifact. Choose the official archive format and versioned filename only after deciding the public package version, reviewing the license notices, and obtaining CI evidence. The official GitHub-generated source archive must be separately compared with the committed source contents; merely hashing a local archive is insufficient.
