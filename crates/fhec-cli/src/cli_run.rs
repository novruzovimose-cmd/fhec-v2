use std::error::Error;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

use fhec_backend::FheBackend;
use fhec_backend_tfhe::TfheBackend;
use fhec_compiler::Compiler;
use fhec_frontend::ast::Type;
use fhec_frontend::{Lexer, Parser};
use fhec_runtime::v4::execute_auto_v4;

#[derive(Clone, Copy)]
enum OutputKind {
    U8,
    U16,
    I8,
    Bool,
}

impl OutputKind {
    fn width(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 8,
            Self::U16 => 16,
            Self::Bool => 1,
        }
    }
}

const MAX_STDIN_INPUT_BYTES: u64 = 4096;

enum InputSource {
    Argument(Vec<String>),
    Stdin,
}

fn parse_input_values(raw: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let values = raw
        .split(',')
        .map(|value| value.trim().to_string())
        .collect::<Vec<_>>();

    if values.iter().any(String::is_empty) {
        return Err("Input values cannot be empty".into());
    }

    Ok(values)
}

fn read_private_inputs<R: BufRead>(reader: R) -> Result<Vec<String>, Box<dyn Error>> {
    let mut limited = reader.take(MAX_STDIN_INPUT_BYTES + 1);
    let mut line = String::new();
    let size = limited.read_line(&mut line)?;

    if size == 0 {
        return Err("No input values supplied on stdin".into());
    }
    if size as u64 > MAX_STDIN_INPUT_BYTES {
        return Err("Input line too long (maximum 4096 bytes)".into());
    }

    parse_input_values(line.trim_end_matches(['\r', '\n']))
}

fn input_summary(count: usize) -> String {
    format!("Encrypted input parameters: {count} (values hidden)")
}

fn encode_bits(value: u16, width: usize) -> Vec<bool> {
    (0..width)
        .map(|index| ((value >> index) & 1) != 0)
        .collect()
}

fn encode_input(ty: &Type, raw: &str) -> Result<Vec<bool>, Box<dyn Error>> {
    let Type::Encrypted(inner) = ty else {
        return Err(format!("Expected encrypted parameter, got {ty:?}").into());
    };

    match inner.as_ref() {
        Type::UInt(8) => {
            let value: u8 = raw
                .parse()
                .map_err(|_| "Invalid encrypted<u8> value. Expected 0..255")?;
            Ok(encode_bits(u16::from(value), 8))
        }
        Type::UInt(16) => {
            let value: u16 = raw
                .parse()
                .map_err(|_| "Invalid encrypted<u16> value. Expected 0..65535")?;
            Ok(encode_bits(value, 16))
        }
        Type::Int(8) => {
            let value: i8 = raw
                .parse()
                .map_err(|_| "Invalid encrypted<i8> value. Expected -128..127")?;
            Ok(encode_bits(u16::from(value as u8), 8))
        }
        Type::Bool => {
            let value: bool = raw
                .parse()
                .map_err(|_| "Invalid encrypted<bool> value. Expected true or false")?;
            Ok(vec![value])
        }
        other => Err(format!("Unsupported encrypted input type: {other:?}").into()),
    }
}

fn output_kind(ty: &Type) -> Result<OutputKind, Box<dyn Error>> {
    let Type::Encrypted(inner) = ty else {
        return Err(format!("Expected encrypted return type, got {ty:?}").into());
    };

    match inner.as_ref() {
        Type::UInt(8) => Ok(OutputKind::U8),
        Type::UInt(16) => Ok(OutputKind::U16),
        Type::Int(8) => Ok(OutputKind::I8),
        Type::Bool => Ok(OutputKind::Bool),
        other => Err(format!("Unsupported encrypted return type: {other:?}").into()),
    }
}

fn decode_output(bits: &[bool], kind: OutputKind) -> Result<String, Box<dyn Error>> {
    if bits.len() != kind.width() {
        return Err(format!(
            "Output width mismatch: expected {}, received {}",
            kind.width(),
            bits.len()
        )
        .into());
    }

    let value = bits.iter().enumerate().fold(
        0u16,
        |acc, (index, bit)| {
            if *bit { acc | (1u16 << index) } else { acc }
        },
    );

    let decoded = match kind {
        OutputKind::U8 => (value as u8).to_string(),
        OutputKind::U16 => value.to_string(),
        OutputKind::I8 => (value as u8 as i8).to_string(),
        OutputKind::Bool => (value != 0).to_string(),
    };

    Ok(decoded)
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 2 {
        return Err(
            "Usage: fhec run <file.fhe> --function <name> (--inputs <a,b> | --inputs-stdin)".into(),
        );
    }

    let path = &args[1];

    if !Path::new(path).is_file() {
        return Err(format!("Source file not found: {path}").into());
    }

    let mut function_name: Option<String> = None;
    let mut input_source: Option<InputSource> = None;

    let mut index = 2;

    while index < args.len() {
        match args[index].as_str() {
            "--function" => {
                if function_name.is_some() {
                    return Err("Duplicate --function argument".into());
                }

                index += 1;
                function_name = Some(args.get(index).ok_or("Missing function name")?.clone());
            }
            "--inputs" => {
                if input_source.is_some() {
                    return Err("Only one input source may be supplied".into());
                }
                index += 1;
                let raw = args.get(index).ok_or("Missing input values")?;
                input_source = Some(InputSource::Argument(parse_input_values(raw)?));
            }
            "--inputs-stdin" => {
                if input_source.is_some() {
                    return Err("Only one input source may be supplied".into());
                }
                input_source = Some(InputSource::Stdin);
            }
            _ => {
                return Err(
                    "Unknown run argument (use --function, --inputs, or --inputs-stdin)".into(),
                );
            }
        }

        index += 1;
    }

    let function_name = function_name.ok_or("Missing --function <name>")?;
    let input_source = input_source.ok_or("Missing --inputs <a,b> or --inputs-stdin")?;
    let input_values = match input_source {
        InputSource::Argument(values) => values,
        InputSource::Stdin => read_private_inputs(io::stdin().lock())?,
    };

    let source = fs::read_to_string(path)?;

    let tokens = Lexer::new(&source)
        .tokenize()
        .map_err(|error| format!("Lexing failed: {error:?}"))?;

    let program = Parser::new(tokens)
        .parse()
        .map_err(|error| format!("Parsing failed: {error:?}"))?;

    let ast_function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .ok_or_else(|| format!("Unknown function: {function_name}"))?;

    if ast_function.parameters.len() != input_values.len() {
        return Err(format!(
            "Function expects {} inputs, received {}",
            ast_function.parameters.len(),
            input_values.len()
        )
        .into());
    }

    let result_kind = output_kind(&ast_function.return_type)?;

    let mut plaintext_bits = Vec::new();

    for (parameter, raw) in ast_function.parameters.iter().zip(&input_values) {
        let bits = encode_input(&parameter.ty, raw)
            .map_err(|error| format!("Invalid input for parameter {}: {error}", parameter.name))?;

        plaintext_bits.extend(bits);
    }

    let compiler = Compiler::new();

    let module = compiler
        .lower(&program)
        .map_err(|error| format!("Lowering failed: {error:?}"))?;

    let function = module
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .ok_or("Compiled function not found")?;

    let circuit = compiler
        .try_lower_to_circuit(function)
        .map_err(|error| format!("Circuit lowering failed for {}: {error:?}", function.name))?;

    circuit
        .verify(function)
        .map_err(|error| format!("Circuit verification failed: {error:?}"))?;

    let circuit_input_bits = circuit
        .inputs
        .values()
        .map(|value| value.bits.len())
        .sum::<usize>();

    if circuit_input_bits != plaintext_bits.len() {
        return Err(format!(
            "Circuit input width mismatch: expected {} bits, encoded {} bits",
            circuit_input_bits,
            plaintext_bits.len()
        )
        .into());
    }

    if circuit.outputs.len() != result_kind.width() {
        return Err(format!(
            "Circuit output width mismatch: expected {} bits, got {}",
            result_kind.width(),
            circuit.outputs.len()
        )
        .into());
    }

    println!("FHEC Encrypted Execution");
    println!("Function: {function_name}");
    println!("{}", input_summary(input_values.len()));
    println!("Encrypting inputs...");

    let backend = TfheBackend::new();

    let encrypted_inputs = plaintext_bits
        .into_iter()
        .map(|bit| backend.encrypt_bool(bit))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Encryption failed: {error:?}"))?;

    let profile_path = Path::new("target/fhec-profiles/tfhe-v4.json");

    if let Some(directory) = profile_path.parent() {
        fs::create_dir_all(directory)?;
    }

    let resolved = backend
        .load_or_calibrate_v4_profile(profile_path, 2)
        .map_err(|error| format!("Hardware profile resolution failed: {error}"))?;

    println!("Hardware profile: {}", resolved.status_name());
    println!(
        "Calibration reused: {}",
        if resolved.calibration_reused() {
            "YES"
        } else {
            "NO"
        }
    );

    let model = resolved.model;

    let (encrypted_outputs, decision) = execute_auto_v4(
        &backend,
        &circuit.circuit,
        &encrypted_inputs,
        &circuit.outputs,
        &model,
    )
    .map_err(|error| format!("TFHE execution failed: {error:?}"))?;

    let decrypted = encrypted_outputs
        .iter()
        .map(|ciphertext| backend.decrypt_bool(ciphertext))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Decryption failed: {error:?}"))?;

    println!("Execution strategy: {:?}", decision.strategy);
    println!(
        "Decrypted result: {}",
        decode_output(&decrypted, result_kind)?
    );
    println!("Encrypted execution successful.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_input_errors_never_echo_plaintext_values() {
        let secret = "private_input_marker_98171";
        let types = [
            Type::Encrypted(Box::new(Type::UInt(8))),
            Type::Encrypted(Box::new(Type::UInt(16))),
            Type::Encrypted(Box::new(Type::Int(8))),
            Type::Encrypted(Box::new(Type::Bool)),
        ];

        for ty in &types {
            let error = encode_input(ty, secret).unwrap_err().to_string();
            assert!(!error.contains(secret), "sensitive value in diagnostic");
        }

        assert!(!input_summary(2).contains(secret));
        assert!(!input_summary(2).contains("37"));
    }

    #[test]
    fn stdin_input_reader_accepts_one_line_and_rejects_oversized_input() {
        use std::io::Cursor;

        let data = Cursor::new(b"37,5\r\n");
        assert_eq!(read_private_inputs(data).unwrap(), vec!["37", "5"]);

        let too_large = Cursor::new(vec![b'7'; 4097]);
        assert!(read_private_inputs(too_large).is_err());

        let empty = Cursor::new(Vec::<u8>::new());
        assert!(read_private_inputs(empty).is_err());

        let incomplete = Cursor::new(b"37,,5\n");
        assert!(read_private_inputs(incomplete).is_err());
    }

    #[test]
    fn unsigned_u16_encoding() {
        let ty = Type::Encrypted(Box::new(Type::UInt(16)));
        let bits = encode_input(&ty, "50000").unwrap();

        assert_eq!(bits.len(), 16);
        assert_eq!(decode_output(&bits, OutputKind::U16).unwrap(), "50000");
    }

    #[test]
    fn signed_i8_encoding() {
        let ty = Type::Encrypted(Box::new(Type::Int(8)));
        let bits = encode_input(&ty, "-42").unwrap();

        assert_eq!(bits.len(), 8);
        assert_eq!(decode_output(&bits, OutputKind::I8).unwrap(), "-42");
    }

    #[test]
    fn boolean_encoding() {
        let ty = Type::Encrypted(Box::new(Type::Bool));
        let bits = encode_input(&ty, "true").unwrap();

        assert_eq!(bits, vec![true]);
        assert_eq!(decode_output(&bits, OutputKind::Bool).unwrap(), "true");
    }

    #[test]
    fn rejects_out_of_range_inputs() {
        let u8_ty = Type::Encrypted(Box::new(Type::UInt(8)));
        let i8_ty = Type::Encrypted(Box::new(Type::Int(8)));

        assert!(encode_input(&u8_ty, "256").is_err());
        assert!(encode_input(&i8_ty, "-129").is_err());
        assert!(encode_input(&i8_ty, "128").is_err());
    }

    #[test]
    fn rejects_invalid_boolean() {
        let ty = Type::Encrypted(Box::new(Type::Bool));

        assert!(encode_input(&ty, "2").is_err());
        assert!(encode_input(&ty, "yes").is_err());
    }

    #[test]
    fn mixed_width_inputs_preserve_bit_order() {
        let types = [
            (Type::Encrypted(Box::new(Type::UInt(8))), "37"),
            (Type::Encrypted(Box::new(Type::Bool)), "true"),
            (Type::Encrypted(Box::new(Type::Int(8))), "-7"),
            (Type::Encrypted(Box::new(Type::UInt(16))), "50000"),
        ];

        let packed = types
            .iter()
            .flat_map(|(ty, raw)| encode_input(ty, raw).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(packed.len(), 33);
        assert_eq!(decode_output(&packed[0..8], OutputKind::U8).unwrap(), "37");
        assert_eq!(
            decode_output(&packed[8..9], OutputKind::Bool).unwrap(),
            "true"
        );
        assert_eq!(decode_output(&packed[9..17], OutputKind::I8).unwrap(), "-7");
        assert_eq!(
            decode_output(&packed[17..33], OutputKind::U16).unwrap(),
            "50000"
        );
    }

    #[test]
    fn unsigned_boundaries_are_exact() {
        let u8_ty = Type::Encrypted(Box::new(Type::UInt(8)));
        let u16_ty = Type::Encrypted(Box::new(Type::UInt(16)));

        for value in ["0", "255"] {
            let bits = encode_input(&u8_ty, value).unwrap();
            assert_eq!(decode_output(&bits, OutputKind::U8).unwrap(), value);
        }

        for value in ["0", "65535"] {
            let bits = encode_input(&u16_ty, value).unwrap();
            assert_eq!(decode_output(&bits, OutputKind::U16).unwrap(), value);
        }

        assert!(encode_input(&u8_ty, "256").is_err());
        assert!(encode_input(&u16_ty, "65536").is_err());
        assert!(encode_input(&u8_ty, "-1").is_err());
    }

    #[test]
    fn signed_boundaries_are_exact() {
        let ty = Type::Encrypted(Box::new(Type::Int(8)));

        for value in ["-128", "-1", "0", "1", "127"] {
            let bits = encode_input(&ty, value).unwrap();
            assert_eq!(decode_output(&bits, OutputKind::I8).unwrap(), value);
        }

        assert!(encode_input(&ty, "-129").is_err());
        assert!(encode_input(&ty, "128").is_err());
    }

    #[test]
    fn boolean_values_are_exact() {
        let ty = Type::Encrypted(Box::new(Type::Bool));

        for value in ["true", "false"] {
            let bits = encode_input(&ty, value).unwrap();
            assert_eq!(decode_output(&bits, OutputKind::Bool).unwrap(), value);
        }

        assert!(encode_input(&ty, "1").is_err());
        assert!(encode_input(&ty, "TRUE").is_err());
    }

    #[test]
    fn rejects_mismatched_output_widths() {
        assert!(decode_output(&[true], OutputKind::U8).is_err());
        assert!(decode_output(&[false; 8], OutputKind::U16).is_err());
        assert!(decode_output(&[true, false], OutputKind::Bool).is_err());
    }

    #[test]
    fn unsigned_encoding_uses_little_endian_bits() {
        let ty = Type::Encrypted(Box::new(Type::UInt(8)));
        let bits = encode_input(&ty, "5").unwrap();

        assert_eq!(
            bits,
            vec![true, false, true, false, false, false, false, false]
        );
    }
    #[test]
    fn rejects_plaintext_parameters() {
        assert!(encode_input(&Type::UInt(8), "42").is_err());
    }
}
