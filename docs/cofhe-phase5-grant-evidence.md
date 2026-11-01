# FHEC V2 — CoFHE Phase 5: independent verification and bounded grant claims

## Architecture and motivation

FHEC is a compiler pipeline, not an alternative on-chain virtual machine. Its input is a high-level encrypted program; its output is a Solidity library interoperating with CoFHE's existing Solidity interfaces. The runtime contract and client SDK are separate integration layers. At the end of Phase 4, user-verified local mock tests established three-way functional comparisons among a generated baseline, an optimized generated library, and a handwritten Solidity reference.

The strongest proposed funding positioning is a developer experience and safety tool: encrypted-language lowering with diagnostic guarantees, inspectable CoFHE Solidity output, deterministic cost budgets, differential tests, and evidence users can reproduce. This is not a claim to have developed CoFHE itself, a new FHE cryptosystem, or a production-audited EVM runtime.

Phase 5 adds two forms of *independent verification* and a conservative funding evidence bundle:

1. A local validator re-checks hashes of the `.fhe` source, canonical test vectors, manual reference, generated Solidity, the original static report, budget reports, raw mock results, and summary HTML. It also regenerates the summary from raw samples using an independent integer oracle and rejects an altered summary even when someone updates its hash in the validation JSON.
2. A read-only on-chain verifier optionally checks the expected EVM chain ID, deployed contract bytecode, matching transaction destinations and method selectors, successful receipts, positive gas use, matching transaction/receipt block identities, and a minimum confirmation count. It never signs transactions, holds keys, submits payloads, or uses methods that can mutate chain state.
3. A grant-evidence report separates **locally verified mock results** from **public-chain receipt integrity**, and explicitly refuses to upgrade either to a verified FHE proof/decryption claim.

### Boundary of evidence

| Evidence | Demonstrated by Phase 5 | Explicitly not demonstrated |
|---|---|---|
| Local compiler artifacts | Hash integrity, source and manual reference consistency | Compiler formally verified or audited |
| Hardhat CoFHE mock | Three-way differential test report revalidated from raw samples | Live FHE coprocessor correctness, production fee model |
| Read-only EVM receipt | Chain ID, deployed bytecode, confirmed successful transaction, selector | Authorized ciphertext proof, task execution, ACL, deciphered result |
| Grant bundle | Repeatable manifest and honest assertions | A $20,000 funding approval or production security certification |

## Real testnet completion gate

Use the official public CoFHE testnet network and current published SDK with a dedicated test wallet. The exact network/chain ID and RPC endpoint must be verified against official documentation. The next *distinct* integration milestone must perform and record:

- A client encrypts at least three distinct inputs, obtaining SDK-accepted ciphertext/input proofs with correctly scoped sender and contract addresses.
- A deployment using an independently controlled test wallet binds the proof and contract to the intended chain, and returns a transaction hash.
- An on-chain function invokes CoFHE operations; task identifiers are followed to an actual completed result, not just an accepted transaction.
- Access-control grants are explicit and tested both positively and negatively. A third party must fail unauthorized view access. If publishing a result, the public reveal path and the private view path must be labeled separately.
- A decrypted output is compared to an independent plaintext oracle for every supported width and operation family included in the claim.
- Record real execution times, network gas, FHE processing fees when measurable, and wallet/provider versions with the same granularity as the mock report. Do not extrapolate speedups from static operation counts or mock gas.
- Independent reviewers replay at least one end-to-end scenario from an unmodified tagged source revision.

No software patch alone can produce these external facts or guarantee the correctness of an unaudited cryptographic integration. Wallet secrets and RPC API tokens stay in local environment variables or secure wallet software; neither is uploaded with the package.

## How to use

Run `tools\cofhe\verify-phase5.ps1` from Developer PowerShell, passing `-Project` and optionally `-ReusePhase4Run` for an already verified Phase 4 report. Without `-ReusePhase4Run`, the script runs Phase 4 again in full and detects its new report directory. It always executes the Phase 5 unit tests. It stores reports under a new `target\cofhe-phase5-*` directory and does not modify project source files.

Optional on-chain receipt verification takes a JSON target descriptor and an *environment variable name* containing a user-configured HTTPS RPC endpoint. It is a **read-only receipt verification**, not a live-encrypted-computation demo.

Sample target descriptor (replace all placeholder data with actual network facts):

```json
{
  "schema": "fhec-phase5-onchain-target-v1",
  "chain_id": 421614,
  "min_confirmations": 5,
  "contract": "0x1111111111111111111111111111111111111111",
  "transactions": [
    {
      "label": "encrypted_input_tx",
      "hash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "to": "0x1111111111111111111111111111111111111111",
      "method_selector": "0x12345678"
    }
  ]
}
```

The example contains dummy values and must not be treated as real transaction evidence. The RPC URL itself is never saved in an evidence report, and the verifier does not print it to the console.

## Proposed $20,000 milestone request

This is a proposal to a funder, not a statement of available or promised grant funds.

| Milestone | Request | Acceptance |
|---|---:|---|
| M1. Real CoFHE public testnet lifecycle | $5,000 | Chain-bound proofs, live result completion, ACL and decryption with published receipts |
| M2. Semantics and safety analyzer | $5,000 | Deny-by-default plaintext/ciphertext dataflow checks, adversarial tests, actionable diagnostics |
| M3. Performance and compatibility | $5,000 | Benchmarks against handwritten Solidity, pinned SDK/contracts version matrix, reproducible latency and fee data |
| M4. Independent reproducibility and release | $5,000 | Hosted Windows/Linux CI, source-only package, third-party reproduction, security review and documented limitations |

## Reviewer checklist

- Did the author explicitly label local mock tests as mocks? Yes, both report and on-chain attestation enforce this.
- Is a public chain transaction actually verified on-chain? Only if the optional read-only verifier succeeds against an independently chosen RPC endpoint.
- Does inclusion mean the FHE computation has completed? No.
- Does the manifest establish remote CI success? No; separate hosted CI links are required.
- Is the patch a security audit? No.
- Is a specific dollar award guaranteed by these results? No.
