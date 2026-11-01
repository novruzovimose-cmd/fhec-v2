# CoFHE Backend Compatibility Audit Worksheet

This table is a **verification worksheet**. Every "not yet verified" entry means this Phase 13 release has no sufficient fresh evidence for that feature, not that the underlying compiler certainly lacks it. Confirm behavior against a pinned `FHE.sol` and SDK release before changing any status.

## Pinned snapshot

The local V2 snapshot inspected for this package specified:

- `@cofhe/sdk`: `^0.7.1`
- `@cofhe/hardhat-plugin`: `^0.7.1`
- `@fhenixprotocol/cofhe-contracts`: `0.2.0`

Upstream CoFHE packages evolve. Some public examples have already migrated to newer contract and SDK versions. Use separate compatibility branches. A silent package upgrade can invalidate source seals, external proof formats and Solidity function signatures.

## Coverage taxonomy

| Feature family | Locally reported evidence | Production-grade requirement |
| --- | --- | --- |
| Encrypted unsigned euint8/euint16/euint32/euint64 | Phase 12 selected-width mock coverage | Test exact pinned upstream type and live ABI |
| Add/sub/mul | Phase 12 selected operation-width families | Optimized/unoptimized output oracle on live testnet |
| Greater-than/selection/repeated computation | Phase 12 selected operation-width families | Live comparison, eager-arm semantics, cost/effect review |
| Encrypted boolean arithmetic | Not sufficiently established by Phase 12 matrix | Full parser/typecheck/emission/mocks/live proofs |
| Equality/less-than/less-or-equal/greater-or-equal | Not sufficiently established | Explicit semantics and result-type tests |
| Division/remainder, if present in pinned CoFHE API | Not sufficiently established | Zero handling, width/overflow semantics and async failures |
| Bitwise and/or/xor/not/shift operations | Not sufficiently established | Type correctness, ciphertext domain and live oracle |
| euint128 and wider integers, encrypted address | Not sufficiently established | Exact type compatibility and ABI tests |
| Casts and encrypted selection across types | Not sufficiently established | Width, bit-order and overflow tests |
| Contract-address-bound external input proofs | Negative mocks reported in prior phases | Current SDK version, real testnet signer/domain checks |
| ACL/permission lifecycle | Selected negative mock checks reported | `allow`, user-only decrypt, revocation, cross-user rejection |
| SDK-based decryption | Mock test coverage reported | Real network authorization and finite timeout behavior |
| Async coprocessor task failures | Not sufficiently established | Retries, finality, interrupted work, no duplicate billing assumption |
| Static expanded call graph and operation budget | Local optimizer/cost-policy tests | Model limitations and live comparison clearly separated |
| CoFHE off-chain task pricing | Not measured by local mocks/native gas | Independent provider task accounting |
| Production deployment security | Not established | Independent audit and remediation |

## Rules for upgrading a row to verified

Each feature needs the following evidence:

1. The pinned upstream library supports the exact operation/type/signature.
2. The FHEC language recognizes and type-checks valid use and rejects incompatible use.
3. The Solidity emitter produces the correct import, type, call and necessary permission behavior.
4. Golden output comparisons against handwritten reference compile correctly.
5. Encrypted local mock cases pass positive and negative tests.
6. A separately authorized real network execution matches the independent plaintext oracle.
7. The result is recorded under an immutable revision with reproducible test output and limitations.

Only after all seven should the row say "real-network verified" for its exact versions and workloads. There is no meaningful universal "100% CoFHE" certificate based solely on a project phase number.
