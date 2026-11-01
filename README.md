# FHEC — FHE-Native Circuit Compiler

FHEC is an experimental Rust compiler for encrypted computations using a TFHE backend.

## Current V1 scope

- Lexer, parser, type checking and FHE semantic analysis
- Intermediate representation and Boolean circuit lowering
- Circuit verification and optimization
- Real TFHE encrypted execution
- Hardware-aware candidate selection
- Analysis, profiling and benchmarking commands

Supported encrypted types include u8, i8, u16 and bool, within the implemented language subset.

## Requirements

Rust 1.85 or later, Cargo, and a supported native build toolchain.

On Windows, install the Visual Studio C++ build tools.

## Build

    cargo build --release --locked -p fhec-cli

## Compile and analyze

    cargo run --release -p fhec-cli -- build examples/addition.fhe
    cargo run --release -p fhec-cli -- analyze examples/addition.fhe

## Encrypted execution

    cargo run --release -p fhec-cli -- run examples/addition.fhe --function add --inputs 37,5

Expected decrypted result: 42.

## Verification

    cargo fmt --all -- --check
    cargo check --workspace --locked
    cargo test --workspace --locked

## Limitations

FHEC does not support arbitrary Rust programs or every FHE scheme.

Performance predictions require appropriate calibration. Default cost-model output must not be interpreted as measured execution time.

FHEC has not undergone an independent cryptographic security audit.

See docs/architecture.md and docs/v1-release-checklist.md.
