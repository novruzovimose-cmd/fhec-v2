use fhec_frontend::{Lexer, Parser};

use super::{CoFheOptions, compile_cofhe, emit_library};

fn compile(source: &str, optimize: bool) -> Result<super::CoFheArtifact, String> {
    let tokens = Lexer::new(source)
        .tokenize()
        .map_err(|e| format!("{e:?}"))?;
    let program = Parser::new(tokens).parse().map_err(|e| format!("{e:?}"))?;
    compile_cofhe(
        &program,
        &CoFheOptions {
            optimize,
            library_name: "FHECOptimized".to_owned(),
        },
    )
    .map_err(|error| error.to_string())
}

#[test]
fn addition_and_subtraction_identity_do_not_emit_tasks() {
    let source = "fn one(a: encrypted<u8>) -> encrypted<u8> { return a + 0; } fn two(a: encrypted<u8>) -> encrypted<u8> { return 0 + a; } fn three(a: encrypted<u8>) -> encrypted<u8> { return a - 0; }";
    let baseline = compile(source, false).unwrap();
    let optimized = compile(source, true).unwrap();
    assert_eq!(baseline.solidity.matches("FHE.add(").count(), 2);
    assert_eq!(baseline.solidity.matches("FHE.sub(").count(), 1);
    assert!(!optimized.solidity.contains("FHE.add("));
    assert!(!optimized.solidity.contains("FHE.sub("));
    assert_eq!(optimized.solidity.matches("return a;").count(), 3);
    assert_eq!(optimized.report.eliminated_encrypted_operations, 3);
    assert_eq!(optimized.report.reduction_percent, 100);
}

#[test]
fn multiplication_one_is_folded_but_zero_is_not() {
    let source = "fn one(a: encrypted<u32>) -> encrypted<u32> { return a * 1; } fn two(a: encrypted<u32>) -> encrypted<u32> { return 1 * a; } fn three(a: encrypted<u32>) -> encrypted<u32> { return a * 0; }";
    let optimized = compile(source, true).unwrap();
    assert_eq!(optimized.solidity.matches("return a;").count(), 2);
    assert!(optimized.solidity.contains("FHE.mul(a, FHE.asEuint32(0))"));
    assert_eq!(optimized.report.eliminated_encrypted_operations, 2);
}

#[test]
fn identical_selection_branches_with_variable_predicate_are_folded() {
    let source = "fn same(flag: encrypted<bool>, a: encrypted<u8>) -> encrypted<u8> { return if flag { a } else { a }; }";
    let baseline = compile(source, false).unwrap();
    let optimized = compile(source, true).unwrap();
    assert!(baseline.solidity.contains("FHE.select(flag, a, a)"));
    assert!(optimized.solidity.contains("return a;"));
    assert_eq!(optimized.report.eliminated_encrypted_operations, 1);
}

#[test]
fn identical_selection_branches_with_computed_predicate_are_kept() {
    let source = "fn same(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { a } else { a }; }";
    let optimized = compile(source, true).unwrap();
    assert!(
        optimized
            .solidity
            .contains("FHE.select(FHE.gt(a, b), a, a)")
    );
    assert_eq!(optimized.report.eliminated_encrypted_operations, 0);
}

#[test]
fn identical_encrypted_operations_are_shared_across_lets() {
    let source = "fn repeated(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { let first = a * b; let second = a * b; return first + second; }";
    let baseline = compile(source, false).unwrap();
    let optimized = compile(source, true).unwrap();
    assert_eq!(baseline.solidity.matches("FHE.mul(").count(), 2);
    assert_eq!(optimized.solidity.matches("FHE.mul(").count(), 1);
    assert!(optimized.solidity.contains("euint8 second = first;"));
    assert_eq!(
        optimized.report.rules_applied["common_subexpression_binding"],
        1
    );
    assert_eq!(optimized.report.eliminated_encrypted_operations, 1);
    assert_eq!(optimized.report.baseline_totals.encrypted_operations, 3);
    assert_eq!(optimized.report.optimized_totals.encrypted_operations, 2);
    assert_eq!(optimized.report.reduction_percent, 33);
}

#[test]
fn distinct_operands_do_not_share_common_subexpressions() {
    let source = "fn repeated(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { let first = a * b; let second = b * a; return first + second; }";
    let optimized = compile(source, true).unwrap();
    assert_eq!(optimized.solidity.matches("FHE.mul(").count(), 2);
    assert_eq!(optimized.report.eliminated_encrypted_operations, 0);
}

#[test]
fn function_calls_are_never_eliminated_through_cse() {
    let source = "fn twice(a: encrypted<u8>) -> encrypted<u8> { return a + a; } fn repeated(a: encrypted<u8>) -> encrypted<u8> { let first = twice(a) * a; let second = twice(a) * a; return first + second; }";
    let optimized = compile(source, true).unwrap();
    assert!(
        optimized
            .solidity
            .contains("euint8 second = FHE.mul(twice(a), a);")
    );
    assert_eq!(optimized.solidity.matches("twice(a)").count(), 2);
}

#[test]
fn source_validation_happens_before_optimization() {
    let source = "fn impossible(a: encrypted<u8>) -> encrypted<u8> { return a + 256; }";
    let error = compile(source, true).unwrap_err();
    assert!(error.contains("exceeds encrypted<u8> range"));
}

#[test]
fn legacy_emitter_keeps_original_behavior() {
    let source = "fn unchanged(a: encrypted<u8>) -> encrypted<u8> { return a + 0; }";
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    assert!(
        emit_library(&program)
            .unwrap()
            .contains("FHE.add(a, FHE.asEuint8(0))")
    );
}

#[test]
fn custom_library_name_is_used_and_invalid_names_are_rejected() {
    let source = "fn identity(a: encrypted<u8>) -> encrypted<u8> { return a; }";
    assert!(
        compile(source, true)
            .unwrap()
            .solidity
            .contains("library FHECOptimized")
    );
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let error = compile_cofhe(
        &program,
        &CoFheOptions {
            optimize: true,
            library_name: "contract".to_owned(),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("Reserved Solidity identifier"));
}

#[test]
fn report_says_static_counts_and_json_is_machine_readable() {
    let source = "fn identity(a: encrypted<u8>) -> encrypted<u8> { return a + 0; }";
    let artifact = compile(source, true).unwrap();
    let json = artifact.report.to_pretty_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "fhec-cofhe-static-optimization-v1");
    assert_eq!(
        value["measurement"],
        "static source-level operation counts, not gas or FHE execution time"
    );
    assert_eq!(value["emitted_variant"], "optimized");
    assert_eq!(value["eliminated_encrypted_operations"], 1);
}
