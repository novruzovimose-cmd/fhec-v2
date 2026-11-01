# FHEC V1 — Stage B3 security and reliability review

## Confirmed implementation properties

- `TfheBackend` stores `ClientKey` and `ServerKey` in private fields. The public `FheBackend` API encrypts and decrypts with the client key and performs Boolean AND/OR/XOR/NOT/MUX with the server key. V1 runs both sides locally, so a compromised host can access plaintext and secrets.
- The circuit executor verifies gate order and operand references before encrypted evaluation, and its public execution strategies return errors on malformed inputs and missing outputs. The B1/B2 runtime failure and shape tests verify these boundaries.
- Persisted V4 profiles contain measured timings, declared gate costs and hardware metadata, not keys or encrypted payloads. B3 rejects impossible width configurations and protects writes against ordinary partial-file replacement. Profiles are untrusted performance data, not authenticated cryptographic settings.
- Saving profiles stages and syncs JSON before replacing the destination. On Unix, rename over a regular file is atomic at the directory-entry level. On Windows, the standard-library implementation uses a backup-and-restore swap because replacement rename may fail; that swap is **not crash-atomic**. Interrupted Windows writes can leave a backup or missing main profile. FHEC can recalibrate a missing/invalid profile, but does not automatically restore orphaned backups.
- The actual TFHE Boolean MUX regression tests all eight combinations of encrypted condition and branch values. This is functional coverage, **not** a formal proof of constant-time behavior or of resistance to side channels.

## Scope and limitations

- The evaluator uses Boolean TFHE gates with backend-managed bootstrapping. Analyzer bootstrapping/key-switch counts are declared estimates, not instrumentation of the cryptographic implementation. No BFV/BGV/CKKS noise-budget scheduler, custom parameter selection or multi-party client/server security boundary is implemented in V1.
- Client keys are generated in process; the project has no encrypted key store, explicit key zeroization, protected memory, remote trust boundary, independent cryptographic audit or certification.
- `--inputs-stdin` reduces exposure in process arguments, but shell history, terminal echo, process memory, OS monitoring, decrypted output and CI logs can still expose information.
- Profile persistence assumes a single trusted writer and a trusted local profile directory. The final-component regular-file/symlink check is defense in depth, not protection against an attacker who can concurrently rewrite the parent directory.

## Release checklist still requiring independent evidence

- Run real TFHE integration tests, full workspace tests, Clippy and Windows/Linux CI on the final release commit.
- Review third-party dependency vulnerability advisories, licenses and the complete release archive for embedded secrets or private runtime traces.
- Confirm profile-recovery and filesystem-permission behavior on Windows as well as Linux. Treat the Windows backup swap as non-atomic until a stronger platform-specific replacement primitive is implemented.
