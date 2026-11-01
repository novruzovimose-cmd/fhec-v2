# TFHE runtime reliability and security notes (V1 audit draft)

## Supported cryptography

FHEC V1 uses the `tfhe` crate's Boolean gate API. The backend owns a client key and a server/evaluation key. The client key is used to encrypt and decrypt Boolean values; the server key evaluates Boolean gates without consulting the client's plaintext inputs. This describes the architecture, not an independently audited cryptographic guarantee.

A TFHE Boolean ciphertext represents one bit. Signed and unsigned integer source-language values are represented by circuits over encrypted bits, not by a native TFHE integer/radix backend.

## Execution boundaries

The circuit verifier rejects invalid gate dependencies before the backend executes. The sequential, parallel, and chunked executors propagate backend operation errors via `ExecutionError`; auto-selection layers wrap analysis and execution errors separately. Applications must avoid logging plaintext inputs, decrypted outputs, client-key objects, or serialized secret-key material.

FHEC does not currently provide a persistent client-key store or a security-hardened key-lifecycle API. Key generation is performed by `TfheBackend::new()`; the key resides in the backend object for its lifetime. Do not claim memory zeroization, key custody, secure enclaves, or multi-party key separation without independently reviewing and implementing them.

## Calibration profiles

Saved JSON profiles contain measured operation timings, scaling curves, a hardware fingerprint, profile metadata, and ciphertext size. They should not contain client keys or ciphertext payloads. The loader rejects incompatible metadata, invalid measurements, malformed JSON, and profiles above the configured file-size limit. Invalid calibration data is not evidence of decrypted values leaking.

A profile is a *performance input*, not a cryptographically trusted configuration. Profiles are not signed or authenticated. Treat profiles from untrusted parties as untrusted measurement data and do not assume their latency predictions are correct. JSON save currently uses ordinary file replacement rather than an independently verified crash-safe atomic write.

## Bootstrapping and noise

The reported Boolean gate bootstraps and key switches come from FHEC's declared operation model, not a hardware counter or a proof that an individual TFHE call performed exactly the reported primitive count. Gate-bootstrapped TFHE Boolean evaluation is distinct from leveled noise-budget management. Analyzer noise risk is a modeled indicator and is **not** a verified decryption-failure probability or a security parameter estimate.

V1 does **not** implement automatic noise scheduling for BFV/BGV/CKKS, multi-scheme switching, or custom cryptographic parameter selection.

## Follow-up audit items

- Verify key-material handling and error messages along actual CLI execution paths.
- Review dependency advisories and third-party licenses before release.
- Verify profile write interruptions, permission failures, and corruption/recalibration behavior in a clean Windows and Linux environment.
- Run supported width/depth matrix tests on the final release commit.
- Confirm source control and release archives exclude local secrets, private profiles, and benchmark data.
