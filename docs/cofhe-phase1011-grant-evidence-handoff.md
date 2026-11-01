# Reproducibility handoff for a $20,000 CoFHE developer-tooling proposal

## What the applicant can show after Phase 10/11

The strongest technical narrative for FHEC is not the number of source files or the total number of passed tests in isolation. It is the architecture and independent evidence that a source program is lowered into encrypted CoFHE operations, that the compiler's optimizations preserve sampled meaning, that the generated contract can be compared to independent Solidity implementations, and that an external developer can inspect the source and reproduce the result without relying solely on a screenshot.

Phase 10/11 strengthens the claim that FHEC's currently implemented CoFHE subset is not just syntactically generating Solidity, but is producing behavior consistent with an independently written unsigned arithmetic oracle for selected input widths and operations in a controlled local mock environment. Phase 11 makes the claim auditable by preserving exact source digests, canonical samples, mock execution observations, an independent result reconstruction, an offline report and clear readiness limitations.

For a funding evaluator, organize this evidence as a sequence: a single representative `.fhe` source example → FHEC-generated optimized Solidity → handwritten reference Solidity → tests with independent BigInt expected values → machine-readable results → offline replay manifest → optional separately verified live Arbitrum Sepolia receipts if they actually exist. Keep each step independently accessible. When the evaluator has only two minutes, the source-to-Solidity code example plus one table of validated results should be immediately understandable.

## The trust statement that makes this credible

The project does not claim general FHE confidentiality or formal verification just because its local CoFHE mocks pass. A real testnet execution trace may demonstrate operational integration, but proving remote cryptographic execution would require much more, including protocol-specific attestation or independent cryptographic review. A source file's SHA-256 protects against accidental change and supports reproducibility; by itself, it is not a signature from a trusted independent auditor. Always use the right phrase for the evidence actually available.

The Phase 11 readiness JSON is deliberately unflattering where evidence is missing. It recognizes four local gates, requires external work for four technical gates (live testnet, coprocessor fees, hosted CI and independent security review), and keeps actual grant approval outside tool control. Those are *gate counts*, not percentages of total V2 functionality. The project may have many other planned milestones not represented in this narrow evidence scorecard.

## Recommended reviewer dossier

Package the complete `target/cofhe-phase1011-dossier-<run-id>` directory without rewriting it. It contains the source snapshot and 13 files covered by a SHA-256 manifest. A reviewer with the verifier source can re-run arithmetic and file-integrity checks without needing your private testnet wallet, RPC tokens, SDK signing credentials or administrative access. Preserve the original full `mock-observations.json`, not a selected subset of green lines.

Keep a separate explanatory README in any eventual public repository. Include the exact Rust toolchain, Cargo.lock, pnpm-lock.yaml, CoFHE SDK version, Hardhat compiler version, chain identifier used by any actual live work, Windows/Linux environment, code revision, invocation, test seed, number of cases per family, selected Solidity variants and known untested functionality. Reviewers are more likely to trust reproducible limited evidence than large unsupported performance or novelty claims.

The portable Phase 11 report intentionally labels every gas unit as local mock gas. Do not apply a network gas price, exchange rate or total protocol cost to these values. If real native-chain receipts are separately available from Phase 8/9, show those in a separate real-testnet table with transaction hashes and the independently verified `gasUsed * effectiveGasPrice` fee calculation, clearly noting any CoFHE coprocessor task charges that remain unmeasured.

## Grant milestone framing

This project is a Rust-to-CoFHE developer tooling effort, with optional independent analysis and optimized circuit generation. The practical value proposition is reducing developer friction and potentially reducing encrypted task complexity for common operations. The proof-of-value burden is to show reproducible compiler semantics, useful examples for real developers and transparent performance/cost measurements. A local test campaign and an integration demo do not alone establish market demand.

Illustrative remaining milestones for a proposed $20,000 engineering grant could be:

| Milestone | Proposed verifiable deliverable | Suggested allocation |
| --- | --- | ---: |
| Public build reproducibility | Clean clone, pinned dependencies, hosted Linux/Windows CI and release artifacts | $4,000 |
| Verified testnet demo | At least one approved live workload with independent public receipt attestation | $4,000 |
| True FHE cost measurement | Native transaction gas plus separately measured coprocessor tasks/charges and repeatability | $4,000 |
| Developer integration package | Production-quality CLI examples, tutorials, documented limitations and reference applications | $4,000 |
| Hardening and independent review | Threat model, source-level fuzzing beyond current subset, security review and remediation | $4,000 |

These amounts are illustrative only and total $20,000. Funding rules may prohibit retroactive funding or limit eligible expenses. If a milestone is already completed, do not present it as remaining work that needs grant funding. The actual opportunity must be checked against the current funding program's specific requirements and eligibility; no program or award is guaranteed by this software patch.

## Suggested evidence page outline

1. **Problem and target user.** A developer wants to express simple encrypted comparisons and arithmetic without manually writing every FHE Solidity operation or reasoning about every shared subexpression.
2. **One readable example.** Show the compact FHEC input and the generated `FHE.select`/`FHE.gt` or arithmetic code side by side, with a short explanation of which encrypted operator is emitted and why.
3. **Compiler architecture.** Show the frontend, semantic analysis, IR lowering, optimization/reporting, CoFHE emitter, Solidity compilation and mock or verified live execution. Explicitly label which parts are covered by tests.
4. **Correctness evidence.** Present the exact 24 supported operation/width families, independent arithmetic oracle and number of mock-encrypted executions, including failure-path tests.
5. **Performance evidence.** Separate static operation-count reduction from mocked gas and independently corroborated real native gas when available. Never mix unmeasured coprocessor cost into a total.
6. **Trust and limitations.** Show the source SHA-256 seal, reproducible corpus, offline replay manifest, known untested domains, missing hosted CI/security audit and any actual live receipt coverage.
7. **Plan and funding request.** State what the next grant milestone purchases and which artifact a reviewer will receive as proof of completion.

## Remaining opportunities to strengthen FHEC V2

**Broader language subset.** The CoFHE backend currently exercises an unsigned arithmetic/comparison subset in these samples. Future lowering could support more constructs and types after careful semantic analysis and privacy-aware control flow design. Adding syntax without cryptographic runtime semantics is not a release-quality feature.

**Property and differential depth.** The new oracle reaches all current width/family combinations, but not all values. A further property suite could vary side conditions, compose multiple operations, examine helper calls and nested FHE `select` branches, and compare FHEC against independent handwritten circuits for many generated programs. Preserve source identity and seed for every corpus.

**Live and economic transparency.** A Phase 8/9 real-network run should be separately authorized by the operator and independently audited. Publish public transaction hashes and exact method inputs only to the degree they are safe and useful. Improve FHE coprocessor accounting before making cost or savings claims. Repeated samples under controlled network conditions are needed to support meaningful performance differences.

**Independent security confidence.** Review proof/signature validation, encrypted input contract-domain separation, key management, result access control, hardcoded chain IDs, generated Solidity import safety, dependency policy and expected failure behavior. Local mock correctness does not eliminate remote protocol or economic security risks.

**Public release discipline.** Revisit the older V1 release candidate gates: hosted CI was not yet independently verified at the old checkpoint, the repository previously lacked a configured remote and had substantial local working-tree changes, and a dependency audit had flagged unmaintained packages. Treat these as evidence-dependent release tasks; do not assume that passing new local CoFHE tests silently resolved them.

## Honest executive status statement

After successful Phase 10/11 local installation, a reasonable statement is:

> FHEC has a working CoFHE code-generation and local differential-testing pipeline. Its current unsigned operation subset is checked against an independent exact-width BigInt arithmetic oracle across all four supported widths and six operation families, using source-sealed generated Solidity and reproducible local CoFHE mocks. Complete test samples and artifacts are packaged for read-only verification. Real testnet operations, protocol task fees, hosted CI and independent security review are tracked as separate evidence gates.

The proposed grant supports the *next verifiable engineering milestone*, not a promise that V2 is universally production-ready or that the funding reviewer will accept the application. Credibility is part of the engineering deliverable.
