# FHEC V2 CoFHE: scope and security limitations

## In scope for the submitted demo

- A Rust frontend and typed CoFHE Solidity emitter operating on the supported encrypted subset.
- Encrypted `u8`, `u16`, `u32`, and `u64` operations, encrypted booleans, local values, supported helper calls, conditional selection, and controlled constant lowering.
- Rejection tests for unsupported types and unsupported constructs in the current backend.
- Two Solidity consuming wrappers and 24 local CoFHE mock tests, including malformed authorization cases.
- A pinned `pnpm-lock.yaml` inherited from the developer's tested Fhenix Hardhat starter environment.

## Not claimed

- Any independent audit or formal verification of FHEC or the generated contracts.
- Live CoFHE or public-testnet execution, real-world threshold decryption, mainnet approval or mainnet readiness.
- Production gas, latency, throughput, or financial benefit measurements.
- Arbitrary Solidity generation, full language support, or compatibility with every CoFHE SDK version.
- Proof that all source code is constant-time at the host/runtime level.
- Production security properties inferred solely from local mock behavior.

## Threat and trust boundaries

The demo's negative tests establish that this pinned mock implementation rejects several incorrect encrypted-input proofs and unauthorized view-decryption requests. These are **regression assertions**, not cryptographic proofs of security. The generated Solidity relies on the semantics and security of the imported Fhenix contracts, plugin, SDK, task manager and configured coprocessor network.

Do not attach real private keys or RPC credentials to the grant-demo repository. The bundled Hardhat config uses only its default local network. Do not use live customer data, production funds, or real accounts for this demonstration.

## Follow-up validation gates

1. Independently run the fresh-checkout CI on the exact commit and both runner platforms.
2. Review third-party dependency licenses and the RustSec/npm dependency advisories.
3. Deploy an equivalent authorized integration to a public CoFHE testnet and record transaction receipts and true gas measurements.
4. Add explicit compiler-to-Solidity differential tests and misuse-oriented property tests for supported types and operations.
5. Obtain external review before recommending generated code for value-bearing deployments.

## Version discipline

The Rust workspace currently reports `0.1.0`, and the CoFHE demo depends on the pinned packages represented by `integration/cofhe-hardhat/pnpm-lock.yaml`. The project label "V2" refers to the development track, not an independently released `2.0.0` package version.
