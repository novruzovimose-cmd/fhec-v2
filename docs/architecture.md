# FHE-Native Circuit Compiler Architecture

## 1. Overview

The FHE-Native Circuit Compiler (FHEC) is a Rust-based compiler that transforms
high-level programs operating on encrypted data into executable homomorphic
circuits.

The compiler is designed to hide cryptographic implementation details from
the programmer while preserving the semantics of the original program.

The initial target backend is TFHE-rs.

---

## 2. Compilation Pipeline

The compiler follows this pipeline:

Source Code
    ↓
Lexer
    ↓
Parser
    ↓
AST
    ↓
Type Checker
    ↓
FHE Semantic Analysis
    ↓
High-Level IR
    ↓
Control-Flow Analysis
    ↓
FHE IR
    ↓
Circuit Lowering
    ↓
Circuit Optimization
    ↓
Backend Lowering
    ↓
Executable FHE Circuit

---

## 3. Workspace Structure

The project is organized as a Cargo workspace.

### fhec-cli

Command-line interface for the compiler.

Responsibilities:

- Parse command-line arguments
- Load source files
- Invoke the compiler
- Display diagnostics
- Inspect generated IR and circuits
- Run benchmarks

### fhec-frontend

Source-language frontend.

Responsibilities:

- Lexing
- Parsing
- AST construction
- Source diagnostics
- Type checking
- FHE-specific semantic analysis

### fhec-ir

Intermediate representation definitions.

Responsibilities:

- Types
- Values
- Basic blocks
- Instructions
- Functions
- Modules
- FHE-specific operations

### fhec-optimizer

Optimization passes.

Responsibilities:

- Constant folding
- Dead-code elimination
- Boolean simplification
- Gate reduction
- Circuit-size optimization
- Noise-aware optimization

### fhec-circuit

Backend-independent circuit representation.

Responsibilities:

- Wires
- Gates
- Inputs
- Outputs
- Circuit construction
- Circuit inspection

### fhec-backend

Backend abstraction layer.

Responsibilities:

- Define the FHE backend interface
- Describe backend capabilities
- Execute generated circuits
- Provide backend-independent interfaces

### fhec-backend-tfhe

TFHE-rs backend.

Responsibilities:

- Translate compiler circuits into TFHE-rs operations
- Encrypt inputs
- Evaluate circuits
- Decrypt outputs
- Handle TFHE-specific constraints

### fhec-runtime

Runtime support for compiled programs.

Responsibilities:

- Runtime abstractions
- FHE execution support
- Versioning
- Runtime utilities

### fhec-compiler

Compiler orchestration.

Responsibilities:

- Coordinate frontend
- Build IR
- Run optimization
- Lower IR to circuits
- Select backend
- Produce compiler output

### fhec-tests

Integration and compiler tests.

Responsibilities:

- End-to-end compilation tests
- Semantic tests
- Circuit correctness tests
- Backend tests

### fhec-bench

Performance benchmarks.

Responsibilities:

- Compilation benchmarks
- Circuit generation benchmarks
- Optimization benchmarks
- FHE execution benchmarks

---

## 4. Core FHE Types

The compiler will eventually support concepts such as:

- `Encrypted<T>`
- `Plaintext<T>`
- `Ciphertext<T>`
- `FheBool`
- `FheInt`
- `FheUInt`
- `Bit`
- `Circuit`
- `Gate`
- `Mux`
- `NoiseBudget`
- `BootstrappingPoint`

Encrypted values must not be treated like ordinary plaintext values.

---

## 5. Encrypted Control Flow

Ordinary branching cannot directly be performed on encrypted values.

For example:

```rust
if encrypted_condition {
    a
} else {
    b
}