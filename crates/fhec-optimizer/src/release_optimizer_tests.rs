use super::{boolean_optimize, fold_add, fold_mul, fold_sub, mux_optimize, optimize_function};
use fhec_ir::{Constant, Function, Operation, Type, Value, ValueId, verify_function};

#[test]
fn unsigned_byte_folding_matches_all_wrapping_inputs() {
    for a in 0u64..=255 {
        for b in 0u64..=255 {
            let lhs = Constant::UInt { value: a, width: 8 };
            let rhs = Constant::UInt { value: b, width: 8 };
            for (actual, expected) in [
                (fold_add(Some(&lhs), Some(&rhs)), (a + b) & 255),
                (fold_sub(Some(&lhs), Some(&rhs)), a.wrapping_sub(b) & 255),
                (fold_mul(Some(&lhs), Some(&rhs)), (a * b) & 255),
            ] {
                assert_eq!(
                    actual,
                    Some(Constant::UInt {
                        value: expected,
                        width: 8
                    }),
                    "a={a}, b={b}"
                );
            }
        }
    }
}

#[test]
fn signed_byte_folding_matches_all_wrapping_inputs() {
    for a in i8::MIN..=i8::MAX {
        for b in i8::MIN..=i8::MAX {
            let lhs = Constant::Int {
                value: i64::from(a),
                width: 8,
            };
            let rhs = Constant::Int {
                value: i64::from(b),
                width: 8,
            };
            for (actual, expected) in [
                (fold_add(Some(&lhs), Some(&rhs)), a.wrapping_add(b)),
                (fold_sub(Some(&lhs), Some(&rhs)), a.wrapping_sub(b)),
                (fold_mul(Some(&lhs), Some(&rhs)), a.wrapping_mul(b)),
            ] {
                assert_eq!(
                    actual,
                    Some(Constant::Int {
                        value: i64::from(expected),
                        width: 8
                    }),
                    "a={a}, b={b}"
                );
            }
        }
    }
}

#[test]
fn integer_folding_respects_sixteen_and_sixty_four_bit_boundaries() {
    let u16_max = Constant::UInt {
        value: 65535,
        width: 16,
    };
    let u16_one = Constant::UInt {
        value: 1,
        width: 16,
    };
    assert_eq!(
        fold_add(Some(&u16_max), Some(&u16_one)),
        Some(Constant::UInt {
            value: 0,
            width: 16
        }),
    );

    let i16_max = Constant::Int {
        value: 32767,
        width: 16,
    };
    let i16_one = Constant::Int {
        value: 1,
        width: 16,
    };
    assert_eq!(
        fold_add(Some(&i16_max), Some(&i16_one)),
        Some(Constant::Int {
            value: -32768,
            width: 16
        }),
    );

    let u64_max = Constant::UInt {
        value: u64::MAX,
        width: 64,
    };
    let u64_one = Constant::UInt {
        value: 1,
        width: 64,
    };
    assert_eq!(
        fold_add(Some(&u64_max), Some(&u64_one)),
        Some(Constant::UInt {
            value: 0,
            width: 64
        }),
    );

    let i64_max = Constant::Int {
        value: i64::MAX,
        width: 64,
    };
    let i64_one = Constant::Int {
        value: 1,
        width: 64,
    };
    assert_eq!(
        fold_add(Some(&i64_max), Some(&i64_one)),
        Some(Constant::Int {
            value: i64::MIN,
            width: 64
        }),
    );
}

#[test]
fn optimized_integer_ir_keeps_declared_width() {
    let mut function = Function::new("wrap", Vec::new(), Type::uint(8));
    let lhs = function.create_value(Type::uint(8));
    let rhs = function.create_value(Type::uint(8));
    let result = function.create_value(Type::uint(8));

    function.push_operation(Operation::Constant {
        result: lhs.id,
        value: Constant::UInt {
            value: 255,
            width: 8,
        },
    });
    function.push_operation(Operation::Constant {
        result: rhs.id,
        value: Constant::UInt { value: 1, width: 8 },
    });
    function.push_operation(Operation::Add {
        result: result.id,
        lhs: lhs.id,
        rhs: rhs.id,
    });
    function.push_operation(Operation::Return {
        value: Some(result.id),
    });

    optimize_function(&mut function);
    verify_function(&function).unwrap();
    assert!(matches!(
        &function.operations[2],
        Operation::Constant { result: id, value: Constant::UInt { value: 0, width: 8 } }
            if *id == result.id
    ));
}

#[test]
fn boolean_simplification_does_not_read_later_constant_definitions() {
    let mut function = Function::new(
        "invalid_forward_boolean",
        vec![Value::new(ValueId(0), Type::Bool)],
        Type::Bool,
    );
    let late = function.create_value(Type::Bool);
    let result = function.create_value(Type::Bool);
    function.push_operation(Operation::And {
        result: result.id,
        lhs: ValueId(0),
        rhs: late.id,
    });
    function.push_operation(Operation::Constant {
        result: late.id,
        value: Constant::Bool(true),
    });
    assert!(verify_function(&function).is_err());
    let original = function.operations.clone();
    boolean_optimize(&mut function);
    assert_eq!(function.operations, original);
}

#[test]
fn double_not_simplification_does_not_read_later_definition() {
    let mut function = Function::new(
        "invalid_forward_not",
        vec![Value::new(ValueId(0), Type::Bool)],
        Type::Bool,
    );
    let late = function.create_value(Type::Bool);
    let result = function.create_value(Type::Bool);
    function.push_operation(Operation::Not {
        result: result.id,
        operand: late.id,
    });
    function.push_operation(Operation::Not {
        result: late.id,
        operand: ValueId(0),
    });
    assert!(verify_function(&function).is_err());
    let original = function.operations.clone();
    boolean_optimize(&mut function);
    assert_eq!(function.operations, original);
}

#[test]
fn mux_simplification_does_not_read_later_condition_definition() {
    let mut function = Function::new(
        "invalid_forward_mux",
        vec![
            Value::new(ValueId(0), Type::Bool),
            Value::new(ValueId(1), Type::Bool),
        ],
        Type::Bool,
    );
    let late = function.create_value(Type::Bool);
    let result = function.create_value(Type::Bool);
    function.push_operation(Operation::Mux {
        result: result.id,
        condition: late.id,
        if_true: ValueId(0),
        if_false: ValueId(1),
    });
    function.push_operation(Operation::Constant {
        result: late.id,
        value: Constant::Bool(true),
    });
    assert!(verify_function(&function).is_err());
    let original = function.operations.clone();
    mux_optimize(&mut function);
    assert_eq!(function.operations, original);
}
