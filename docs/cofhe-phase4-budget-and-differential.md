# FHEC V2 Phase 4: resource budgets and differential verification

## Scope and status

This increment adds a strict static operation-budget gate and a broader Hardhat mock benchmark harness for FHEC CoFHE. It does not change the Rust compiler backend, its optimizer rules, TFHE runtime behavior, smart-contract access-control design, or live network deployment. The Phase 3 baseline is protected by checksums before installation.

The intended value is developer trust: encrypted operations are expensive and their execution cost is not obvious from ordinary high-level syntax. FHEC can already generate Solidity and report static operations; Phase 4 adds a machine-readable admission decision and differential evidence across common encrypted arithmetic families and widths. It must be described as a development and verification tool, not a formal FHE cost guarantee.

## System architecture

1. `examples/cofhe_phase4.fhe` contains 26 compiler functions: six operation families for each width (`u8`, `u16`, `u32`, `u64`) and two nested helper functions. The operation families are `add`, `sub`, `mul`, `gt`, `max`, and repeated multiplication. This is the source of generated Solidity.
2. The existing Rust CLI emits `FHECPhase4Baseline.sol` and `FHECPhase4Optimized.sol` from the same input, using the pinned `fhec-backend-cofhe`. Only the optimized variant enables the existing CoFHE optimizer. The verifier records hashes of each output. No source files are overwritten by the CLI.
3. The existing Rust analyzer emits a versioned cost report, including expanded call-graph operation estimates for each function. `tools/cofhe/check-cost-policy.mjs` reads this report and performs a strict resource-budget decision. Rejecting a budget terminates validation before deploying generated code.
4. `integration/cofhe-hardhat/phase4/FHECPhase4Manual.sol` is an explicit handwritten-like reference containing standard FHE operations and reused intermediate values. It is checked into the package and is **not** produced by the Rust compiler. Its deterministic source generator is separate from the FHEC backend and checked against committed bytes. The independent numerical oracle is implemented separately in the TypeScript test.
5. `FHECPhase4Comparison.sol` exposes three entry methods per width, with shared input-proof conversion, output permissions, and storage. The three variants share a contract address so input proofs can be contract-bound; the method selectors and storage slots differ, which is disclosed as benchmark overhead.
6. `FHECPhase4Differential.test.ts` uses pinned Fhenix Hardhat mocks. For each width and operation, it encrypts the same pair separately for all three variants, invokes the contract, decrypts a permitted result, and compares it with an independent modular arithmetic oracle. It also tests wrong signer, wrong consuming contract, mismatched proof, and stranger decryption denial.
7. `tools/cofhe/summarize-phase4.mjs` rejects missing, duplicate, out-of-range, or oracle-inconsistent evidence before computing per-family median and p95 gas. No cross-family average is presented as an FHE performance advantage.
8. `tools/cofhe/render-phase4-report.mjs` creates a self-contained, offline-readable HTML table with prominent mock-network disclaimers. It has no CDN, embedded executable JavaScript, or third-party telemetry. It does not contain the compiler implementation.
9. `tools/cofhe/verify-phase4.ps1` performs generation, policy tests, Rust regressions, Solidity compilation, mock execution, evidence aggregation, report rendering, and exact restoration of temporarily staged Solidity files. It does not change V1 or replace Phase 3 Rust sources.

## Design invariants

- Every result is attributed to the exact source SHA-256 and generated Solidity SHA-256.
- Hardhat MOCK transactions must not be reported as testnet or mainnet gas.
- The source-level cost policy applies to **one named entry function**. The call-graph expansion counts syntactically repeated helper calls and eager encrypted select branches. It is not an upper bound on network gas, asynchronous coprocessor latency, or total execution time.
- The policy accepts nonnegative *safe* integers. If a serialized metric exceeds JavaScript's safe integer range, the policy rejects the report instead of approving a rounded number.
- The TypeScript test independently derives results modulo `2^N` for arithmetic families. The comparison and select families use native boolean and maximum semantics.
- Encrypted select is represented by supported `FHE.select` operations. No plaintext branch based on encrypted data is introduced.
- The Solidity wrapper branches only on the public operation selector supplied by the caller.
- The input proof must be bound to the consuming contract and submitting signer. Negative tests exercise relevant mismatches.
- `FHE.allowThis` and `FHE.allowSender` are retained in the benchmark wrapper. No automatic unrestricted publication of ciphertext decryption is added.
- The verifier preserves preexisting `contracts/*.sol` files byte-for-byte. New temporary contract files are removed after the test run.
- The installer refuses to replace unexpected project files and refuses an unknown Phase 3 baseline. A failed installation removes only the newly added Phase 4 payload files when their hashes remain unchanged.

## Cost policy

An example of an expected acceptance decision:

```
node tools/cofhe/check-cost-policy.mjs --report target/phase4/optimized-static-report.json --entry repeated_u8 --mode optimized --max-ops 2 --max-calls 0 --max-depth 0 --out target/phase4/cost-policy.json
```

The call emits `schema: fhec-cofhe-cost-policy-v1`, the named function and selected baseline/optimized variant, each checked bound, observed static counts, the verdict, and limitations. A successful policy returns exit code 0; a policy violation returns 3; malformed inputs and file errors return 2. Evidence files are written with exclusive-create semantics and are never silently replaced. The Windows verifier uses `verify-phase4-contrast.mjs` to assess both expected variants with one successful command, avoiding platform-specific PowerShell handling of intentional nonzero policy rejections.

For the redundant fixture, `repeated_u8` has three baseline encrypted operations and two optimized operations. The test harness requires the optimized version to pass a maximum of two expanded encrypted operations and the baseline to be rejected by the same rule. This demonstrates the gate and one safe optimization; it does not imply broad runtime cost savings.

## Differential test matrix

| Operation | u8 | u16 | u32 | u64 | Result |
|---|---|---|---|---|---|
| add | tested | tested | tested | tested | modular unsigned integer |
| sub | tested | tested | tested | tested | modular unsigned integer |
| mul | tested | tested | tested | tested | modular unsigned integer |
| gt | tested | tested | tested | tested | encrypted boolean |
| max | tested | tested | tested | tested | encrypted unsigned integer |
| repeated | tested | tested | tested | tested | modular unsigned integer |

Each family can execute 1–8 deterministic input pairs sampled from the pinned 9,472-line oracle corpus. With the default of two pairs per family, 48 comparisons run and each comparison uses three encrypted contract executions. Four additional security-negative tests check proof and permission behavior. One pair per family provides a faster smoke test; eight per family provides more breadth. Neither is a statistically representative performance benchmark across real-world contracts.

## Evidence and interpretation

The script creates a uniquely named `target/cofhe-phase4-*` directory. Its intended contents include generated Solidity, static report, policy decision, raw mock test samples, per-family gas summary, offline HTML evidence report, a verifier transcript, and a final validation JSON.

The summary preserves each family's sample count, median mock gas, p95 mock gas, and signed percentage differences. A negative `optimized_vs_manual_median_reduction_percent` means that the optimized generated code used **more** gas than the manual implementation for that family. We do not hide such results.

The benchmark does not claim:

- that FHEC compiles arbitrary Solidity or replaces Solidity development;
- that the optimizer reduces execution costs for all programs;
- that compiling encrypted select guarantees constant-time execution at every system layer;
- that mocks prove real FHE decryption, network privacy, throughput, or gas;
- that static operation counts are accurate network cost estimates;
- that the input corpus represents audited production workloads.

## Local execution on Windows

After the Phase 3 R2 patch is installed, extract the Phase 4 patch ZIP to a new temporary directory, verify its externally provided SHA-256, and execute the installer against `Documents\fhec-v2`. The default case count is two per family.

```
& "<extracted>\Install-FHECPhase4.ps1" -Project "$env:USERPROFILE\Documents\fhec-v2" -SkipDependencyInstall -CasesPerFamily 2
```

When the installer finds a mismatch in any prerequisite file, it stops without changing the project. After validation, inspect the evidence directory. Only after a successful local mock run should the broader full-workspace regression, independent Solidity review, pinned SDK migration check, and live-network integration be attempted.

## Follow-up engineering milestones

1. Add a more representative case generator with explicit stress and edge-case distributions and a recorded random seed.
2. Compare source-to-Solidity development effort and output quality against a manually authored CoFHE library reviewed by an independent developer.
3. Separate proof-verification, storage, function-dispatch, ACL, and encrypted-operation costs where feasible.
4. Integrate budget configuration as a first-class Rust CLI and language policy once backward compatibility and report schema migration are reviewed.
5. Migrate the supported SDK adapter only after upstream version and input-proof validation rules are checked.
6. Deploy the same family matrix on a legitimate testnet; record chain ID, receipts, contract source, task completion, and permitted decryption outcomes.
7. Strengthen encrypted-data disclosure diagnostics and document access-control effects of optimizer rules.
8. Run independent code review and hosted CI before public release or grant submission.

## Important validation note

The TypeScript mock harness can be checked with equivalent isolated reference libraries, but only an actual FHEC Rust compilation proves that the generated baseline and optimized Solidity have the intended syntax and semantics. The Windows verification script regenerates these libraries from source before compiling and testing. If any step fails, the patch is not fully verified, and no Phase 4 success claim should be made.
