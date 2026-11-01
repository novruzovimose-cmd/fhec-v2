use crate::circuit_lowering::{AdderStrategy, lower_function_to_circuit_with_adder};
use crate::lowering::lower_program;
use fhec_frontend::{lexer::Lexer, parser::Parser};

fn run_cases(
    ty: &str,
    op: &str,
    pairs: &[(u8, u8)],
    expected: impl Fn(u8, u8) -> u8,
    boolean: bool,
) {
    let output_ty = if boolean { "bool" } else { ty };

    let source = format!(
        "fn check(a: encrypted<{ty}>, b: encrypted<{ty}>) -> encrypted<{output_ty}> {{ return a {op} b; }}"
    );

    let tokens = Lexer::new(&source).tokenize().expect("lexer");
    let program = Parser::new(tokens).parse().expect("parser");
    let module = lower_program(&program).expect("IR lowering");
    let function = &module.functions[0];

    for strategy in [AdderStrategy::ParallelPrefix, AdderStrategy::RippleCarry] {
        let compiled = lower_function_to_circuit_with_adder(function, strategy);

        compiled.verify(function).expect("circuit verification");

        for &(a, b) in pairs {
            let mut input = (0..8).map(|n| ((a >> n) & 1) != 0).collect::<Vec<_>>();

            input.extend((0..8).map(|n| ((b >> n) & 1) != 0));

            let actual = compiled
                .circuit
                .evaluate(&input, &compiled.outputs)
                .expect("evaluation")
                .iter()
                .enumerate()
                .fold(0u8, |bits, (n, bit)| bits | (u8::from(*bit) << n));

            assert_eq!(
                actual,
                expected(a, b),
                "type={ty}, op={op}, a={a}, b={b}, strategy={strategy:?}"
            );
        }
    }
}

fn inputs() -> Vec<(u8, u8)> {
    let edge = [0u8, 1, 2, 7, 15, 16, 63, 64, 127, 128, 129, 254, 255];

    let mut pairs = Vec::new();

    for a in edge {
        for b in edge {
            pairs.push((a, b));
        }
    }

    let mut state = 0xA176_2C31u32;

    for _ in 0..256 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;

        let a = state as u8;

        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;

        pairs.push((a, state as u8));
    }

    pairs
}

#[test]
fn differential_unsigned_u8_arithmetic() {
    let pairs = inputs();

    run_cases("u8", "+", &pairs, u8::wrapping_add, false);
    run_cases("u8", "-", &pairs, u8::wrapping_sub, false);
    run_cases("u8", "*", &pairs, u8::wrapping_mul, false);
}

#[test]
fn differential_signed_i8_arithmetic() {
    let pairs = inputs();

    run_cases(
        "i8",
        "+",
        &pairs,
        |a, b| (a as i8).wrapping_add(b as i8) as u8,
        false,
    );

    run_cases(
        "i8",
        "-",
        &pairs,
        |a, b| (a as i8).wrapping_sub(b as i8) as u8,
        false,
    );

    run_cases(
        "i8",
        "*",
        &pairs,
        |a, b| (a as i8).wrapping_mul(b as i8) as u8,
        false,
    );
}

#[test]
fn differential_unsigned_comparisons() {
    let pairs = inputs();

    run_cases("u8", "==", &pairs, |a, b| u8::from(a == b), true);
    run_cases("u8", "!=", &pairs, |a, b| u8::from(a != b), true);
    run_cases("u8", "<", &pairs, |a, b| u8::from(a < b), true);
    run_cases("u8", "<=", &pairs, |a, b| u8::from(a <= b), true);
    run_cases("u8", ">", &pairs, |a, b| u8::from(a > b), true);
    run_cases("u8", ">=", &pairs, |a, b| u8::from(a >= b), true);
}

#[test]
fn differential_signed_comparisons() {
    let pairs = inputs();

    run_cases("i8", "==", &pairs, |a, b| u8::from(a == b), true);
    run_cases("i8", "!=", &pairs, |a, b| u8::from(a != b), true);

    run_cases(
        "i8",
        "<",
        &pairs,
        |a, b| u8::from((a as i8) < (b as i8)),
        true,
    );

    run_cases(
        "i8",
        "<=",
        &pairs,
        |a, b| u8::from((a as i8) <= (b as i8)),
        true,
    );

    run_cases(
        "i8",
        ">",
        &pairs,
        |a, b| u8::from((a as i8) > (b as i8)),
        true,
    );

    run_cases(
        "i8",
        ">=",
        &pairs,
        |a, b| u8::from((a as i8) >= (b as i8)),
        true,
    );
}
