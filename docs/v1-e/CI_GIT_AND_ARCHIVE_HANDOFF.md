# Stage E to final Git and CI handoff

The user explicitly wants publication **only after a finalized V1**. Stage E therefore performs no Git mutation. This checklist exists so the later Git/publishing work can be done quickly and without accidental inclusion of local materials.

## Why D is still open

The last observed Stage D result was `STAGE D LOCAL CLEAN-SOURCE VALIDATION PASSED`, followed by `HOSTED WINDOWS/LINUX CI IS NOT YET VERIFIED`. The RustSec audit was also unavailable. That is successful **local** evidence, not complete cross-platform release evidence. It is misleading to declare Stage D closed until the exact final commit has passed hosted Windows, Ubuntu and advisory jobs.

## Preparing the exact release commit (only after final local signoff)

1. Inspect `git status --short` and every untracked file. Stage only the precise source, example, documentation, license and workflow paths that belong in the release. Do **not** use indiscriminate `git add .`.
2. Review staged paths using `git diff --cached --name-status`, then inspect sensitive or surprising changes with `git diff --cached`. Verify generated `target/`, local benchmark reports, archives, shell histories, temporary patches, profile data, backup `.rs` files and secrets are excluded.
3. Check the documented public version against all Cargo packages, README and CHANGELOG. If you edit anything, rerun the source-export build and relevant compiler gates.
4. Create the clean local commit deliberately. An unpublished local Git commit does not itself create a public demo; **pushing** makes content available to GitHub.
5. Push that commit only when prepared for a GitHub-hosted source checkout, then allow GitHub Actions to run on Windows and Ubuntu. This is not equivalent to publishing a tagged GitHub Release.
6. Open the `FHEC V1 Cross-platform Release Gates` workflow, the RustSec audit job and any older CI workflow. Confirm all required jobs are green on the same full commit hash.
7. Check out the exact commit in a fresh directory and repeat locked build/test/release checks, or verify the GitHub workflow logs demonstrate them on the exact hash.
8. Review GitHub's source archive for **the commit/tag being released**. Inspect SHA-256, complete file list, backups, profiles, full LICENSE, examples, docs and binary/source distinction.
9. Publish/tag only once the complete release checklist is approved. A green local E candidate archive is not permission to publish.

## Evidence ledger to fill later

| Evidence | Expected | Actual |
|---|---|---|
| Full release commit SHA | 40 hex digits | **TBD** |
| Public repository URL | User-owned GitHub repository | **TBD** |
| Intended tag | Consistent with Cargo/README/CHANGELOG | **TBD** |
| Hosted Windows run URL | Successful on exact SHA | **TBD** |
| Hosted Ubuntu run URL | Successful on exact SHA | **TBD** |
| Hosted RustSec run URL | Successful on exact SHA | **TBD** |
| Final clean-checkout log | Locked tests + Clippy + release build | **TBD** |
| Released source archive digest | SHA-256, exact commit | **TBD** |
| Dependency license signoff | Completed | **TBD** |
| Publish approval | Explicit | **TBD** |

## Do not mistake these for final proof

- Local Stage D and Stage E results are not an exact GitHub commit validation.
- A candidate ZIP made before a source change is obsolete after that change.
- An analyzer printed estimate of `0.000 ms` under an uncalibrated model is not a valid empirical speed comparison.
- An ignored test does not count as a passing test, even if the surrounding suite reports success.
- `cargo audit` being absent is not evidence that dependencies are safe.
- GitHub workflow files existing locally does not mean the hosted jobs have run.
- GitHub Actions job names and links cannot be inferred from this document. Record them from actual run results.
