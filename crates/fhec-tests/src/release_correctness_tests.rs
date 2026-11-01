use fhec_analyzer::v4::PredictionTracker;
use fhec_backend::FheBackend;
use fhec_backend_tfhe::TfheBackend;
use fhec_compiler::Compiler;
use fhec_frontend::{lexer::Lexer, parser::Parser};
use fhec_runtime::v4::execute_auto_v4_profiled;

fn encode(value: u8) -> Vec<bool> {
    (0..8).map(|bit| (value & (1u8 << bit)) != 0).collect()
}

fn decode(bits: &[bool]) -> u8 {
    assert_eq!(bits.len(), 8);

    bits.iter().enumerate().fold(
        0u8,
        |value, (index, bit)| {
            if *bit { value | (1u8 << index) } else { value }
        },
    )
}

fn execute_case(
    backend: &TfheBackend,
    model: &fhec_analyzer::v4::V4CostModel,
    expression: &str,
    lhs: u8,
    rhs: u8,
) -> u8 {
    let source = format!(
        "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> {{ return {expression}; }}"
    );

    let tokens = Lexer::new(&source).tokenize().expect("lexing failed");
    let program = Parser::new(tokens).parse().expect("parsing failed");

    let compiler = Compiler::new();
    let module = compiler.lower(&program).expect("IR lowering failed");

    let function = module
        .functions
        .iter()
        .find(|function| function.name == "test")
        .expect("test function missing");

    let selection = compiler
        .lower_to_circuit_v4(function, model)
        .expect("circuit selection failed");

    selection
        .circuit_function
        .verify(function)
        .expect("circuit verification failed");

    assert_eq!(selection.circuit_function.outputs.len(), 8);

    let inputs = encode(lhs)
        .into_iter()
        .chain(encode(rhs))
        .map(|bit| backend.encrypt_bool(bit).expect("encryption failed"))
        .collect::<Vec<_>>();

    let mut tracker = PredictionTracker::default();

    let (outputs, _) = execute_auto_v4_profiled(
        backend,
        &selection.circuit_function.circuit,
        &inputs,
        &selection.circuit_function.outputs,
        model,
        &mut tracker,
    )
    .expect("TFHE execution failed");

    let plaintext = outputs
        .iter()
        .map(|output| backend.decrypt_bool(output).expect("decryption failed"))
        .collect::<Vec<_>>();

    decode(&plaintext)
}

#[test]
fn real_tfhe_arithmetic_boundary_matrix() {
    let backend = TfheBackend::new();

    let (_, model) = backend
        .calibrate_v4_cost_model(1)
        .expect("TFHE calibration failed");

    let cases = [
        ("a + b", 255u8, 1u8, 0u8),
        ("a - b", 0u8, 1u8, 255u8),
        ("a * b", 16u8, 16u8, 0u8),
        ("a * b", 255u8, 2u8, 254u8),
        ("(a + b) - b", 250u8, 20u8, 250u8),
        ("(a - b) + b", 3u8, 7u8, 3u8),
    ];

    for (expression, lhs, rhs, expected) in cases {
        let actual = execute_case(&backend, &model, expression, lhs, rhs);

        assert_eq!(
            actual, expected,
            "expression={expression}, lhs={lhs}, rhs={rhs}"
        );
    }
}

#[test]
fn real_tfhe_arithmetic_matches_rust_wrapping_semantics() {
    let backend = TfheBackend::new();

    let (_, model) = backend
        .calibrate_v4_cost_model(1)
        .expect("TFHE calibration failed");

    let cases = [(0u8, 0u8), (1u8, 255u8), (128u8, 2u8), (37u8, 5u8)];

    for (lhs, rhs) in cases {
        for (expression, expected) in [
            ("a + b", lhs.wrapping_add(rhs)),
            ("a - b", lhs.wrapping_sub(rhs)),
            ("a * b", lhs.wrapping_mul(rhs)),
        ] {
            let actual = execute_case(&backend, &model, expression, lhs, rhs);

            assert_eq!(
                actual, expected,
                "expression={expression}, lhs={lhs}, rhs={rhs}"
            );
        }
    }
}

#[test]
fn invalid_source_does_not_reach_cryptographic_execution() {
    let invalid_sources = [
        "fn broken(",
        "fn test(a: encrypted<u8>) -> encrypted<u8> { return a + ; }",
    ];

    for source in invalid_sources {
        let tokens = Lexer::new(source).tokenize();

        if let Ok(tokens) = tokens {
            assert!(
                Parser::new(tokens).parse().is_err(),
                "invalid source unexpectedly parsed: {source}"
            );
        }
    }
}
