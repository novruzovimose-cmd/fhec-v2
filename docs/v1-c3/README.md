# FHEC V1 - Stage C3 documentation entry point

This is the **source-grounded, testable** V1 documentation set prepared after local C1 and C2 acceptance. It does not assert that a public V1 release, Linux CI, clean checkout, vulnerability scan, or third-party cryptographic certification has occurred.

Start with [the full engineering and user guide](FHEC_V1_COMPLETE_GUIDE.md). Use the focused companions when working on a specific subsystem:

| File | Purpose |
| --- | --- |
| [FHEC_V1_COMPLETE_GUIDE.md](FHEC_V1_COMPLETE_GUIDE.md) | Full supported scope, workflows, architecture, limitations, operator matrix, and reproducibility |
| [LANGUAGE_REFERENCE.md](LANGUAGE_REFERENCE.md) | Language grammar, supported types, semantics, examples, and rejected constructs |
| [ANALYZER_AND_OPTIMIZER.md](ANALYZER_AND_OPTIMIZER.md) | Candidate reports, intervals, cost models, hints vs implemented actions, noise terminology |
| [BENCHMARKS_AND_CALIBRATION.md](BENCHMARKS_AND_CALIBRATION.md) | Real TFHE vs clear-circuit timings, calibration rejection, profile use, experimental protocol |
| [SECURITY_AND_RUNTIME.md](SECURITY_AND_RUNTIME.md) | Local trust assumptions, key handling, redaction, file integrity, noise scope |
| [DEVELOPER_INSTALL_AND_TROUBLESHOOT.md](DEVELOPER_INSTALL_AND_TROUBLESHOOT.md) | Windows/Linux build prerequisites, commands, common errors, exact executable name |
| [C3_ACCEPTANCE_AND_REMAINING.md](C3_ACCEPTANCE_AND_REMAINING.md) | What C3 verifies, what it does not, and D-G release requirements |

## Related documents already present in the project

- `docs/architecture.md` - prior pipeline design; where implementation differs, use tested current code behavior.
- `docs/benchmark-methodology.md` - C1 benchmark commands, expected output types, statistical limitations.
- `docs/cli-workflow-c1.md` - CLI developer workflow.
- `docs/c2-cli-contract.md` - C2 executable acceptance contract, including redaction and `--inputs-stdin`.
- `docs/tfhe-runtime-security-review.md`, `docs/tfhe-cli-input-security.md`, `docs/tfhe-b3-security-signoff.md` - B-stage boundaries and remaining security obligations.
- `docs/v1-feature-contract.md`, `docs/v1-security-model.md` - earlier release drafts; reconcile these against the final public contract before tagging.

## C3 installer and checks

The C3 installer **adds** documentation, four sample `.fhe` programs, and a live PowerShell verification tool. It never edits existing Rust files, manifests, the lockfile, or Git history. Existing conflicting files cause it to stop before making changes. It checks each copied file's SHA-256 digest and removes only files it created if installation fails.

From an existing PowerShell session in the repository:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$HOME\Downloads\fhec-c3\install.ps1" -Project (Get-Location).Path
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "tools\verify-c3.ps1" -Project (Get-Location).Path
```

`-FastOnly` skips the full workspace cryptographic test suite. A fast run is **not** sufficient for full C3 acceptance; without `-FastOnly`, the verifier runs the full test suite and checks the release CLI directly.

## Version language

**FHEC V1** is the planned product release. The Rust workspace can still have manifest version `0.1.0`; do not silently equate the two. The CLI may print `FHEC COMPILER V5.6` in an analyzer section; that is an internal analyzer/compiler report-generation label, **not** a released package version.

This pack is documentation and regression verification, not a claim of audited production security.
