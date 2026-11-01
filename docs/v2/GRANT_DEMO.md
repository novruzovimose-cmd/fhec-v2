# FHEC V2: Reproducible CoFHE compiler demonstration

## Technical claim

FHEC is a Rust-language FHE compiler with an additional Fhenix CoFHE Solidity backend. It can compile typed encrypted expressions and control-flow selection into Solidity using `@fhenixprotocol/cofhe-contracts/FHE.sol`. The generated library is called by smart-contract wrappers and tested against the Fhenix Hardhat **mock** environment.

The companion integration project is self-contained within `integration/cofhe-hardhat`. The developer no longer needs a separate sibling `fhec-cofhe-reference` checkout or its non-portable configuration.

## One-command Windows verification

Requirements: Rust stable with a functioning C++ linker, Node.js 24, pnpm 10, and normal dependency-registry access for the first installation.

```powershell
cd "$env:USERPROFILE\Documents\fhec"
Set-Location -LiteralPath "$env:USERPROFILE\Documents\fhec-v2"
& .\tools\cofhe\verify-grant-demo.ps1
```

The script performs formatting, focused Rust test and Clippy checks, invokes `fhec-cli cofhe` on `examples/cofhe_extended.fhe`, installs pinned Hardhat dependencies, compiles the generated Solidity, executes comparison and extended security mock tests, then restores the generated contract's previous state. It produces a timestamped JSON report, execution transcript and generated Solidity under `target/cofhe-grant-demo-*`.

Run the expensive full workspace regression suite as an additional validation gate:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
Set-Location -LiteralPath "$env:USERPROFILE\Documents\fhec-v2"
& .\tools\cofhe\verify-grant-demo.ps1 -FullWorkspace
```

For a second run with already installed Node dependencies:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
Set-Location -LiteralPath "$env:USERPROFILE\Documents\fhec-v2"
& .\tools\cofhe\verify-grant-demo.ps1 -SkipDependencyInstall
```

## Generated program

The source file `examples/cofhe_extended.fhe` includes:

- `pick_greater`: encrypted `u8` greater-than and branchless selection;
- `add_u16`: typed encrypted addition with a local binding;
- `multiply_u32`: encrypted multiplication;
- `greater_u64`: encrypted comparison resulting in an encrypted boolean;
- `twice_u8` and `four_times_u8`: intra-program function calls and `u8` modular arithmetic.

The Solidity wrapper files in `integration/cofhe-hardhat/contracts` consume the generated library, accept CoFHE encrypted inputs and proofs, and configure recipient access through the Fhenix `FHE` library.

The tests verify expected arithmetic values and boundary behavior, proof binding to submitter and consuming contract, ciphertext-proof mismatch rejection, and unauthorized decryption rejection.

## What was verified before this patch

On 2026-10-23 UTC, the Windows environment produced a JSON report with the following assertions: `rust_backend_tests`, `rust_cli_tests`, `rust_backend_clippy`, `solidity_compile`, `existing_comparison_test_suite`, `extended_execution_test_suite`, and reference restoration all passed.

The user-supplied report identified:

- Input source SHA-256: `2BE47D9EA0CA53EEF12D904A2F97D042CEF2DAA20E23053F99188557A7D0AFFE`
- Generated Solidity SHA-256: `79B5C60A6EF82FFE42AE33CFD1335CD8E3BC4F779DEDF84A495CAFDFDCF9DEE7`
- Environment: `Fhenix Hardhat CoFHE MOCK`

These hashes describe the previous run, not a proof of successful execution of the new self-contained harness. Re-run and compare the new report; record its outputs separately.

## CI

`.github/workflows/fhec-v2-verification.yml` contains two Rust matrix jobs (Windows and Ubuntu) and one Ubuntu Hardhat mock-execution job. Workflow content is prepared but **not verified by hosted GitHub Actions until the repository exists, is pushed with permission, and the workflows pass on the exact commit**.

## Presenting the evidence

1. Show the `.fhe` source and explain the typed encrypted operations.
2. Run the generator and show its `FHE.select`, comparison and arithmetic outputs.
3. Execute the Hardhat mock suites, including unauthorized-access rejection.
4. Show the JSON report and reproducible commit ID after hosted CI passes.
5. Explicitly state that public testnet execution and real network gas measurements remain future work.

Never describe mock gas estimates as testnet or production gas. Never describe mock testing as live threshold-FHE execution, a production security audit, or a deployment approval from Fhenix.
