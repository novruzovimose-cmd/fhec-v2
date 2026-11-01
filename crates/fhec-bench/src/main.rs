mod compare_benchmark;
mod optimizer_benchmark;
mod tfhe_benchmark;
use std::env;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::time::Instant;

use fhec_compiler::Compiler;
use fhec_frontend::{lexer::Lexer, parser::Parser};

struct ResultRow {
    name: String,
    repetitions: usize,
    compilation_median_ns: u128,
    execution_median_ns: u128,
    result_bits: usize,
}

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        let lower = values[middle - 1];
        lower + (values[middle] - lower) / 2
    } else {
        values[middle]
    }
}

fn run_case(
    name: &str,
    source: &str,
    a: u8,
    b: u8,
    repetitions: usize,
) -> Result<ResultRow, Box<dyn Error>> {
    let mut compilation = Vec::with_capacity(repetitions);
    let mut execution = Vec::with_capacity(repetitions);
    let mut output_width = 0;
    let inputs: Vec<bool> = (0..8)
        .map(|bit| ((a >> bit) & 1) == 1)
        .chain((0..8).map(|bit| ((b >> bit) & 1) == 1))
        .collect();
    for _ in 0..repetitions {
        let start = Instant::now();
        let tokens = Lexer::new(source)
            .tokenize()
            .map_err(|e| format!("{e:?}"))?;
        let program = Parser::new(tokens).parse().map_err(|e| format!("{e:?}"))?;
        let compiler = Compiler::new();
        let module = compiler.lower(&program).map_err(|e| format!("{e:?}"))?;
        let function = module.functions.last().ok_or("missing function")?;
        let circuit = compiler.lower_to_circuit(function);
        circuit.verify(function).map_err(|e| format!("{e:?}"))?;
        compilation.push(start.elapsed().as_nanos());
        let start = Instant::now();
        let output = circuit
            .circuit
            .evaluate(black_box(&inputs), &circuit.outputs)
            .map_err(|e| format!("{e:?}"))?;
        execution.push(start.elapsed().as_nanos());
        output_width = output.len();
        let decoded =
            output.iter().enumerate().fold(
                0u16,
                |acc, (bit, value)| if *value { acc | (1u16 << bit) } else { acc },
            );
        let expected = match name {
            "add_u8" => a.wrapping_add(b),
            "sub_u8" => a.wrapping_sub(b),
            "mul_u8" => a.wrapping_mul(b),
            "eq_u8" => u8::from(a == b),
            "nested_calls_u8" => a.wrapping_add(b).wrapping_add(b),
            "mux_u8" | "nested_mux_u8" => a.max(b),
            "comparison_mux_bool" => u8::from(if a > b { a == b } else { a < b }),
            _ => return Err("unknown benchmark".into()),
        };
        if decoded != expected as u16 {
            return Err(format!("{name}: expected {expected}, got {decoded}").into());
        }
        black_box(output);
    }
    Ok(ResultRow {
        name: name.to_owned(),
        repetitions,
        compilation_median_ns: median(compilation),
        execution_median_ns: median(execution),
        result_bits: output_width,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args
        .windows(2)
        .any(|pair| pair[0] == "--mode" && pair[1] == "compare")
    {
        return compare_benchmark::run(&args);
    }
    if args
        .windows(2)
        .any(|pair| pair[0] == "--mode" && pair[1] == "optimizer")
    {
        return optimizer_benchmark::run(&args);
    }
    if args.iter().any(|arg| arg == "--mode") {
        return tfhe_benchmark::run(&args);
    }
    let mut repetitions = 30usize;
    let mut output_dir = String::from("target/fhec-benchmarks");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--iterations" => {
                i += 1;
                repetitions = args.get(i).ok_or("missing --iterations value")?.parse()?;
            }
            "--output" => {
                i += 1;
                output_dir = args.get(i).ok_or("missing --output value")?.clone();
            }
            _ => return Err(format!("unknown argument: {}", args[i]).into()),
        }
        i += 1;
    }
    if repetitions == 0 || repetitions > 100_000 {
        return Err("iterations must be between 1 and 100000".into());
    }
    let cases = [
        (
            "add_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; }",
        ),
        (
            "sub_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a - b; }",
        ),
        (
            "mul_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a * b; }",
        ),
        (
            "eq_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return a == b; }",
        ),
        (
            "nested_calls_u8",
            "fn add(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return a + b; } fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return add(add(a, b), b); }",
        ),
        (
            "mux_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { a } else { b }; }",
        ),
        (
            "nested_mux_u8",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { if a == b { b } else { a } } else { b }; }",
        ),
        (
            "comparison_mux_bool",
            "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return if a > b { a == b } else { a < b }; }",
        ),
    ];
    let mut rows = Vec::new();
    for (name, source) in cases {
        let row = run_case(name, source, 37, 5, repetitions)?;
        println!(
            "{}: compile median {} ns, clear circuit median {} ns",
            row.name, row.compilation_median_ns, row.execution_median_ns
        );
        rows.push(row);
    }
    fs::create_dir_all(&output_dir)?;
    let mut csv = String::from(
        "name,repetitions,compilation_median_ns,clear_circuit_median_ns,output_bits\n",
    );
    let mut json = String::from("{\"benchmark_kind\":\"clear_circuit_not_tfhe\",\"cases\":[");
    for (index, row) in rows.iter().enumerate() {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            row.name,
            row.repetitions,
            row.compilation_median_ns,
            row.execution_median_ns,
            row.result_bits
        ));
        if index != 0 {
            json.push(',');
        }
        json.push_str(&format!("{{\"name\":\"{}\",\"repetitions\":{},\"compilation_median_ns\":{},\"clear_circuit_median_ns\":{},\"output_bits\":{}}}", row.name, row.repetitions, row.compilation_median_ns, row.execution_median_ns, row.result_bits));
    }
    json.push_str("]}\n");
    fs::write(format!("{output_dir}/results.csv"), csv)?;
    fs::write(format!("{output_dir}/results.json"), json)?;
    println!("Reports written to {output_dir}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{median, run_case};

    #[test]
    fn median_handles_odd_even_and_large_values() {
        assert_eq!(median(vec![7, 1, 3]), 3);
        assert_eq!(median(vec![20, 10]), 15);
        assert_eq!(median(vec![u128::MAX, u128::MAX - 1]), u128::MAX - 1);
    }

    #[test]
    fn encrypted_conditionals_lower_and_evaluate_correctly() {
        let cases = [
            (
                "mux_u8",
                "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { a } else { b }; }",
            ),
            (
                "nested_mux_u8",
                "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<u8> { return if a > b { if a == b { b } else { a } } else { b }; }",
            ),
            (
                "comparison_mux_bool",
                "fn test(a: encrypted<u8>, b: encrypted<u8>) -> encrypted<bool> { return if a > b { a == b } else { a < b }; }",
            ),
        ];

        for (name, source) in cases {
            for (a, b) in [(37, 5), (5, 37), (0, 0), (255, 0), (0, 255)] {
                let row = run_case(name, source, a, b, 1).unwrap();
                let expected_width = if name == "comparison_mux_bool" { 1 } else { 8 };
                assert_eq!(row.result_bits, expected_width);
            }
        }
    }
}
