# FHEC V1 supported-language reference

## 1. Contract and scope

FHEC V1 is a small, statically typed language for describing computations over TFHE-backed encrypted Boolean circuits. The frontend includes a lexer, parser, AST, type checker, and FHE-aware semantic checks. The compiler lowers its supported subset through an internal IR to a circuit, selects a candidate under a cost model, verifies circuit wiring, and can execute on the TFHE backend.

This is **not Rust syntax compatibility**, general-purpose compilation, or a promise that every grammar token is executable. This reference distinguishes parser syntax from execution-backed support. The operator matrix below is grounded in the earlier source snapshot plus locally passed tests reported during stages A, B, C1 and C2; the C3 verifier checks its example claims against the user's current checkout.

## 2. Functions and types

A function declaration has the form:

```fhe
fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> {
    return a + b;
}
```

- `fn`, a function name, parameter list, `->` and a return type are required by the parser.
- Braced blocks contain `let`, `return`, or expression statements.
- Identifiers begin with an ASCII letter or `_`, followed by ASCII letters, digits or `_`.
- Supported encrypted CLI input/output types are `encrypted<u8>`, `encrypted<i8>`, `encrypted<u16>` and `encrypted<bool>`; applicability depends on operator and lowering support.
- The frontend also represents plain `bool`, integer widths, and `plaintext<T>` in its type system. **Representability in the AST is not a guarantee that every such type can be used as an executable encrypted function parameter.**
- `encrypted<T>` encloses the type in angle brackets; nested encrypted wrappers, arbitrary integer widths, and arbitrary user-defined types are not guaranteed executable.

### Source examples

```fhe
fn greater(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> {
    return a > b;
}
```

```fhe
fn larger(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> {
    return if a > b {
        a
    } else {
        b
    };
}
```

`if` is an expression, not a JavaScript/C-style statement with implicit plaintext control flow. Branches with an encrypted condition are lowered to circuit-level MUX operations. The compiler does **not** decrypt the condition to decide which branch to evaluate.

```fhe
fn mix(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> {
    let sum = a + b;
    return sum - a;
}
```

Type-inferred `let` bindings and ordinary function calls are supported in tested cases, including forward references and nested calls. Recursive calls are rejected by the inliner; loops and arbitrary recursion are not in the supported V1 subset.

## 3. Operator and value matrix

| Syntax / operation | V1 executable scope | Caveats |
| --- | --- | --- |
| `a + b` | encrypted `u8`, `i8`, `u16` cases | Fixed-width, wrapping result semantics; verify literal interactions by type |
| `a - b` | encrypted integer cases | Fixed-width, wrapping result semantics |
| `a * b` | encrypted integer cases, notably `u8`/`i8` tests | Computationally expensive; do not assume arbitrary widths or optimal circuits |
| `==`, `!=` | corresponding comparable operands, returning encrypted Boolean for encrypted operands | Mixed encrypted/plaintext operands rejected unless explicitly allowed by type rules |
| `<`, `<=`, `>`, `>=` | supported encrypted integer comparison cases, including signed `i8` | Signed comparison honors two's-complement ordering in tested cases |
| `if condition { x } else { y }` | compatible branch results with Boolean condition | Encrypted condition lowers to MUX; both paths must be valid |
| `let x = expression;` | local bindings with inference | Duplicate declarations and out-of-scope references rejected |
| `return expression;` | function result | Must match declared result type |
| `f(x, y)` | statically known nonrecursive function calls | Correct argument count and types; recursion rejected |
| `true`, `false` | Boolean literals in parser and supported expressions | No blanket claim for all mixed Boolean/encrypted contexts |
| `a && b`, `a || b`, `a ^ b` | **Not advertised as supported source operators** | Circuit IR has Boolean gates; this does not make the text syntax legal |
| `a / b` | **Not supported** in the executable V1 subset | Slash token existence does not imply parser/executor support |
| Unary source `-x` | **Not advertised** | CLI may accept signed negative inputs such as `-3`, a different interface |
| `for`, `while`, arrays, structs, pointers | **Not supported** | No general control flow or aggregate-language support |

Operator precedence in the inspected parser is multiplication before addition/subtraction, then comparisons. Parentheses explicitly group expressions. Do not rely on chained comparisons being mathematically meaningful; write a Boolean selection explicitly or separate comparisons into supported functions.

## 4. Integer semantics

Encrypted integers are represented as bit vectors, with unsigned and signed interpretations handled at the type boundaries. A `u8` result cannot represent 256, so fixed-width addition and multiplication can wrap. Likewise, signed `i8` operations can wrap according to their 8-bit two's-complement representation. The release optimizer's constant folding must not invent out-of-width values; that property is covered by A3 tests.

The command-line `--inputs` value is a comma-separated sequence of decimal values matching the selected function parameters. Boolean parameters have strict accepted forms as tested by the CLI suite. Negative signed integer values belong in CLI input parsing, not necessarily source unary expression syntax.

For example, if `add` accepts two `encrypted<u8>` parameters, the command `run ... --function add --inputs 37,5` encrypts each input, evaluates the circuit, decrypts the output and prints `42`. The demo values are **public**; never mistake this output for a confidentiality guarantee.

## 5. What a successful `build` actually means

The current `build` command lexes and parses source, lowers the functions, produces a Boolean circuit per function, verifies input/output wiring and prints a summary including bit counts. **It does not write a reusable `.fhec` executable/circuit artifact** in the inspected V1 design. A `Build successful.` message proves a build/verification command completed, not that a distributable binary was emitted. `run` performs actual encrypted execution, and `analyze` produces a candidate/cost report.

If the repository later adds a persistent artifact writer, update this section and tests rather than silently redefining `build`.

## 6. Error classes

Expected errors include malformed tokens/syntax; wrong return types; encrypted/plaintext mixing; undefined variables; unknown functions; incorrect call arity; recursion; missing else or mismatched branch results; out-of-range CLI input values; unknown CLI commands; and invalid circuit/IR references. A rejected program should exit nonzero, not reach encrypted evaluation. The CLI intentionally redacts rejected user-supplied plaintext values; debugging logs should avoid including actual user inputs.

Avoid assuming every validation produces a specific English sentence. Test error classes and exit codes rather than pinning human-facing wording without versioned contracts.

## 7. Representative test programs

Included with C3 (under `examples/c3/`):

- `nested_selection.fhe`: source arithmetic combined with encrypted comparison and conditional choice.
- `signed_comparison.fhe`: signed `i8` ordering.
- `wide_addition.fhe`: `u16` arithmetic.
- `boolean_selection.fhe`: encrypted Boolean condition and Boolean branch values.

C3 verification runs `build` and `analyze` for these samples. A successful compile/analysis does **not** prove every input/output combination; the A-stage exhaustive and B-stage real TFHE tests supply complementary evidence.

## 8. Intentionally excluded from V1

No arrays/structs, loops, unrestricted recursion, GPU execution, OpenFHE, CKKS, BGV, BFV, multi-party key management, or general-purpose Rust parsing. A future release may add these; they are not implicit V1 promises.

## 9. Source pointers for maintainers

- `crates/fhec-frontend/src/{lexer.rs,parser.rs,ast.rs,type_checker.rs,fhe_analysis.rs}`
- `crates/fhec-compiler/src/{lowering.rs,circuit_lowering.rs,v4.rs}`
- `crates/fhec-ir/src/{types.rs,operation.rs,verifier.rs}`
- `crates/fhec-cli/src/{main.rs,cli_run.rs}`
- `crates/fhec-tests/src/lib.rs`

These are pointers, not formal statements that every function is free of panic paths. The production panic audit and adversarial IR coverage remain independent signoff items.
