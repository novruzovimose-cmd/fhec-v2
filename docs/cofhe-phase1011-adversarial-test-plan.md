# Phase 10/11 detailed and adversarial acceptance matrix

## Test discipline

The trust boundary for this patch is a set of previously preflighted Phase 8/9 generated Solidity sources, a new independently implemented arithmetic oracle, the local CoFHE Hardhat mock execution layer and a separate offline result verifier. Each has different failure modes. Run tests in increasing order of cost and preserve diagnostic output whenever an earlier stage fails. Do not skip the source-seal check to make a failing test pass.

The installer performs an exact source SHA-256 baseline check before copying files and verifies the payload manifest's path allowlist, file size and SHA-256. An already-existing destination path with different content is a hard stop. Newly copied identical files can be reused on repeat installation. On a failed validation, only files newly copied in this installer attempt and still byte-identical to its own payload are removed. Previously installed files, user edits, target evidence and source snapshots are preserved.

The verifier performs five independent gates before it can mark a test as locally passed: accepted predecessor provenance; 100% passing Node offline tests and golden corpus comparisons; strict TypeScript/Hardhat compilation; successful local mock execution plus wrong consuming-contract rejection; and independent output/evidence replay. A missing stage cannot be replaced by a screenshot or a forged summary count.

## Independent arithmetic matrix

| Field | Valid source domain | Rejection conditions |
| --- | --- | --- |
| `width` | Exactly 8, 16, 32 or 64 | Unsupported width, signed width or unbounded integer |
| `operation` | Exactly add, sub, mul, gt, max or repeated | Unknown selector, unsupported symbol or mismatched operation name |
| `a`, `b` | Decimal strings from 0 to 2^width-1 | Negative, floating, out-of-range or imprecisely represented inputs |
| `expected` | Decimal string, except boolean for gt | Wrong modular wrap, signed comparison or boolean/integer confusion |
| `seed` | Lowercase 64-character hexadecimal | Uppercase, truncated, malformed or missing value |
| `cases_per_family` | Safe integer 2–16 | NaN, fraction, 0, 1, 17 or larger |
| `variant_mode` | optimized-tuned or three-way | Unknown implementation, omitted comparator, conflicting order |
| `corpus` | Exact canonical object with ordered cases | Edited sample, missing field, new field or reordering |
| `file` | New unique path | Overwrite of earlier fixture, corpus or evidence |

Unsigned modulo semantics are exact only when the width bound is explicit. The independent BigInt oracle must compute `a+b`, `a-b`, `a*b` and `2*a*b` modulo `2^width`, not modulo the JavaScript safe integer range. The greater-than result must remain boolean, and max must remain unsigned. For 64-bit values near `2^64-1`, use BigInt throughout every arithmetic operation. Rounding the input through `Number` is an error even when a small test coincidentally passes.

The first eight vectors per family test zero/zero, max/one, one/max, max/max, half+1/half, zero/max, one/one and half/half+1. Those vectors exercise zero identities, overflow and underflow, equality and ordering without relying on randomness. The pseudorandom samples start at index eight, derived deterministically from SHA-256 domain-separated left/right tags, the complete test seed, width, operation and index. Repeating the same command must produce the same ordered corpus and hash on another machine using the same code revision.

## Golden fixture reproducibility

Six fixtures are included because a handwritten reference function can regress in a future patch. Each fixture is a complete JSON snapshot with a known SHA-256 hash of its canonical semantic object. `verify-fixtures.mjs` validates the entire snapshot against a newly regenerated corpus. The six configurations are 2, 3 and 8 cases per family, each paired with optimized-tuned and three-way variant modes.

Suggested fixture mutations to test in a temporary copy:

1. Change `add`'s `max + 1` expected from zero to `max`.
2. Change a `sub` underflow result to a negative decimal string.
3. Change a `mul` overflow result to its unrestricted product.
4. Change `gt`'s expected boolean to the string `"true"`.
5. Swap the order of two operation families.
6. Change a source case's operation selector without changing its label.
7. Change a `repeated` formula to twice the sum rather than twice the product.
8. Delete a sample, duplicate a sample ID or add an unsupported width.
9. Change the deterministic seed while retaining old inputs.
10. Introduce a non-JSON number larger than JavaScript safe integer precision.

All these mutations must be rejected by canonical corpus validation. Mutations should never be made to the user's primary source or the originals stored in the patch ZIP.

## Contract-level mock checks

For every selected independent case, the harness encrypts `a` and `b` separately with `client.encryptInputs` and binds each encryption to the contract that will consume it. It submits the operation selector and two encrypted inputs to the corresponding `runU8`, `runU16`, `runU32` or `runU64` entry point on each selected implementation. It checks a successful local mock receipt, reads the result handle with the correct unsigned or boolean type, decrypts through the local SDK and compares the returned result to the independently computed expected value.

The standard two-variant test compares the optimized wrapper with the tuned reference. The three-way test additionally compares the straight reference implementation. If the optimized and handwritten paths agree with each other but disagree with the independent oracle, the suite must still fail. The independent oracle is the correctness standard, not majority voting among compiler implementations.

After the main corpus, the negative contract-domain test creates an encrypted input for a different consuming contract than the one processing the input, then expects the mock transaction to revert. The test runner must exit nonzero if that rejection does not occur. Only when the entire Hardhat command succeeds, including this negative test, may the PowerShell wrapper pass the `--negative-confirmed true` flag to the independent validator.

The harness writes `mock-observations.json` after the suite, including incomplete observations if a test failed before the full corpus. The independent validator explicitly refuses any incomplete result. It is intentionally not possible to manufacture a pass by omitting the failed case from the observations.

## Mock evidence integrity attacks

The independently replayable mock evidence has fields identifying its provenance, canonical corpus hash, original corpus file hash and prepared source-seal file hash. Every case contains an exact case ID, width, operation, expected value and per-variant observation. For each variant it preserves the SDK-observed output, an equality flag, measured mock gas in an unsigned decimal string and finite mock timing fields.

The validator must detect:

- An evidence file claiming `real-testnet` instead of the allowed mock tier.
- A changed corpus digest, source-seal digest or raw corpus file length.
- Any missing/duplicate/reordered case or renamed operation.
- A changed expected result in either the canonical corpus or the observation.
- A changed SDK-observed value, including one that still appears internally consistent with another edited field.
- Any missing, extra or reordered variant name.
- Zero, negative, floating, non-decimal or missing mock gas.
- NaN, negative or extraordinarily long mock timing values.
- An incomplete result record or a `full_corpus_completed: false` flag.
- Less than all 24 supported operation/width families represented in the corpus.

Mock gas is summed using BigInt and displayed separately by operation family and implementation. Its median is a direct integer median, not a percentile estimated from a larger imaginary sample. These are mock values affected by extra Hardhat mocking logic; never compare them directly to real Arbitrum Sepolia native fees as if the environments were equivalent.

## Source seal and dossier attacks

Phase 11 includes exactly six Solidity source files, the original Phase 8/9 live plan and its original source-seal JSON inside its own portable `sources` directory. The independent dossier verifier recomputes the predecessor source seal and validates that all seven source/plan inputs match exactly. This links the new arithmetic results to the same prepared compiler output used by the local Hardhat harness, even though the final dossier lives in a separate directory.

The dossier verifier should be tested against:

1. Modified Solidity comment or whitespace that changes the raw bytes.
2. Modified deployment wrapper Solidity but unchanged named contract.
3. Changed Phase 8/9 canonical plan.
4. Missing or changed source-seal file.
5. A symlink masquerading as a regular evidence file.
6. A deleted or renamed corpus.
7. A changed corpus seed with unchanged mock observations.
8. One fake expected value in the saved observation JSON.
9. A mock gas value edited without a matching source observation.
10. A manually edited summary that no longer matches the raw trace.
11. HTML content changed to present a fabricated speedup.
12. HTML injection through a family label or limitation field.
13. An edited readiness file promoting mock results to confirmed live receipts.
14. An edited grant gate claiming awarded funding.
15. A removed file from the manifest's fixed 13-file inventory.
16. A SHA-256 mismatch in any fixed filename or byte length.

The independent manifest includes a SHA-256 of the canonical ordered file-inventory representation. The verifier does not merely compare its own saved hash to the same untrusted string; it reconstructs source seals, arithmetic oracles, mock observations, statistical output, HTML and readiness gates first, then recomputes every file digest and the aggregate digest.

This design detects accidental or unsophisticated malicious edits to the local evidence, but a malicious editor who controls the complete verifier code and all files could fabricate a new self-consistent dossier. An immutable public code revision, external hash notarization and third-party replication are additional trust layers, not features claimed by this standalone patch.

## Installer safety tests

The installer must be idempotent when the exact same patch is installed twice. It must refuse installation without side effects when any of the six reviewed Phase 8/9 baseline hashes differ, any payload file differs from the packaged manifest, any listed path is duplicated, an unlisted path is present, a target destination exists with different bytes, or the project name is not exactly `fhec-v2`.

Test rollback in an expendable clone: introduce a local verifier failure after installation starts and confirm only newly copied, still-identical patch files are removed. No pre-existing Rust, TypeScript, Solidity, Cargo manifest, dependency lockfile, prior phase evidence, generated source, Git history, wallet or remote network state should be altered. Any user-modified new file is preserved and reported, not forcibly overwritten by rollback.

The local verifier must preserve environment variables it temporarily sets for the Hardhat mock process and restore the caller's original working directory even when a test fails. It must not read `FHEC_TESTNET_PRIVATE_KEY`, set a live-network consent phrase, invoke `hardhat run --network arb-sepolia`, or call transaction-submission JSON-RPC methods. No tool in this patch should need an HTTPS network endpoint for its core functionality after pinned dependencies have been installed.

## Command-level acceptance evidence

On a reviewed local workspace, the following should pass in the expected order:

1. `node --test` on all Phase 10 and Phase 11 test files.
2. `node tools/cofhe/phase10/verify-fixtures.mjs` with six matching fixtures.
3. `node tools/cofhe/phase89/source-seal.mjs --directory ... --verify ...` on the Phase 8/9 prepared source.
4. `node tools/cofhe/phase10/oracle.mjs --cases N --mode ... --seed ... --out ...` producing an immutable canonical corpus.
5. `pnpm exec hardhat compile --config phase89/hardhat.phase89.config.ts` with the prepared source directory.
6. `pnpm exec tsc --noEmit --project phase10/tsconfig.phase10.json` for the new test suite.
7. `pnpm exec hardhat test --config phase89/hardhat.phase89.config.ts phase10/test/FHECPhase10Properties.test.ts` with all cases and negative test passing.
8. A second check that prepared source hashes are identical after testing.
9. `node tools/cofhe/phase10/validate-results.mjs` accepting the exact raw mock result.
10. `node tools/cofhe/phase11/report-html.mjs` creating clearly limited offline evidence.
11. `node tools/cofhe/phase11/readiness.mjs` distinguishing local and external gates.
12. `node tools/cofhe/phase11/attest.mjs --directory ... --write ...` creating the fixed-file manifest.
13. The same `attest.mjs --verify` reproducing all checks from the stored dossier.

This chain is deliberately sequential, because each later stage requires the exact upstream output. Parallel process races could otherwise read half-written observations or a different source seal. Node source-level unit tests can run independently, but an immutable evidence chain must be assembled in order.

## Release and funding statement

Acceptable public summary after all checks: "FHEC V2 compiled source-sealed CoFHE Solidity and passed an independently generated full-width arithmetic corpus across its optimized and reference implementations under local CoFHE mocks. The exact source, corpus, mock observations and arithmetic validation are archived in a reproducible offline SHA-256 dossier."

Unacceptable claims from this evidence alone: "Every FHEC operation is formally proven correct," "we independently proved all remote coprocessor execution," "the model caps all CoFHE fees," "real testnet gas equals our mock gas," "mainnet is production-ready," "hosted CI is complete," or "a $20,000 grant is secured."
