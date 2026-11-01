use fhec_frontend::{Lexer, Parser};

use super::{CoFheOptions, compile_cofhe};

fn compile(source: &str) -> Result<super::CoFheArtifact, String> {
    let tokens = Lexer::new(source)
        .tokenize()
        .map_err(|error| format!("{error:?}"))?;
    let program = Parser::new(tokens)
        .parse()
        .map_err(|error| format!("{error:?}"))?;
    compile_cofhe(
        &program,
        &CoFheOptions {
            optimize: true,
            library_name: "FHECGraphTest".into(),
        },
    )
    .map_err(|error| error.to_string())
}

#[test]
fn helper_expansion_counts_repeated_calls() {
    let source = "fn leaf(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a * b; } fn twice(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return leaf(a, b) + leaf(a, b); }";
    let report = compile(source).unwrap().report;
    let twice = report
        .call_graph
        .iter()
        .find(|item| item.name == "twice")
        .unwrap();
    assert_eq!(twice.baseline.direct_encrypted_operations, 1);
    assert_eq!(twice.baseline.expanded_encrypted_operations, 3);
    assert_eq!(twice.baseline.direct_helper_calls, 2);
    assert_eq!(twice.baseline.expanded_helper_calls, 2);
    assert_eq!(twice.baseline.max_helper_call_depth, 1);
}

#[test]
fn helper_expansion_counts_nested_calls_and_depth() {
    let source = "fn leaf(a: encrypted<u8>) -> encrypted<u8> { return a + a; } fn middle(a: encrypted<u8>) -> encrypted<u8> { return leaf(a) * leaf(a); } fn root(a: encrypted<u8>) -> encrypted<u8> { return middle(a) + leaf(a); }";
    let report = compile(source).unwrap().report;
    let root = report
        .call_graph
        .iter()
        .find(|item| item.name == "root")
        .unwrap();
    assert_eq!(root.baseline.direct_encrypted_operations, 1);
    assert_eq!(root.baseline.expanded_encrypted_operations, 5);
    assert_eq!(root.baseline.direct_helper_calls, 2);
    assert_eq!(root.baseline.expanded_helper_calls, 4);
    assert_eq!(root.baseline.max_helper_call_depth, 2);
}

#[test]
fn encrypted_branch_calls_are_eagerly_counted() {
    let source = "fn plus(a: encrypted<u8>) -> encrypted<u8> { return a + a; } fn choose(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { plus(a) } else { plus(b) }; }";
    let report = compile(source).unwrap().report;
    let choose = report
        .call_graph
        .iter()
        .find(|item| item.name == "choose")
        .unwrap();
    assert_eq!(choose.baseline.direct_encrypted_operations, 2);
    assert_eq!(choose.baseline.expanded_encrypted_operations, 4);
    assert_eq!(choose.baseline.expanded_helper_calls, 2);
}

#[test]
fn optimization_changes_expanded_cost() {
    let source = "fn id(a: encrypted<u8>) -> encrypted<u8> { return a + 0; } fn doubled(a: encrypted<u8>) -> encrypted<u8> { return id(a) * id(a); }";
    let report = compile(source).unwrap().report;
    let doubled = report
        .call_graph
        .iter()
        .find(|item| item.name == "doubled")
        .unwrap();
    assert_eq!(doubled.baseline.expanded_encrypted_operations, 3);
    assert_eq!(doubled.optimized.expanded_encrypted_operations, 1);
    assert_eq!(doubled.eliminated_expanded_operations, 2);
}

#[test]
fn direct_recursion_is_rejected() {
    let source = "fn looped(a: encrypted<u8>) -> encrypted<u8> { return looped(a); }";
    let error = compile(source).unwrap_err();
    assert!(error.contains("Recursive CoFHE helper call is unsupported"));
    assert!(error.contains("looped -> looped"));
}

#[test]
fn indirect_recursion_is_rejected() {
    let source = "fn alpha(a: encrypted<u8>) -> encrypted<u8> { return beta(a); } fn beta(a: encrypted<u8>) -> encrypted<u8> { return gamma(a); } fn gamma(a: encrypted<u8>) -> encrypted<u8> { return alpha(a); }";
    let error = compile(source).unwrap_err();
    assert!(error.contains("Recursive CoFHE helper call is unsupported"));
    assert!(error.contains("alpha -> beta -> gamma -> alpha"));
}

#[test]
fn unknown_helper_is_rejected_before_graph_reporting() {
    let source = "fn bad(a: encrypted<u8>) -> encrypted<u8> { return unknown(a); }";
    assert!(compile(source).is_err());
}

#[test]
fn separate_library_functions_are_reported_individually() {
    let source = "fn first(a: encrypted<u32>) -> encrypted<u32> { return a * a; } fn second(b: encrypted<u64>) -> encrypted<u64> { return b + b; }";
    let report = compile(source).unwrap().report;
    assert_eq!(report.call_graph.len(), 2);
    assert_eq!(
        report.call_graph[0].baseline.expanded_encrypted_operations,
        1
    );
    assert_eq!(
        report.call_graph[1].baseline.expanded_encrypted_operations,
        1
    );
    assert_eq!(report.call_graph[1].baseline.max_helper_call_depth, 0);
}

#[test]
fn expanded_report_serializes_to_json() {
    let source = "fn leaf(a: encrypted<u16>) -> encrypted<u16> { return a + 0; } fn root(a: encrypted<u16>) -> encrypted<u16> { return leaf(a); }";
    let json = compile(source).unwrap().report.to_pretty_json().unwrap();
    let data: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(data["call_graph"][1]["name"], "root");
    assert_eq!(
        data["call_graph"][1]["baseline"]["expanded_encrypted_operations"],
        1
    );
    assert_eq!(
        data["call_graph"][1]["optimized"]["expanded_encrypted_operations"],
        0
    );
}
