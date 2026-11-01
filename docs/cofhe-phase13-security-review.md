# FHEC / CoFHE Independent Security Review Request

## Scope and prerequisite

Request external review of the exact publicly accessible FHEC V2 source revision and its pinned CoFHE Solidity dependencies. Provide the Phase 12 local qualification candidate and any later independently verified Phase 13 testnet receipts. Reviewers should not be asked to endorse a generic claim that the system is production-secure or supports every CoFHE API.

## Reviewer workstreams

### Compiler semantics

- Model function call expansion and encrypted selection eagerness, especially selected values with encrypted conditions.
- Compare source-level widths and signedness against actual Solidity and CoFHE encrypted types.
- Inspect all algebraic simplifications for identities that are safe in modular encrypted arithmetic.
- Distinguish structural common subexpression sharing from potentially nondeterministic ciphertext operations and ACL side effects.
- Test arithmetic wraparound and boundaries at 0, 1, maximum values, and high-order bit transitions.
- Check that failed type analysis cannot produce a partially trusted emitted contract.
- Check errors do not echo plaintext CLI inputs or private proof material.
- Verify every unsupported language construct and backend operation is rejected rather than silently transformed.

### Contract and encryption boundaries

- Validate use of `FHE.sol`, imported library versions and user-controlled `external*` proof conversion.
- Verify consuming-contract binding and signer/domain separation for SDK-generated proof batches.
- Validate permissions to read, transform, store, share and decrypt encrypted handles.
- Test contract-address confusion, chain-ID replay, signer substitution, stale proof, cross-contract ciphertext re-use, and incorrect caller authorization.
- Verify no encrypted boolean or integer is accidentally exposed via events, storage wrappers, revert messages, logs or backend telemetry.
- Test asynchronous task availability, timeout, partial failure and retry/replay handling.
- Compare security properties under local mocks and actual testnet protocol behavior.

### Build and release

- Review dependency lockfile provenance and known advisories.
- Reproduce Rust CLI output in a separate environment and compare exact generated Solidity.
- Run both Windows/Linux hosted jobs at the reviewed Git SHA.
- Review installer path controls, source-seal logic, rollback behavior and evidence manifest weaknesses.
- Confirm that CI workflows cannot execute a paid testnet run from untrusted pull requests.
- Inspect test wallet isolation and ensure secrets are neither printed nor committed.

### Protocol economics and reliability

- Compare optimized generated, unoptimized generated, and handwritten Solidity under identical testnet conditions.
- Keep mock gas, native-chain gas, coprocessor task charges, SDK encryption time and decryption delay as different metrics.
- Inspect completed, failed and timed-out tasks and evaluate exactly-once/retry/idempotency assumptions.
- Ensure statistically weak comparisons are marked exploratory and regressions are disclosed.

## Requested deliverables

1. Reviewer organization/author and public or verifiable contact.
2. Exact reviewed Git commit and CoFHE SDK/contract versions.
3. Threat model, assumptions and excluded components.
4. Findings with severity, reproduction instructions and affected revision.
5. Remediation commits and reviewer confirmation of fixes.
6. A signed report or a public report URL whose authenticity can be independently checked.
7. Explicit distinction between an assessment, formal audit and cryptographic certification.

## Status

At Phase 13 installation this independent review remains **PENDING**. Passing local tests does not alter that status. The grant reviewer should see unresolved findings and limitations rather than a fabricated all-green audit badge.
