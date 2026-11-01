# FHEC V1 security model â€” draft

## Scope

FHEC V1 compiles supported encrypted-language programs into Boolean circuits and executes them with TFHE. The underlying TFHE implementation is a separate cryptographic dependency.

## Boundaries

- Encrypted values are represented as ciphertexts in the TFHE-backed execution path. Inspect all plaintext conversions and debug/reporting before release.
- Encrypted conditional logic must be lowered to circuit operations, never host-language branching on decrypted secrets.
- A successful compiler verification does not establish protection against all side channels, unsafe hosting environments, or malicious toolchains.
- Hardware profiles, analyzer timings, predicted noise, and optimization hints are operational metadata, not a proof of confidentiality.
- Backend gate bootstrapping does not imply FHEC has a general-purpose noise scheduler for other FHE schemes.

## Must verify before public V1 tag

- [ ] No secret key or plaintext secret is logged or persisted unintentionally.
- [ ] Malformed source and IR fail with diagnostics instead of release-path panics.
- [ ] Encrypted/plaintext operation restrictions are enforced.
- [ ] Invalid encrypted control flow is rejected or lowered securely.
- [ ] Runtime parameter and profile failures are handled without exposing sensitive data.
- [ ] Dependency and license review is complete.

**Security statement:** FHEC has not received an independent cryptographic security audit. Do not use this project as a standalone security certification or production safety guarantee.
