# FHEC V2 CoFHE backend extension

This optional, additive V2 backend increment builds on the previously verified `pick_greater` CoFHE mock demo. It does not alter FHEC V1 or authorize publication.

## Supported source features

- `encrypted<u8>`, `encrypted<u16>`, `encrypted<u32>`, `encrypted<u64>` and `encrypted<bool>` function parameters and return values.
- Encrypted unsigned addition, subtraction, multiplication and comparisons.
- Constant-time encrypted selection via `FHE.select`.
- Straight-line local encrypted bindings followed by an explicit `return`.
- Calls between functions in the emitted `FHECGenerated` library.
- Integer constants converted to ciphertexts with a validated unsigned width.

The backend still rejects signed, plaintext and unsupported-width ABI types, statements after return, unsupported branch statements, and other unimplemented source patterns. Emission is not a security audit.

## Validation

Run the guarded script from Windows PowerShell after reviewing the patch:

```powershell
cd "$env:USERPROFILE\Documents\fhec"
& "$env:USERPROFILE\Documents\fhec-v2\tools\cofhe\verify-extended.ps1"
```

It runs Rust backend tests, Rust CLI regression tests, compiles the extended example into a unique directory under V2 `target`, temporarily installs the resulting Solidity in the Fhenix Hardhat starter, compiles it, and runs the previously passing five comparison tests. It restores the reference source afterward and verifies its SHA-256 hash. Do not report success unless the complete script prints its final success marker.

Passing mock tests does not demonstrate live CoFHE execution, true gas costs, production ACL behavior or contract security. Additional deployment, authorization-negative tests, profiling and independent review are required before representing the backend as production-ready.
