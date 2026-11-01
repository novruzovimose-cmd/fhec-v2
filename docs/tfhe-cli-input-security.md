# FHEC V1 CLI Input Confidentiality

FHEC V1's local `fhec run` command is **not** a remote computation service. The local process creates both the TFHE client (secret) key and evaluation key. It encrypts inputs, runs the Boolean ciphertext circuit, and decrypts the output in that same process. The secret key is intentionally needed for decryption; this workflow does not protect values from the local operating system, process debugger, terminal, shell, or an adversary controlling the execution host.

## Recommended input channel

`fhec run examples/addition.fhe --function add --inputs-stdin` reads one comma-separated line from standard input (maximum 4096 bytes). Pipe that line from a trusted source. It no longer needs to appear as a command-line argument. For a non-sensitive smoke test in PowerShell:

```powershell
'37,5' | cargo run --release -p fhec-cli -- run examples/addition.fhe --function add --inputs-stdin
```

The `--inputs <a,b>` form remains supported for examples and non-sensitive test values, but argv may be observable through process inspection, command-history capture, monitoring software, or CI job logs. A literal in a PowerShell command may also enter shell history, including if it is piped to stdin. **Do not paste genuine secrets into a shell command** expecting `--inputs-stdin` to hide them. An interactive terminal may echo typed stdin; use a trusted, non-echoing source when confidentiality is required. Standard input is not a secure secret vault.

FHEC does not print supplied input values in normal status output, and failed input conversion does not quote rejected values. Errors describe expected types/ranges instead. Program source paths and function names may still appear in diagnostics. Successful runs intentionally print the **decrypted output**. Treat that output and its terminal/log destination as sensitive when appropriate. Captured stdout/stderr, memory dumps, shell history and external monitoring are outside FHEC V1's protection boundary.

## Key and profile boundaries

The TFHE backend generates a fresh in-process client/evaluation key pair. The client key is held in a private backend field; existing runtime APIs use it to encrypt/decrypt Boolean ciphertexts and use the evaluation key for circuit gates. The V1 hardware profile contains **performance calibration metadata, not keys or plaintext input values**. Do not serialize the backend object, client key, ciphertexts or plaintext arguments to diagnostics or benchmark output. In-memory plaintext and secret-key data are **not** guaranteed to be locked in RAM, zeroized at process exit, or safe from local memory inspection.

The runtime's noise diagnostics are backend-specific TFHE gate-operation estimates; they do **not** implement a multi-scheme, general-purpose noise-budget scheduler or constitute an independent cryptographic security audit.

## Verification

Run `cargo test --locked -p fhec-cli -p fhec-runtime`, `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked`. The CLI unit tests exercise redacted error text and bounded stdin; the runtime's Boolean mock tests cover a 33-bit mixed-width packing layout and both wide/shallow and narrow/deep execution shapes across sequential, parallel, chunked, and automatic schedules. Mock-backend shape tests check scheduling behavior, **not** TFHE cryptographic security. Production-scale or remote threat models require separate review.
