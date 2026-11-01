use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

use fhec_frontend::TypeChecker;
use fhec_frontend::ast::Program;

use super::call_graph::{analyze_call_graph, validate_call_graph};
use super::cost_report::{CoFheCostReport, build_report};
use super::lowering::{Signature, lower_function};
use super::optimizer::optimize_program;
use super::type_map::CoType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoFheError(pub String);

impl fmt::Display for CoFheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CoFheError {}

pub fn emit_library(program: &Program) -> Result<String, CoFheError> {
    Ok(compile_cofhe(program, &CoFheOptions::default())?.solidity)
}

#[derive(Clone, Debug)]
pub struct CoFheOptions {
    pub optimize: bool,
    pub library_name: String,
}

impl Default for CoFheOptions {
    fn default() -> Self {
        Self {
            optimize: false,
            library_name: "FHECGenerated".into(),
        }
    }
}

#[derive(Debug)]
pub struct CoFheArtifact {
    pub solidity: String,
    pub report: CoFheCostReport,
}

pub fn compile_cofhe(
    program: &Program,
    options: &CoFheOptions,
) -> Result<CoFheArtifact, CoFheError> {
    TypeChecker::new()
        .check(program)
        .map_err(|error| CoFheError(format!("Type checking failed: {}", error.message)))?;

    if program.functions.is_empty() {
        return Err(CoFheError("No functions to compile".into()));
    }

    if options.library_name != "FHECGenerated" {
        check_identifier(&options.library_name)?;
    }

    let mut signatures = HashMap::new();
    let mut imports = BTreeSet::new();
    imports.insert("FHE".to_owned());

    for function in &program.functions {
        check_identifier(&function.name)?;
        if function.name == options.library_name {
            return Err(CoFheError(format!(
                "Function name '{}' conflicts with generated library name",
                function.name
            )));
        }
        let mut parameters = Vec::new();
        for parameter in &function.parameters {
            check_identifier(&parameter.name)?;
            let ty = CoType::from_ast(&parameter.ty)?;
            imports.insert(ty.solidity());
            parameters.push(ty);
        }
        let result = CoType::from_ast(&function.return_type)?;
        imports.insert(result.solidity());
        signatures.insert(function.name.clone(), Signature { parameters, result });
    }

    for function in &program.functions {
        lower_function(function, &signatures)?;
    }
    validate_call_graph(program)?;

    let (optimized, stats) = optimize_program(program);
    for function in &optimized.functions {
        lower_function(function, &signatures)?;
    }
    let call_graph = analyze_call_graph(program, &optimized)?;

    let report = build_report(program, &optimized, stats, options.optimize, call_graph);
    imports.insert("ebool".to_owned());
    let mut solidity = format!(
        "// SPDX-License-Identifier: Apache-2.0\npragma solidity ^0.8.25;\n\nimport {{{}}} from \"@fhenixprotocol/cofhe-contracts/FHE.sol\";\n\nlibrary {} {{\n",
        imports.into_iter().collect::<Vec<_>>().join(", "),
        options.library_name
    );

    let selected = if options.optimize {
        &optimized
    } else {
        program
    };
    for function in &selected.functions {
        solidity.push_str(&lower_function(function, &signatures)?);
    }

    solidity.push_str("}\n");
    Ok(CoFheArtifact { solidity, report })
}

pub(crate) fn check_identifier(value: &str) -> Result<(), CoFheError> {
    let mut chars = value.chars();
    if !matches!(chars.next(), Some('a'..='z' | 'A'..='Z' | '_'))
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        || value.starts_with("__")
    {
        return Err(CoFheError(format!("Invalid Solidity identifier: {value}")));
    }

    const RESERVED: &[&str] = &[
        "abstract",
        "after",
        "alias",
        "apply",
        "assembly",
        "auto",
        "bool",
        "break",
        "byte",
        "bytes",
        "case",
        "catch",
        "constant",
        "constructor",
        "continue",
        "contract",
        "copyof",
        "days",
        "default",
        "define",
        "delete",
        "do",
        "else",
        "emit",
        "enum",
        "error",
        "event",
        "external",
        "false",
        "final",
        "fixed",
        "for",
        "from",
        "function",
        "global",
        "hours",
        "if",
        "immutable",
        "import",
        "in",
        "indexed",
        "inline",
        "int",
        "interface",
        "internal",
        "is",
        "let",
        "library",
        "mapping",
        "match",
        "memory",
        "minutes",
        "modifier",
        "mutable",
        "new",
        "null",
        "of",
        "override",
        "partial",
        "payable",
        "pragma",
        "private",
        "promise",
        "public",
        "pure",
        "reference",
        "relocatable",
        "return",
        "returns",
        "revert",
        "seconds",
        "sizeof",
        "static",
        "storage",
        "string",
        "struct",
        "supports",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "type",
        "typedef",
        "typeof",
        "ufixed",
        "uint",
        "unchecked",
        "using",
        "var",
        "view",
        "virtual",
        "weeks",
        "while",
        "years",
    ];

    if RESERVED.contains(&value) {
        return Err(CoFheError(format!("Reserved Solidity identifier: {value}")));
    }
    if value == "FHE"
        || value == "FHECGenerated"
        || value == "ebool"
        || [8, 16, 32, 64]
            .iter()
            .any(|width| value == format!("euint{width}"))
    {
        return Err(CoFheError(format!(
            "CoFHE import identifier cannot be shadowed: {value}"
        )));
    }
    Ok(())
}
