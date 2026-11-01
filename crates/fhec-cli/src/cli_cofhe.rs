use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use fhec_backend_cofhe::{CoFheOptions, compile_cofhe};
use fhec_frontend::{Lexer, Parser};

pub fn generate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let usage = "Usage: fhec cofhe <source.fhe> --out <output.sol> [--optimize] [--report <report.json>] [--library <name>]";
    let Some(source) = args.first() else {
        return Err(usage.into());
    };
    if source.starts_with("--") {
        return Err(usage.into());
    }

    let mut output: Option<&str> = None;
    let mut report: Option<&str> = None;
    let mut optimize = false;
    let mut library: Option<&str> = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--out" if output.is_none() && index + 1 < args.len() => {
                output = Some(args[index + 1].as_str());
                index += 2;
            }
            "--report" if report.is_none() && index + 1 < args.len() => {
                report = Some(args[index + 1].as_str());
                index += 2;
            }
            "--library" if library.is_none() && index + 1 < args.len() => {
                library = Some(args[index + 1].as_str());
                index += 2;
            }
            "--optimize" if !optimize => {
                optimize = true;
                index += 1;
            }
            _ => return Err(usage.into()),
        }
    }

    let Some(output) = output.filter(|value| !value.is_empty() && !value.starts_with("--")) else {
        return Err(usage.into());
    };
    if report.is_some_and(|value| value.is_empty() || value.starts_with("--"))
        || library.is_some_and(|value| value.is_empty() || value.starts_with("--"))
    {
        return Err(usage.into());
    }

    let source_path = Path::new(source);
    let output_path = Path::new(output);
    let report_path = report.map(Path::new);

    if !source_path.is_file() {
        return Err(format!("Source file not found: {}", source_path.display()).into());
    }

    for path in [Some(output_path), report_path].into_iter().flatten() {
        if path.exists() {
            return Err(format!(
                "Output file already exists; refusing to overwrite: {}",
                path.display()
            )
            .into());
        }
        if path == source_path {
            return Err("Generated output must differ from the source file".into());
        }
    }
    if report_path == Some(output_path) {
        return Err("Solidity and cost report must have distinct paths".into());
    }

    let source = fs::read_to_string(source_path)?;
    let tokens = Lexer::new(&source)
        .tokenize()
        .map_err(|error| format!("Lexing failed: {error:?}"))?;
    let program = Parser::new(tokens)
        .parse()
        .map_err(|error| format!("Parsing failed: {error:?}"))?;
    let options = CoFheOptions {
        optimize,
        library_name: library.unwrap_or("FHECGenerated").to_owned(),
    };
    let artifact = compile_cofhe(&program, &options)?;
    let report_json = if report_path.is_some() {
        Some(artifact.report.to_pretty_json()?)
    } else {
        None
    };

    let mut created: Vec<PathBuf> = Vec::new();
    let written = (|| -> Result<(), Box<dyn Error>> {
        let mut solidity_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output_path)?;
        created.push(output_path.to_path_buf());
        solidity_file.write_all(artifact.solidity.as_bytes())?;
        solidity_file.sync_all()?;

        if let (Some(path), Some(json)) = (report_path, report_json.as_ref()) {
            let mut cost_file = OpenOptions::new().write(true).create_new(true).open(path)?;
            created.push(path.to_path_buf());
            cost_file.write_all(json.as_bytes())?;
            cost_file.sync_all()?;
        }
        Ok(())
    })();
    if let Err(error) = written {
        for file in created {
            fs::remove_file(&file).map_err(|cleanup| {
                format!(
                    "Output failed: {error}; failed to clean up {}: {cleanup}",
                    file.display()
                )
            })?;
        }
        return Err(error);
    }

    println!("Generated CoFHE library: {}", output_path.display());
    if let Some(path) = report_path {
        println!("Static CoFHE optimization report: {}", path.display());
    }
    Ok(())
}
