# FHEC V2 CoFHE optimization experiment

This is a private engineering experiment that makes FHEC a source-to-Solidity optimization layer. The deployed smart contract still uses the official Fhenix CoFHE Solidity library and SDK.

## Compiler interface

The existing `fhec cofhe input.fhe --out generated.sol` remains unoptimized by default.

`fhec cofhe input.fhe --out optimized.sol --optimize --report report.json --library MyGeneratedLibrary` produces an optimized library and a structured static analysis report.

The optimizer validates the unoptimized input before applying transformations. It currently removes addition or subtraction of zero, multiplication by one, selections with the same branch result when the condition is a local variable, and repeated encrypted binary expressions across local bindings when no helper functions are involved.

The compiler deliberately does not simplify multiplication by zero, remove computed encrypted conditions, reorder operands, or common-subexpression-eliminate function calls. Those cases require stronger proofs about side effects and ciphertext handle semantics.

## Reproducible local verification

On Windows PowerShell from the V2 root:

`& .\tools\cofhe\verify-optimization.ps1 -SkipDependencyInstall`

Omit `-SkipDependencyInstall` on first run. The verifier checks formatting, Rust regression tests, clippy, generates baseline and optimized Solidity, validates a JSON report, compiles both libraries with the pinned Hardhat CoFHE dependency set, and checks encrypted plaintext equality with CoFHE mocks. Existing comparison and extended test suites run again. Temporarily staged generated libraries are restored or removed after the test.

The supplied fixture contains six direct encrypted operations across four function definitions before optimization and two afterwards (four fewer, 66% fewer in this fixture). These counts refer to the written functions, not a runtime call graph. They do not establish a 66% gas reduction or speedup; in particular, CoFHE mock gas is not representative of the deployed protocol.

## Scope and honest limitations

- A source-level operation is a syntactic `FHE.add`, `FHE.mul`, `FHE.select`, etc. A trivial `FHE.asEuint*` conversion is counted separately.
- Helper function invocations are counted separately. A helper invoked twice may execute its own encrypted operations twice, which the direct count does not model.
- Reported dependency depth is intraprocedural syntactic nesting, not measured execution latency.
- Optimized and baseline libraries may produce different encrypted handles or ACL artifacts even if decrypted values match. Permission/decryption behavior must be checked in each consuming application.
- Differential tests use official CoFHE Hardhat mocks, not a live threshold coprocessor. The experiment is not yet a testnet result.
- The integration may only be publicly claimed to show an operation-count reduction and equal results on the tested mock cases after all checks pass on the user's Windows environment.

## Next engineering gate

Use a dedicated testnet-only account with small faucet funds to deploy both variants on a supported testnet, verify encrypted execution/decryption, collect transaction hashes and receipts, and compare real network gas separately from the static report. Do not place any seed phrases, keys, `.env` files, or private RPC tokens in a public repository or a support request.
