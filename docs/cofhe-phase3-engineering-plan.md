# FHEC V2: Phase 3 engineering and grant-readiness plan

Status: local engineering package. Source is private. This is not a mainnet or testnet security certification.

## Executive decision

FHEC should cooperate with Solidity instead of positioning itself as a replacement for Solidity. The reliable story is a higher-level language for encrypted expressions that emits inspectable, deployable CoFHE Solidity, with explicit optimization controls, static cost evidence, and differential testing. Solidity stays the integration layer for contracts, permissions, external input proofs, and deployment. The FHEC compiler handles source analysis, encrypted branch lowering, helper expansion, and mechanical optimization.

The immediate differentiator is *reproducibility*: input source, generated Solidity, optimization report, corpus hash, test transcript, local mock results, and eventually independent live-network receipts. An applicant should be able to audit a small example without access to private compiler internals. A technical reviewer should be able to request a larger source review under an appropriate arrangement.

## What is implemented in this package

1. An interprocedural CoFHE call-graph analysis. It includes direct counts, expanded counts for one invocation of each function, repeated helper calls, and maximum helper-call depth. It treats both arms of an encrypted select as eager computations. Counts are static, not network gas.
2. Deterministic diagnostics for direct and indirect recursive helper calls. Recursive expansion is rejected before Solidity emission instead of silently producing a misleading finite estimate.
3. Nine focused Rust tests for helper expansion, repeated helper calls, recursion, encrypted conditional calls, source validation, JSON reporting, and optimization changes across calls.
4. An auditable encrypted differential suite comparing FHEC's unoptimized output, FHEC's optimized output, and independently written CoFHE Solidity. The test suite compares decrypted plaintext to a modular unsigned-integer oracle. It does not assert optimized code is faster or cheaper than a competent human implementation.
5. A deterministic JSON Lines corpus covering 8-, 16-, 32-, and 64-bit encrypted arithmetic/comparisons, with additional 8-bit repeated multiplication scenarios. The corpus contains 9,472 vectors with explicit operand widths and expected mathematical results. Its SHA-256 is pinned by the verifier. The complete corpus is a future testing asset; the three-way Hardhat run currently executes the repeated-u8 partition only.
6. A PowerShell verification script that checks the corpus, Rust tests, Rust lint, static report structure, generated Solidity compilation, and 24 selected on-chain mock comparison cases by default, with a bounded 1–128 case setting. It restores existing generated Solidity files and records evidence separately under target.
7. A hash-guarded installer that refuses unrecognized baselines, validates all packaged payload hashes, backs up replaced files, and restores sources after a verification failure.

## Accurate interpretation of measurements

The previous benchmark contained six syntactic encrypted FHE operations in unoptimized FHEC-generated source and two in optimized output. This is a 66.7% reduction in *static operations for that specially redundant fixture*. It is not a representative average across all contracts. The prior five mock transaction comparisons averaged approximately 18.46% lower gas for the optimized version. The current three-way suite adds a handwritten Solidity reference, which is important because a competent Solidity author can manually reuse intermediate encrypted handles. If FHEC and manual Solidity have similar gas costs, the contribution is automatic compilation and safer tooling, not magical execution efficiency.

The corpus oracle uses unsigned modular arithmetic. For an N-bit unsigned integer, addition, subtraction, multiplication, and the repeated demo return results modulo 2^N. Comparisons produce booleans. An encrypted boolean may not be branched upon as a plaintext EVM condition: both alternatives of a secure select must be represented in the encrypted computation. The pipeline can introduce task scheduling, ACL side effects, and ciphertext-handle differences, so plaintext-output equivalence is necessary but not sufficient for a complete cryptographic security claim.

## Core invariant and threat model

- Never mistake Fhenix Hardhat mock execution for actual coprocessor FHE execution. The mock layer can keep test plaintext on-chain and charge different gas than production networks.
- Never generate Solidity that branches on `ebool` with ordinary `if` or `require`. Use supported encrypted selection operations.
- Never claim arbitrary ciphertext equivalence: operations may produce different handles and ACL metadata while decrypting to identical values.
- Never discard function calls or encrypted operations whose observable permission or task effects have not been considered. Algebraic optimizations are restricted to justified expression patterns and validated in full lowering.
- Reject recursive helpers until the language has an explicit bounded recursion or loop-lowering contract.
- Treat any static cost model as a defined approximation with explicit assumptions, not a forecast of exact gas or latency.
- Bind externally encrypted inputs to sender, chain, and consuming contract with SDK-supported proofs. Test mismatched sender, wrong consuming contract, and corrupted inputs.
- Use explicit allow rules for encrypted storage and decryption; treat ACL settings as security-critical behavior, not compiler optimization hints.
- Store no real wallet private keys in the source, evidence ZIP, logs, demo video, or GitHub repository.

## Architecture

FHEC frontend: parse syntax, type-check encrypted expressions, build symbol and function signatures. CoFHE lowering: validate permitted unsigned encrypted types, emit FHE operations, handle helper function scope and reference types. Optimization: safe local algebraic rules and limited common subexpression binding reuse. Audit pass: traverse function AST, build helper graph, detect cycles, compute expanded operation counts. Emission: one Solidity library with deterministic imports. Proof and execution harness: external SDK encryption, contract-bound input proofs, on-chain mock dispatch, decrypt and compare to an independent numerical oracle. Evidence: report schema plus SHA-256 of sources, generated output, corpus, and recorded tests.

The call-graph cost model is deliberately separate from Solidity text rendering. A function definition can contain one FHE operation and invoke a helper twice; a direct operation count is one, while the expanded count can include three or more task expressions. The expansion model uses memoized DFS and checked arithmetic; it rejects cycles because the expansion is otherwise unbounded or ambiguous. For each helper, it records its direct operations, expanded operations, direct helper-call sites, expanded helper-call instances, and deepest helper nesting.

## Phase 3 acceptance tests

### Backend and semantics

- Check identity addition on both sides, subtraction of zero, and multiplication by one on both sides.
- Keep multiplication by zero unless the observable FHE operation semantics permit removal and its security effects are reviewed.
- Preserve computed encrypted predicates rather than deleting them merely because select branches happen to match.
- Ensure lowerer validates both input and optimized ASTs.
- Validate unsupported widths and signed types with human-readable errors.
- Validate every local binding before optimization.
- Validate that an encrypted comparison cannot be returned in an encrypted integer context.
- Validate direct and indirect helper recursion errors.
- Validate helper expansion for repeated calls, multiple nested levels, and function definitions never called by others.
- Validate eager evaluation accounting for encrypted select branches.
- Validate deterministic JSON fields and stable function ordering.
- Validate counts with at least one source where optimized direct and expanded counts differ.
- Re-run the existing Rust backend tests, CLI tests, Rustfmt, and Clippy with denied warnings.

### Solidity and crypto mock behavior

- Compile baseline and optimized generated libraries with the pinned Fhenix mock Solidity environment.
- Compile a handwritten comparison reference using the same FHE operations and permissions.
- Check 24 deterministic encrypted test inputs by default; optionally 128 with the script argument.
- For each case, obtain distinct SDK-encrypted input proofs bound to the consuming contract for all three calls.
- Check that each case produces the same plaintext as the independent modular arithmetic oracle.
- Log three transaction mock-gas figures; never label them live testnet gas.
- Keep existing 5 comparison, 19 extended, and 5 optimizer tests available; run them during the full regression command before a grant submission.
- Preserve preexisting Solidity files in `integration/cofhe-hardhat/contracts`, restoring their original SHA-256 after any verifier outcome.

### Packaging and data hygiene

- Install only into an explicitly resolved V2 directory.
- Verify the exact known R2 source hashes before replacing the three backend files.
- Verify every payload file against the package's internal checksum manifest.
- Refuse to overwrite any additive path already present.
- Back up all replaced source files before the first write.
- Remove only newly introduced additive files during rollback, never arbitrary workspace files or build output.
- Keep target output, temporary test logs, and evidence after success for inspection.
- Reject a modified post-install state on repeat install; do not silently overwrite user edits.
- Keep V1 `Documents\fhec` unchanged.
- Do not upload the private source package to a public repository by accident.

## Future Phase 4: meaningful independent benchmark suite

The current corpus is an infrastructure asset and a limited u8 demonstration. The next benchmark must include multiple actual encrypted programs, not just one redundant multiplication example. Create cases where the optimizer should help, cases where it should make no difference, and cases where a manual Solidity author already shares intermediate results. Include u8, u16, u32, u64, comparison, arithmetic, nested encrypted selection, and helper calls. Use identical hardhat compiler configuration, SDK setup, input shapes, signer, permissions, and deployment model for all variants. Always report deployment gas separately from method transaction gas. Distinguish input proof verification, encrypted operations, storage, and ACL costs.

Build a machine-readable benchmark result with each source SHA, compiler commit/tag, emitted Solidity SHA, network chain ID, transaction hash, method label, input family, backend version, median / p95 gas or completion time, and sample count. Store raw observations, not only percentage improvements. If some operation requires asynchronous completion, measure submission and final result latency separately.

Use a gold-standard handwritten Solidity implementation written independently from the compiler output. It should reuse handles where safe and apply standard CoFHE permission practices. Run tests across edge cases and random seeded pairs. Add an independent code review before calling any advantage a genuine improvement. The goal is not to prove that generated code beats ideal human code in all cases; the goal is to show consistent quality and predictable diagnostics while saving developer effort.

## Future Phase 5: privacy-specific language guarantees

Before public release, design and test stronger semantics. Model encrypted values, plaintext values, and explicit disclosure boundaries separately. Reject accidental plaintext-dependent control flow when encrypted data is involved. Make output permissions explicit at the integration boundary; generated pure libraries should not silently grant broad ciphertext access. Distinguish content confidentiality from caller anonymity and public transaction metadata. Add diagnostics for unbounded, unsupported, or misestimated cryptographic operations. Document when implementation-level ciphertext handles may be observably different after optimization.

For each added optimizer rule, record a unique rule ID, preconditions, excluded cases, concrete before/after examples, tests for mathematical equivalence, tests for encrypted mock execution, and proof obligations regarding ACL/task side effects. Evaluate rule applicability before code generation. Never let a cost estimate authorize a semantic rewrite by itself.

## Future Phase 6: SDK and framework compatibility

The current source uses the pinned `@cofhe/sdk` and Hardhat plugin versions in the local project. The official Fhenix SDK/Hardhat package names and network support can evolve. Treat migration to `@cofhe/hardhat-plugin` and supported SDK changes as a separate compatibility project, with exact pinned versions, a reproducible fresh installation, and a compatibility matrix. Avoid upgrading dependencies inside a correctness patch, especially one that already has verified encrypted-input signer behavior.

Build an adapter only once the target upstream API is confirmed. Cover recipient/sender binding, consuming contract binding, encrypted input proof shape, decrypt-for-view vs published decryption, and ACL propagation. Keep older test behavior stable until the new adapter passes the exact negative-proof tests that previously caught `InvalidSigner` issues.

## Future Phase 7: real testnet evidence

Create a dedicated testnet deployer wallet. Obtain testnet funds only through a legitimate faucet or funded test wallet. Never share a seed phrase, private key, or signing export with a collaborator. The verifier should read a configured wallet in the local environment, and the script must never print secrets to a transcript. Confirm the supported chain, RPC, contract addresses, and current SDK protocol versions from official Fhenix sources on the day of deployment. Do not hard-code assumed addresses from old tutorials.

Deploy baseline, optimized, and manual comparison wrappers. Record chain ID and contract addresses, perform one encrypted input submission per variant using the same logical inputs, await the legitimate decrypted result or accepted proof flow, and log public transaction hashes. Preserve the difference between network transaction gas and off-chain FHE coprocessor execution fees. Compare like-for-like transactions with sufficient repetitions. Confirm decryption correctness and ACL restrictions, and flag all examples that only work under mocks.

Do not announce a live result until a third party can independently verify at least the contract code, transaction receipt, network identity, and an allowed plaintext result. A demonstration run on a single testnet is evidence of feasibility, not a comprehensive security audit or throughput benchmark.

## Future Phase 8: release and funding evidence

Produce a concise public source-limited GitHub repository with a diagram, high-level language snippet, generated Solidity fragment, optimizer explanation, numerical case study, test transcript, and anonymized evidence JSON. Keep private source outside that repository. Make it clear which pieces are reproducible independently and which must be shown in a live walkthrough or restricted source review. A public-only demo without any verifiable test methodology is weak grant evidence.

Record separately: achieved functionality, test environment, known limitations, future milestones, developer benefits, compatibility risks, and requested funding breakdown. Avoid calling FHEC "the first" or "10x better" without a scoped benchmark and comparative literature review. For funding, propose measurable milestones such as protocol-compatible lowering, advanced diagnostics, benchmarks, CI matrix, and testnet integration rather than asking for money on the basis of a single mock test.

## Presentation and reviewer flow

1. Explain the friction in encrypted smart-contract development in one sentence.
2. Show a small FHEC source example (not private compiler implementation).
3. Run FHEC to generate a Solidity library and static cost report.
4. Show what the optimizer removed and why it is allowed for the selected example.
5. Show all three CoFHE variants compiling and decrypting to the same correct value.
6. Show the manual Solidity result side by side to demonstrate an honest baseline.
7. Explain mocked versus live execution before displaying any gas comparison.
8. Link the transcript and hashes to the demo, keeping credentials and unpublished source private.
9. Present the remaining roadmap as milestones with acceptance criteria.

## Hard stop criteria

Stop publication if any of the following is observed: unequal decrypted results, ACL bypass, proof accepted for the wrong sender or contract, unexpected source hash mismatch, failed restore, unreviewed dependency update, missing upstream network address confirmation, conflicting benchmark environments, or claims that cannot be supported by evidence. Correct the issue before recording the grant video.

## How to use this package

Extract the archive into a new temporary folder. Verify its provided SHA-256. Run `Install-FHECPhase3.ps1` with `-Project "$env:USERPROFILE\Documents\fhec-v2" -SkipDependencyInstall`. The script checks source hashes and payloads before modifying the V2 workspace, creates a backup, runs tests, and rolls back on failure. The default three-way test uses 24 encrypted examples; use `-CaseCount 128` after default validation if you want a longer run. Keep the validation report and transcript produced under `target\cofhe-phase3-*`.

The generated corpus is test data, not live-chain evidence. Its 9,472 rows deliberately include widths or operations not yet wired to the three-way integration contract; those rows are intended for the subsequent compiler/runtime adapter and additional test contracts. This distinction belongs in any outside-facing description of the project.

