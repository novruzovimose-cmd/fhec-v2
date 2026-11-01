# FHEC V1 CLI Workflow (Stage C draft)

Run these commands from the project root after installing the Rust toolchain and the required native C/C++ tooling for the TFHE dependencies.

## Build and analyze

```powershell
cargo build --release --locked -p fhec-cli
cargo run --release --locked -p fhec-cli -- build examples/conditional_mux.fhe
cargo run --release --locked -p fhec-cli -- analyze examples/conditional_mux.fhe
```

The package is named `fhec-cli`; the executable's final filename depends on its Cargo binary target definition. The `fhec` spelling shown in CLI help is an interface name, **not** proof that installing this package creates a `fhec` executable. Inspect `target/release` or `Cargo.toml` to verify the shipped binary name.

`build` currently parses, lowers, constructs, and verifies functions, then prints a summary. It **does not serialize a reusable compiled artifact** in the inspected implementation. Do not describe it as an artifact-producing compiler command.

## Encrypted execution

```powershell
cargo run --release --locked -p fhec-cli -- run examples/conditional_mux.fhe --function pick_greater --inputs 37,5
```

The B2 alternative is `--inputs-stdin` for data that should not appear directly in command-line argument listings. Both invoke real TFHE execution and intentionally display the decrypted result; input secrecy is not equivalent to output secrecy. Avoid checking real private input values into logs, source files, or scripts.

## Benchmark workflow

```powershell
cargo run --release --locked -p fhec-cli -- bench --iterations 3
```

`bench` launches Cargo internally from the current directory. It requires the FHEC source checkout and development toolchain; it is **not** a self-contained benchmarking subcommand after copying only the CLI executable to another machine. For accurate encrypted execution timing, explicitly select `--mode tfhe`, `--mode optimizer`, or `--mode compare`; see `docs/benchmark-methodology.md`.

## Error behavior

Malformed sources, nonexistent input files, invalid numeric inputs, missing calibration profiles in comparison mode, and rejected CLI arguments should return nonzero process exit codes. This remains a release verification item until negative-path CLI integration tests pass against the final V1 build.

This file is a scoped workflow guide, not a declaration that Windows/Linux clean-install checks are complete.
