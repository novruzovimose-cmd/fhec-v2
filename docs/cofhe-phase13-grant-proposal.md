# FHEC V2 — Proposed $20,000 CoFHE Developer Tooling Milestones

Status: **draft funding request**, not an approved award or a statement of eligibility.

## Proposal in one paragraph

FHEC is a Rust compiler and static analyzer for encrypted programs that can emit CoFHE-compatible Solidity and provide operation counts, helper-call depth reports, and provable source-to-report integrity checks. Existing local qualification includes independent modular arithmetic oracles, width-aware encrypted Hardhat mock differentials, integrity seals, adversarial tests and a portable release capsule. We request milestone-based support to establish independently reproducible real CoFHE execution, expand practical developer coverage, disclose actual protocol costs, and turn the local prototype into developer-facing tooling that makes confidential contract authoring and performance testing easier.

## What we can already honestly claim

- Source-to-Solidity compilation demonstrated for a selected CoFHE operation subset.
- CoFHE generated and handwritten differential tests across unsigned widths 8, 16, 32 and 64 in the Phase 12 local mock dossier.
- Recorded 24 selected operation-width families and 216 local encrypted Hardhat mock executions, as reported in the user's successfully qualified Phase 12 candidate.
- Independent plaintext reference validation and adversarial evidence-corruption checks in the existing local phase track.
- Source inventory and portable local evidence capsule, independently verifiable by local scripts.
- A guarded preexisting Arbitrum Sepolia **opt-in** Phase 8 runner and separate Phase 9 read-only receipt verifier.

Do not claim these prove mainnet performance, live CoFHE task billing, a third-party security audit, full FHE.sol API support, or grant acceptance.

## Problem statement

Writing confidential Solidity by hand requires understanding encrypted-type semantics, evaluator op counts, client proof binding, permissioning, dynamic FHE task behavior and the distinction between static cost estimates and the native gas bill. Existing general-purpose application examples do not fully answer compiler-level optimization and auditability questions. FHEC aims to provide a source-level development path with explicit encrypted operations, generated contract output, and reports that can be reproduced and externally inspected.

## Intended public developer deliverables

1. Version-pinned Rust CLI and source examples for CoFHE-generated Solidity.
2. Compatibility table separating implemented operations from unsupported or unverified CoFHE features.
3. End-to-end wallet-independent local mock workflow, with an optional separately approved public testnet path.
4. Report showing source hash, operation graph cost model, source seal, live receipts, native gas, SDK timing and unknown protocol fees as separate fields.
5. Minimal integration tutorial, troubleshooting guide and explicit failure modes.
6. Hosted Windows/Linux regression and evidence reproducibility.
7. Independent security review and remediation record.

## Five deliverables and proposed disbursements

| Milestone | Proposed amount | Acceptance gate |
| --- | ---: | --- |
| M1: Public reproducibility and API matrix | $4,000 | Reviewed public revision, hosted Windows/Linux CI, source anchors and supported API matrix |
| M2: Verified real CoFHE testnet execution | $5,000 | Operator-authorized Arbitrum Sepolia generated-contract executions, matching SDK outputs, two read-only RPC corroborations |
| M3: Measured network and task economics | $4,000 | Native gas and FHE service costs separately quantified with transparent methodology, known unknowns and regressions |
| M4: SDK and developer integrations | $4,000 | Public tutorial, starter example, permission and proof lifecycle walkthrough, independently reproduced user setup |
| M5: Security review and release hardening | $3,000 | External findings, severity triage, fixes and reviewer follow-up |
| **Total requested** | **$20,000** | **Subject to independent program approval and milestone negotiation** |

If any milestone has already been completed when submitting the application, renegotiate it as future work. Do not request retroactive payment for unspecified past labor unless the funder's terms explicitly permit it.

## Reviewer-friendly KPIs

| KPI | Measurement | Evidence requirement |
| --- | --- | --- |
| Supported operations | Count of individually tested language/backend pairs | Pinned API matrix and passing named test |
| Generated correctness | Live exact oracle matches per operation and width | SDK outputs plus transaction hashes |
| Reproducibility | Independent clean checkout on two OSes | Hosted public CI links and revision SHA |
| Optimization changes | Difference in encrypted task graph and native gas | Static model and independent live receipt comparison separately |
| Documentation usability | Independent engineer reproduces basic demo | Run log and feedback |
| Security | Resolved high/critical findings | Independent assessor report and remediation revision |
| Adoption | Real outside developers integrating the CLI | Reproducible projects, issues and public feedback |

## Funding outreach sequence

1. Submit a precise technical summary to the active Fhenix Fellowship application. Ask whether $20,000 milestone-based support for independent developer tooling is eligible. A fellowship is not the same as a guaranteed cash award.
2. Include a repository URL, a short architecture diagram, a 90-second demo and a Phase 12 evidence link labeled **local**.
3. Show the Phase 13 roadmap and state exactly which external gates remain open at application time.
4. Ask for the specific grant/bounty track, expected milestone terms, supported jurisdictions, identity/KYC requirements and IP/open-source licensing obligations.
5. If invited, submit a one-page technical budget and an evidence table with direct source hashes, hosted CI links and testnet explorer links.
6. Keep copies of the application, review feedback and milestone definitions. Do not interpret a Discord compliment or fellowship admission as a cash grant.

Fhenix Fellowship: https://www.fhenix.io/ecosystem/fellowship

Official SDK development repository: https://github.com/FhenixProtocol/cofhesdk

Fhenix protocol and Solidity interfaces: https://github.com/FhenixProtocol/cofhe-contracts

## Suggested 90-second demo storyboard

- 0–15 s: Show a 10-line FHEC encrypted max or arithmetic source and explain which values stay encrypted.
- 15–35 s: Build generated CoFHE Solidity and show its `FHE.sol` calls.
- 35–50 s: Run the local encrypted mock differential and show matching independent oracle values.
- 50–65 s: Show static cost and optimization contrast with clear **not actual gas** labeling.
- 65–78 s: Display the Phase 12 signed-off local candidate and Phase 13 gate status; show a real testnet transaction explorer only after it truly exists.
- 78–90 s: Summarize developer value, exact proposed milestones, documentation link and open limitations.

## Language to avoid in the application

- "Fully supports all CoFHE" without a versioned API support matrix.
- "Production audited" without an independently verifiable external audit.
- "20% lower gas" based on static task counts, mock gas or an insufficient sample.
- "All fees capped" when only EVM native gas is bounded.
- "Our grant is approved" without a real award notice.

## Strong and credible closing line

We have completed a rigorous local prototype and constructed a fail-closed evidence pipeline. We seek funding to complete independent real-network verification, improve supported CoFHE APIs and developer onboarding, and deliver a reproducible open-source compiler tool that other teams can inspect and use.
