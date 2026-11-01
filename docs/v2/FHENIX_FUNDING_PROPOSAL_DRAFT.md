# Draft: FHEC — a higher-level CoFHE compiler and optimization developer tool

**Status:** Discussion draft. Not a submitted application, funding commitment, formal valuation, or claim of Fhenix endorsement.

## Why this matters to Fhenix

CoFHE exposes encrypted values and operations through familiar Solidity APIs. FHEC explores an additional higher-level workflow: a Rust-implemented compiler that type-checks a dedicated encrypted language, lowers encrypted conditionals into selection, and emits Solidity code using Fhenix CoFHE primitives. Its long-term differentiator would be explainable, CoFHE-specific cost and correctness analysis rather than simple Solidity template generation.

## Evidence available today

FHEC has an existing TFHE-oriented Rust compiler pipeline and a V2 CoFHE backend. The current local demonstration compiles encrypted arithmetic, comparison, typed selection and function chaining into a generated Solidity library. A pinned local Hardhat environment compiled the generated contracts and passed 24 combined mock execution/security tests, in addition to focused Rust tests and Clippy checks. Real public-testnet CoFHE execution remains to be demonstrated.

## Proposed funding request: USD $20,000, milestone based

| Proposed milestone | Amount | Acceptance evidence |
| --- | ---: | --- |
| M1: Testnet execution and measurement | $5,000 | Supported public CoFHE testnet deployment, transaction links, ciphertext execution results, gas and latency observations |
| M2: Compiler correctness and feature depth | $5,000 | Extended typed operations, differential tests for generated Solidity, explicit unsupported-feature diagnostics |
| M3: CoFHE-aware analyzer and optimizer | $5,000 | Reproducible static operation-count/cost reports, optimizer comparisons and documented accuracy limits |
| M4: Developer-ready toolkit and adoption | $5,000 | Stable CLI workflow, complete setup/CI, example applications, at least one external developer reproduction and issue feedback |

Funding amounts, milestones, review criteria, jurisdictional eligibility, payment schedules and IP requirements are proposed terms only. Ask the Fhenix team which grant or fellowship process supports infrastructure tooling; Fellowship entry itself does not guarantee funding.

## Demo to share with reviewers

- Language source: `examples/cofhe_extended.fhe`
- Generated output: `target/cofhe-grant-demo-*/FHECGenerated.sol`
- Fhenix mock harness: `integration/cofhe-hardhat`
- Reproduction script: `tools/cofhe/verify-grant-demo.ps1`
- Source and verified test transcript: `target/cofhe-grant-demo-*/validation-*`
- Hosted CI: `.github/workflows/fhec-v2-verification.yml`, once verified against a committed revision

## Requested outcome

An initial technical review and a conversation with Fhenix engineering/ecosystem staff about compatibility, developer demand, the best testnet validation path, and eligibility for approximately $20,000 in engineering milestones.

## Before submission

- Confirm GitHub repository and uploaded code match the demonstration.
- Run hosted CI and attach the exact commit hash.
- Record a concise screencast of generation, Solidity compilation and test results.
- Confirm third-party notices and the distribution/license decision.
- Submit accurate screenshots, logs and limitations only; do not claim testnet or mainnet success.
