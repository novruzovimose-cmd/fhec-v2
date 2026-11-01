# FHEC V2 / CoFHE Phase 6: test-first productionization path

This document defines what the compiler/toolchain can prove, what the CoFHE SDK and network must prove, and what constitutes a credible funding milestone. Local Hardhat mocks, even when they return correct decrypted values, are **not actual coprocessor FHE execution**. An installer cannot certify production security or guarantee grant funding.

## System boundary

1. High-level FHEC source and compiler type checker enforce encrypted types and encrypted select semantics.
2. Typed intermediate representation and optimizer select legal rewrites, with a report recording expanded encrypted operations and function-call depth.
3. CoFHE code generator emits inspectable Solidity using FHE imported types. A separate wrapper validates external ciphertext proofs and grants intended results only to the contract and submitting account.
4. Hardhat mock backend exercises encryption proofs, signatures, ACL, decryption, and a known plaintext oracle. The independent test set includes u8, u16, u32, u64.
5. Public testnet execution is separate and optional. The user must supply an HTTPS RPC URL, dedicated test-wallet key, sufficient test ETH, and explicit transaction consent. A successful SDK trace is **provisional** until independently matched against chain receipts and completed CoFHE tasks.
6. Evidence bundling must preserve source identity, generated Solidity digests, test corpus version, environment labels, and limitations; no performance claim can mix mock and live measurements.

## Why FHEC can differentiate from handwritten Solidity

The output remains Solidity because Solidity provides deployment and Fhenix's protocol boundary. FHEC's value comes from language-level privacy semantics, optimized encrypted operations, automatic lowering, cost explanation and budget admission, and independent differential validation. FHEC does not make Solidity irrelevant. A fair performance claim requires matching function semantics, equivalent ciphertext parameter widths and access rules, the same chain/SDK versions, and enough samples to report median and p95, not a single favorable result.

## Phase 6 deliverables

- A pure deterministic oracle with exact modular arithmetic and boundary cases, for four unsigned widths and six operation families.
- 48 encrypted comparisons plus five negative cases in the default smoke run; 384 boundary comparisons plus five negative cases in the extended run (16 per family); or 672 comparisons (16 boundary plus 12 deterministic pseudorandom vectors per family) plus five negative cases in the complete run (28 per family). Every run uses a Hardhat mock, not a live FHE coprocessor.
- Two immutable JSON golden corpora are packaged: `tools/cofhe/phase6/golden-boundary-v1.json` (384 rows) and `tools/cofhe/phase6/golden-full-v1.json` (672 rows). Node tests regenerate both mathematically, compare every recorded field, and fail if an operation, seed, bit-width or case label drifts. The patch manifest binds both golden corpora to explicit hashes.
- A generated account-partitioned Solidity wrapper that checks proofs with `FHE.asEuint{width}`, passes the resulting encrypted values to the optimized Rust-generated library, and explicitly grants `FHE.allowThis` and `FHE.allowSender`. The cleartext operation selector is public and no ciphertext-dependent EVM branch is used.
- A separate local mock test file that validates decrypted results against the independent modular arithmetic oracle for every case, tests mismatched contracts, wrong signers, proof swapping, unsupported operation selectors, and unauthorized decryption.
- A local offline verifier that recomputes the oracle and rejects altered mock statistics, missing test families, and spurious claims of live execution.
- An optional opt-in Arbitrum Sepolia script that deploys the optimized generated wrapper, encrypts and submits **three** test vectors, waits for decryption, compares results against the plaintext oracle, and emits an explicitly provisional SDK trace.
- Guarded PowerShell installation, file-level cryptographic digests, and generated-source cleanup with preservation if an external process modified a generated file.

## Acceptance tests: exhaustive gate matrix

The following is the planned *complete* release gate list. Only boxes explicitly proven by the latest Phase 6 run can be checked. The mere presence of test source does not count as passing the test.

### A. Language and type correctness

- [ ] A01 Lexer handles all supported encrypted primitives and malformed input boundaries.
- [ ] A02 Parser rejects unbounded encrypted branching, missing returns, malformed expressions and unsupported syntax.
- [ ] A03 Type checker rejects encrypted/plaintext mixing and incompatible unsigned widths.
- [ ] A04 Literal conversion rejects negative or out-of-range unsigned values.
- [ ] A05 A boolean expression cannot be silently returned as an encrypted integer.
- [ ] A06 Loop and recursion policy is explicit; direct and indirect recursive helpers fail closed.
- [ ] A07 Identifier validation prohibits unsafe Solidity names and shadowed imports.
- [ ] A08 Diagnosed failure cases do not echo plaintext secret inputs.

### B. Optimization soundness

- [ ] B01 Addition by zero and subtraction by zero preserve modular arithmetic.
- [ ] B02 Multiplication by one preserves values; multiplication by zero is only simplified if safe for effect and noise semantics.
- [ ] B03 Common-subexpression elimination never merges distinct encrypted inputs.
- [ ] B04 Encrypted select preserves both arms and never converts a ciphertext predicate to a plaintext branch.
- [ ] B05 Helper invocations are counted and are not unsafely removed or reordered.
- [ ] B06 Nested helpers expand costs correctly; recursion is rejected.
- [ ] B07 Optimizer on/off produce equivalent outputs for every boundary test, not just syntactically similar Solidity.
- [ ] B08 Static cost budgets fail when a baseline exceeds configured operations, calls or depth.
- [ ] B09 Optimizer cost reduction claims distinguish static task count from real gas/fees.

### C. Solidity, SDK and privacy boundaries

- [ ] C01 Generated library compiles with the pinned Solidity compiler and CoFHE contracts.
- [ ] C02 Generated wrapper accepts valid input proofs for every supported width.
- [ ] C03 Proof bound to another consuming contract is rejected.
- [ ] C04 Proof bound to a different signer is rejected.
- [ ] C05 Swapped proof/hash pairing is rejected.
- [ ] C06 Unsupported public operation selectors revert before encrypted computation.
- [ ] C07 Encrypted result can be read by the designated submitter.
- [ ] C08 Unauthorized account cannot decrypt another submitter's result.
- [ ] C09 Identical inputs and unsigned extreme values yield correct encrypted outputs.
- [ ] C10 Boolean comparisons are independently checked at equality and max/min boundaries.
- [ ] C11 Public decryption and private view are separately authorized; the test harness does not expose a public reveal by default.
- [ ] C12 Client SDK is confirmed compatible with pinned CoFHE contract and Hardhat plugin versions.

### D. Test coverage and performance

- [ ] D01 Phase 4 three-way comparison (optimized, unoptimized, handwritten Solidity) passes.
- [ ] D02 Phase 4 signer/contract/proof/ACL negative suite passes.
- [ ] D03 Phase 6 48-case smoke matrix passes with Rust-generated optimized library.
- [ ] D04 Phase 6 389-case boundary-plus-negative suite passes with Rust-generated optimized library.
- [ ] D05 Mock latency and mock gas report identify the mocking environment.
- [ ] D06 Independent oracle recomputation catches altered expected values and incomplete test partitions.
- [ ] D07 Real testnet comparison uses identical inputs and equivalent Solidity wrappers for the generated and manual versions.
- [ ] D08 Median/p95 end-to-end latency and gas are captured from real network and reported separately from coprocessor fees.
- [ ] D09 Benchmarks include optimizer wins, regressions, and equal-cost examples; no cherry-picked aggregate speedup claim.
- [ ] D10 Repeat the benchmarks on at least two independently configured machines.

### E. Genuine protocol and chain execution

- [ ] E01 A dedicated test wallet is funded and explicitly approves network transactions.
- [ ] E02 The RPC endpoint independently confirms the expected chain ID.
- [ ] E03 Contract creation receipt confirms deployed bytecode and successful status.
- [ ] E04 SDK encryption produces sender- and contract-bound inputs.
- [ ] E05 Real transaction receipts confirm successful on-chain input verification.
- [ ] E06 FHE task actually completes; accepted transaction alone is insufficient evidence.
- [ ] E07 Private ACP/view decryption succeeds for the authorized account.
- [ ] E08 Unauthorized ACP/view decryption is rejected on the real network.
- [ ] E09 Public decryption, if advertised, is separately checked through the SDK's signed publish flow.
- [ ] E10 Actual decrypted outputs match the independent arithmetic oracle.
- [ ] E11 At least one external reviewer independently reproduces an E04-E10 example from tagged source.

### F. Release, supply chain, reproducibility

- [ ] F01 Workspace tests, clippy and formatting pass from a clean source checkout.
- [ ] F02 Pinned Rust toolchain, CoFHE dependencies, Solidity compiler and lockfile are recorded.
- [ ] F03 Windows hosted CI passes from source-only checkout.
- [ ] F04 Linux hosted CI passes from source-only checkout.
- [ ] F05 Dependency audit does not contain unresolved exploitable vulnerabilities; unmaintained packages are tracked.
- [ ] F06 Source and generated artifacts have SHA-256 digests, timestamps and version identifiers.
- [ ] F07 Installer refuses unknown baselines and never overwrites unreviewed source changes.
- [ ] F08 Failed validation preserves the previous working implementation and reports the error.
- [ ] F09 No wallet secrets, API credentials, local `.env`, private data, or build caches are in the published source ZIP.
- [ ] F10 Reproducible README, runnable demo, video, license and concrete known limitations are published.
- [ ] F11 A third-party security reviewer examines ciphertext proof, ACL, decryption and optimizer semantics.
- [ ] F12 Release tag, archived source checksum and hosted CI results correspond to the same commit.

## Release tiers

| Tier | Pass criteria | Claim permitted |
|---|---|---|
| Local demo | A+B+C except C11-12; D01-06 | Functional mock-backed compiler demonstration |
| Grant-ready prototype | Local demo + E01-07 + F01-10 + credible performance report | Developer-tool prototype with independently reproducible real testnet example |
| Production-oriented candidate | E01-11 + D07-10 + all F gates | Reviewed integration candidate with enumerated risks |
| Security-certified production | Independent audit against precise deployed release, remediation and protocol signoff | Only the particular release and threat model approved by reviewers |

No tier is automatically 100% in all possible dimensions. Percentage-complete estimates should explicitly name the tier, evidence and unresolved gates.

## Proposed grant narrative

Present FHEC as a compiler/optimizer that reduces the engineering burden of confidential Solidity applications, not as a competitor to the Solidity language. Open the technical demo with real encrypted inputs, show generated Solidity and `FHE.select` on an encrypted comparison, compare static task counts and measured network gas separately, demonstrate failed unauthorized decryption, and provide a one-command reproduction route. A $20,000 request can be structured as milestone-based funding, with explicit acceptance tests. No grant size or award probability can be promised without a verified current program offer.

## Security and deployment policy

The default Phase 6 installer runs only local verification. It cannot deploy. Live deployment is a separate command requiring a `-Live -AllowLiveTransactions` combination, an HTTPS RPC URL, and a locally managed testnet private key. The runner checks chain ID before deploying. It does not log the private key, save raw proofs, or attempt mainnet deployment. Its outputs are provisional even after successful SDK decryption, until independently correlated with receipt and task-completion evidence.

The account-bound Solidity maps isolate result handles by submitting address, rather than overwriting a shared state slot for everyone. The public getter may reveal a ciphertext handle but cannot itself grant decryption; authorization rests with CoFHE ACL. We intentionally avoid public decryption in this patch. A later security-specific patch must model ACP grants and revocations explicitly and test them on real networks.

## Limitations of the current patch

The construction environment can run Node tests and compile/test a CoFHE mock wrapper using an equivalent source-level library. The final Rust code generation must be executed on the user's Windows Rust environment; until it passes, this patch is a source candidate, not a Windows-verified release. The optional Arbitrum Sepolia script was not executed against a real network here. It may require adjustments to match any upstream SDK API or ACP changes beyond the pinned lockfile.
