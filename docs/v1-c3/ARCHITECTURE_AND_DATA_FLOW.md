# FHEC V1 implementation-oriented architecture

## Scope

The earlier `docs/architecture.md` explains the intended multi-stage FHEC architecture. This document narrows that broad design to the **observable, supported V1 data path** and avoids implying unimplemented backends or source-language capabilities.

## Source-to-circuit

```text
.fhe source
  -> lexical tokens
  -> parser / AST
  -> type and encrypted-operation checks
  -> typed intermediate representation
  -> function-call inlining and encrypted branch/MUX lowering
  -> fixed-width Boolean circuit lowering
  -> circuit and operand validation
  -> optional supported IR optimizations
  -> candidate generation (ripple carry / parallel prefix)
  -> hardware-aware cost comparison (if a suitable model is supplied)
  -> selected circuit
```

The CLI build path currently constructs and verifies its circuit in memory, prints counts and stops. It does not export a persistent circuit artifact. The analyzer path compares multiple candidates using the **default uncalibrated cost model** and prints structural, cost, uncertainty and IR attribution reports. It does not directly time the encrypted evaluator.

## Encrypted execution

```text
Selected function and typed CLI inputs
  -> validate values and widths
  -> generate or obtain local TFHE client/server keys
  -> encrypt each typed input into Boolean ciphertext bits
  -> evaluate Boolean gates (including encrypted MUX)
  -> decrypt designated output bits locally
  -> reconstruct declared output type
  -> print decrypted result
```

The current use is single-machine/local. It does not implement an isolated remote evaluator protocol, distributed key custody or homomorphic handling of general programs. Runtime execution strategy can choose sequential, parallel, layered or chunked schedules based on circuit shape and the available model; injected-error and wide/deep-shape tests cover representative boundary conditions.

## Workspace crates

| Crate | Primary V1 role |
| --- | --- |
| `fhec-frontend` | Source tokenization, syntax tree, static type and encrypted semantics |
| `fhec-ir` | Typed IR values, instructions, functions, verifier |
| `fhec-optimizer` | Fixed-width folding, Boolean simplification, safe supported rewrites, hint planning |
| `fhec-circuit` | Boolean wires, gates, circuit correctness and clear evaluator |
| `fhec-compiler` | Orchestration, IR-to-circuit lowering, candidates, selection and report assembly |
| `fhec-analyzer` | Gate/depth, strategy, modeled cost, profiles, uncertainty, noise, attribution |
| `fhec-backend` | Backend interfaces and generic circuit execution |
| `fhec-backend-tfhe` | Real TFHE Boolean-gate cryptography, profiles and persistence |
| `fhec-runtime` | Execution strategy selection and runtime coordination |
| `fhec-cli` | Build/analyze/run/profile/bench command interface |
| `fhec-bench` | Clear, TFHE, optimizer and candidate comparison benchmark tools |
| `fhec-tests` | End-to-end and real encrypted regression coverage |

## Optimization ordering and safety

Optimizer candidates are not freely reordered past their definitions. Earlier A-stage regressions guard overlapping reassociation rewrites and IR dominance (operand must be defined before use). Arithmetic folding respects declared width and signed bit interpretation. Multiple candidate implementations are verified and compared under the same cost policy; lower modeled latency wins. This does **not** substitute for differential equivalence testing on every possible FHE program.

## Analyzer/TFHE division of responsibility

TFHE-rs is responsible for underlying encrypted primitive semantics and parameter implementation. FHEC's analyzer accounts for declared operation categories, depth, predicted latency, memory, intervals and noise-pressure assumptions. Modeled counts and noise results are not verified TFHE kernel traces or generalized noise scheduling for other schemes.

## Storage and profiles

`Cargo.lock` pins the dependency graph for development/test. `target/` stores compiled binaries, ephemeral benchmark outputs and local profile JSON. Profiles are validated against structural bounds and hardware/backend compatibility and reused when possible. Stage B3 made profile writes safer but did not promise crash-atomic Windows replacement. No private TFHE key material should be serialized into the performance profile schema.

## Current product boundary

Supported V1: TFHE Boolean circuits, selected encrypted integer types, comparisons, conditional MUX, function inlining, developer CLI and benchmarking. Not supported V1: GPU, OpenFHE/BFV/BGV/CKKS, arrays/structs/loops, multiparty client/server key custody, generic automatic noise-budget scheduling, or an independently certified security implementation.
