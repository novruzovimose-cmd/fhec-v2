# FHEC V1 security boundary and TFHE runtime notes

## 1. Threat model

The supported V1 workflow runs encryption, homomorphic Boolean circuit evaluation and decryption locally in one process/environment, using TFHE-rs. The local client key can be present in process memory. The intended protection is that the evaluated ciphertext computation does not require decrypting encrypted values at every conditional or gate; **it is not a guarantee against an attacker with control of the host OS, debugger or process memory**.

Do not claim remote zero-knowledge, multi-party security, encrypted key storage, protected memory, side-channel resistance, or independent cryptographic certification. No custom TFHE security proof is provided by the compiler's regression suite.

## 2. Key handling and data flow

The TFHE backend generates and holds client and server keys in process; Boolean ciphertexts are encrypted with the client key, operated on by the server/evaluation facilities and decrypted when the caller explicitly asks for results. `run` intentionally prints the decrypted result after successful execution. Real private inputs and their decrypted results should not be logged or stored as development fixtures.

B3 tests checked that persisted performance profiles contain measurements/hardware metadata rather than serialized client/server keys. This narrows exposure but is not a replacement for a full audit of every log, panic message, dependency, benchmark output or memory dump.

## 3. Encrypted control flow

An `if` condition with encrypted Boolean type is lowered to MUX operations. Its truth value is not decrypted to choose a plaintext control-flow path during circuit construction. The B3 TFHE test exercised the encrypted Boolean MUX truth table. Correctness tests are **not** a formal demonstration of constant-time execution across an entire host or defense against timing side channels from compilation structure.

For dependent control flow, both branches must compile into compatible circuit output types. Unsupported mixing of `encrypted<T>` and nonencrypted operands must be rejected unless explicit source-language type semantics allow it; the current subset intentionally does not support arbitrary plaintext/encrypted coercion.

## 4. CLI exposure

The `--inputs a,b` argument can expose values to shell history, process inspection and operating-system telemetry. The B2 alternative is `--inputs-stdin`, which accepts one bounded line (not an interactive secret-management channel). This reduces argument exposure but does not protect against terminal echo, malware, shell pipeline logging, OS tracing, process snapshots or printed decrypted output. Input-value validation diagnostics should describe the type/range error without repeating a rejected plaintext value; C2 executable tests exercise that policy.

Avoid command examples involving real personal data. Public demo values `37,5` are used in acceptance only.

## 5. Malformed IR and execution errors

Frontend type checking, IR operand validation, circuit verification and executor validation must reject malformed function wiring, missing operands, incorrect input counts and bad output references without invoking undefined behavior. A-stage and B-stage regression tests cover representative cases, including injected backend failures across execution strategies. They do not exhaust every malformed internal value.

Runtime error paths must not be silently converted into success. Audit panic paths (`unwrap`, `expect`, `panic!`) individually: many occur only in tests or in formatting of an in-memory `String`, while production paths accepting malformed external input deserve stricter handling. A grep tally is not a severity count or exploitability proof.

## 6. Hardware profile trust

Saved `target/fhec-profiles/tfhe-v4.json` files are performance calibration metadata, **not secrets** or authenticated statements of correctness. They are local untrusted inputs to latency and scaling estimation. Load validation checks schema, hardware compatibility, widths and size limits. Stage B3 added safer temporary-file persistence, but on Windows replacing the old file through a backup swap remains **not crash-atomic**.

The profile directory should have trusted local permissions and a single authorized writer. An attacker able to mutate the parent directory or its contents concurrently can interfere with profile replacement. A valid profile does not authorize untrusted circuit execution.

## 7. Noise and parameter limitations

TFHE Boolean gates use the underlying TFHE-rs implementation and its parameter choices; analyzer reports of bootstraps, key switches, refreshes, memory and noise pressure are model-derived and backend-specific. They must not be claimed as independent instrumentation or a guarantee about BFV/BGV/CKKS levels. V1 does not contain arbitrary FHE scheme switching, user-defined cryptographic parameter optimization or a generic automatic cross-scheme noise scheduler.

## 8. Release security reviews still outstanding

- Dependency advisory scan with a dated report, including transitive dependencies and advisories discovered at release time.
- License inventory and compatibility review covering Rust dependencies and non-Rust native dependencies.
- Secret scan of the exact intended Git tree, build artifact and source archives; `.gitignore` alone is insufficient.
- Clean-checkout validation on supported Windows and Linux environments and current native TFHE dependencies.
- Confirmation that no real input or decrypted output has been unintentionally included in examples or benchmark JSON.
- Independent security analysis/certification (optional for a research V1 but required for any strong security-assurance claim).

These are not automatically passed by Stage C3 documentation checks. Keep the related D-G release checklist items open until supported by results.
