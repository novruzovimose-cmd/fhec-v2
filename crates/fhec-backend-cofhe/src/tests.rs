use fhec_frontend::{Lexer, Parser};

use super::emit_library;

fn emit(source: &str) -> Result<String, String> {
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    emit_library(&program).map_err(|error| error.to_string())
}

#[test]
fn encrypted_selection_uses_cofhe_select() {
    let solidity = emit(
        "fn pick_greater(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { a } else { b }; }",
    )
    .unwrap();

    assert!(solidity.contains("function pick_greater(euint8 a, euint8 b)"));
    assert!(solidity.contains("return FHE.select(FHE.gt(a, b), a, b);"));
    assert!(!solidity.contains("if ("));
}

#[test]
fn unsigned_widths_emit_correct_solidity_types() {
    for (width, expected) in [
        (8, "euint8"),
        (16, "euint16"),
        (32, "euint32"),
        (64, "euint64"),
    ] {
        let source = format!(
            "fn add(a: encrypted<u{width}>, b: encrypted<u{width}>) -> encrypted<u{width}> {{ return a + b; }}"
        );
        let solidity = emit(&source).unwrap();
        assert!(solidity.contains(&format!(
            "function add({expected} a, {expected} b) internal returns ({expected})"
        )));
        assert!(solidity.contains("return FHE.add(a, b);"));
    }
}

#[test]
fn multiplication_is_supported() {
    let solidity = emit(
        "fn multiply(a: encrypted<u16>, b: encrypted<u16>) -> encrypted<u16> { return a * b; }",
    )
    .unwrap();
    assert!(solidity.contains("return FHE.mul(a, b);"));
}

#[test]
fn comparison_returns_encrypted_boolean() {
    let solidity = emit(
        "fn greater(a: encrypted<u32>, b: encrypted<u32>) -> encrypted<bool> { return a > b; }",
    )
    .unwrap();
    assert!(solidity.contains("returns (ebool)"));
    assert!(solidity.contains("return FHE.gt(a, b);"));
}

#[test]
fn local_bindings_and_function_calls_are_lowered() {
    let solidity = emit(
        "fn twice(a: encrypted<u16>) -> encrypted<u16> { return a + a; } fn quadruple(a: encrypted<u16>) -> encrypted<u16> { let x = twice(a); return twice(x); }",
    )
    .unwrap();
    assert!(solidity.contains("euint16 x = twice(a);"));
    assert!(solidity.contains("return twice(x);"));
}

#[test]
fn annotated_local_binding_compiles() {
    let solidity = emit(
        "fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { let total: encrypted<u8> = a + b; return total; }",
    )
    .unwrap();
    assert!(solidity.contains("euint8 total = FHE.add(a, b);"));
}

#[test]
fn integer_constant_is_encrypted_in_operand_context() {
    let solidity =
        emit("fn threshold(a: encrypted<u16>) -> encrypted<bool> { return a > 255; }").unwrap();
    assert!(solidity.contains("FHE.gt(a, FHE.asEuint16(255))"));
}

#[test]
fn encrypted_arithmetic_accepts_integer_literal() {
    let solidity = emit("fn inc(a: encrypted<u8>) -> encrypted<u8> { return a + 1; }").unwrap();
    assert!(solidity.contains("FHE.add(a, FHE.asEuint8(1))"));
}

#[test]
fn unsupported_widths_are_rejected() {
    for width in [1, 7, 24, 128, 256] {
        let source =
            format!("fn add(a: encrypted<u{width}>) -> encrypted<u{width}> {{ return a; }}");
        let error = emit(&source).unwrap_err();
        assert!(error.contains("Unsupported CoFHE encrypted type"));
    }
}

#[test]
fn unsupported_signed_values_are_rejected() {
    let error = emit("fn signed(a: encrypted<i8>) -> encrypted<i8> { return a; }").unwrap_err();
    assert!(error.contains("Unsupported CoFHE encrypted type"));
}

#[test]
fn plaintext_parameters_are_rejected() {
    let error = emit("fn leak(a: u8) -> encrypted<u8> { return a; }").unwrap_err();
    assert!(error.contains("Type checking failed") || error.contains("requires encrypted"));
}

#[test]
fn out_of_range_integer_literal_is_rejected() {
    let error =
        emit("fn overflow(a: encrypted<u8>) -> encrypted<u8> { return a + 256; }").unwrap_err();
    assert!(error.contains("exceeds encrypted<u8> range"));
}

#[test]
fn incompatible_operands_are_rejected() {
    let error =
        emit("fn bad(a: encrypted<u8>, b: encrypted<u16>) -> encrypted<u8> { return a + b; }")
            .unwrap_err();
    assert!(error.contains("Type checking failed"));
}

#[test]
fn comparison_cannot_be_returned_as_integer() {
    let error =
        emit("fn bad(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a > b; }")
            .unwrap_err();
    assert!(error.contains("Type checking failed"));
}

#[test]
fn reserved_solidity_identifiers_are_rejected() {
    let error = emit("fn contract(a: encrypted<u8>) -> encrypted<u8> { return a; }").unwrap_err();
    assert!(error.contains("Reserved Solidity identifier"));
}

#[test]
fn imported_cofhe_identifiers_cannot_be_shadowed() {
    let error = emit("fn bad(FHE: encrypted<u8>) -> encrypted<u8> { return FHE; }").unwrap_err();
    assert!(error.contains("cannot be shadowed"));
}

#[test]
fn unreachable_statements_are_rejected() {
    let error =
        emit("fn bad(a: encrypted<u8>) -> encrypted<u8> { return a; let x = a; }").unwrap_err();
    assert!(error.contains("Unreachable statement"));
}

#[test]
fn unsupported_branch_statements_are_rejected() {
    let error = emit(
        "fn bad(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { let c = a; a } else { b }; }",
    )
    .unwrap_err();
    assert!(error.contains("selection branches require one result expression"));
}
