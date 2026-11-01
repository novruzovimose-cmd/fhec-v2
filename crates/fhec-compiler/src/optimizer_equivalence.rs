#[cfg(test)]
mod tests {
    use fhec_analyzer::hints::{
        HintPriority, OptimizerHint, OptimizerHintKind, OptimizerHintSet, TargetMetric,
    };
    use fhec_ir::{Function, Operation, Type, Value, ValueId};
    use fhec_optimizer::{hints::build_optimization_plan, optimize_function_with_plan};

    use crate::lower_function_to_circuit;

    fn bool_parameters(count: usize) -> Vec<Value> {
        (0..count)
            .map(|index| Value::new(ValueId(index as u32), Type::Bool))
            .collect()
    }

    fn assert_equivalent(original: &Function, optimized: &Function) {
        let original_circuit = lower_function_to_circuit(original);

        let optimized_circuit = lower_function_to_circuit(optimized);

        original_circuit.circuit.verify().unwrap();

        optimized_circuit.circuit.verify().unwrap();

        let input_count = original.parameters.len();

        for mask in 0..(1usize << input_count) {
            let inputs = (0..input_count)
                .map(|bit| ((mask >> bit) & 1) == 1)
                .collect::<Vec<_>>();

            let original_output = original_circuit
                .circuit
                .evaluate(&inputs, &original_circuit.outputs)
                .unwrap();

            let optimized_output = optimized_circuit
                .circuit
                .evaluate(&inputs, &optimized_circuit.outputs)
                .unwrap();

            assert_eq!(
                original_output, optimized_output,
                "optimization changed output for inputs {:?}",
                inputs
            );
        }
    }

    #[test]
    fn depth_and_parallelism_rewrites_are_equivalent() {
        let mut function = Function::new("deep_and", bool_parameters(4), Type::Bool);

        let first = function.create_value(Type::Bool);
        let second = function.create_value(Type::Bool);
        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: first.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::And {
            result: second.id,
            lhs: first.id,
            rhs: ValueId(2),
        });

        function.push_operation(Operation::And {
            result: result.id,
            lhs: second.id,
            rhs: ValueId(3),
        });

        function.push_operation(Operation::Return {
            value: Some(result.id),
        });

        let hints = OptimizerHintSet {
            hints: vec![
                OptimizerHint {
                    kind: OptimizerHintKind::ReduceDepth,
                    priority: HintPriority::High,
                    target: TargetMetric::CircuitDepth,
                    current_value: 3.0,
                    reason: "deep chain".to_string(),
                },
                OptimizerHint {
                    kind: OptimizerHintKind::IncreaseParallelism,
                    priority: HintPriority::High,
                    target: TargetMetric::Parallelism,
                    current_value: 1.0,
                    reason: "low parallelism".to_string(),
                },
            ],
        };

        let plan = build_optimization_plan(&hints);

        let mut optimized = function.clone();

        optimize_function_with_plan(&mut optimized, &plan);

        assert_equivalent(&function, &optimized);
    }

    #[test]
    fn mux_rewrites_are_equivalent() {
        let mut function = Function::new("mux", bool_parameters(1), Type::Bool);

        let true_value = function.create_value(Type::Bool);

        let false_value = function.create_value(Type::Bool);

        let result = function.create_value(Type::Bool);

        function.push_operation(Operation::Constant {
            result: true_value.id,
            value: fhec_ir::Constant::Bool(true),
        });

        function.push_operation(Operation::Constant {
            result: false_value.id,
            value: fhec_ir::Constant::Bool(false),
        });

        function.push_operation(Operation::Mux {
            result: result.id,
            condition: ValueId(0),
            if_true: true_value.id,
            if_false: false_value.id,
        });

        function.push_operation(Operation::Return {
            value: Some(result.id),
        });

        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReplaceExpensiveGates,
                priority: HintPriority::High,
                target: TargetMetric::GateCost,
                current_value: 1.0,
                reason: "mux".to_string(),
            }],
        };

        let plan = build_optimization_plan(&hints);

        let mut optimized = function.clone();

        optimize_function_with_plan(&mut optimized, &plan);

        assert_equivalent(&function, &optimized);
    }

    #[test]
    fn crypto_operation_reduction_is_equivalent() {
        let mut function = Function::new("duplicate_and", bool_parameters(2), Type::Bool);

        let first = function.create_value(Type::Bool);

        let second = function.create_value(Type::Bool);

        function.push_operation(Operation::And {
            result: first.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::And {
            result: second.id,
            lhs: ValueId(0),
            rhs: ValueId(1),
        });

        function.push_operation(Operation::Return {
            value: Some(second.id),
        });

        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReduceCryptoOperations,
                priority: HintPriority::High,
                target: TargetMetric::BootstrapCount,
                current_value: 2.0,
                reason: "duplicate crypto work".to_string(),
            }],
        };

        let plan = build_optimization_plan(&hints);

        let mut optimized = function.clone();

        optimize_function_with_plan(&mut optimized, &plan);

        assert_equivalent(&function, &optimized);
    }

    fn deep_boolean_chain(kind: &str) -> Function {
        let mut function = Function::new("deep_chain", bool_parameters(6), Type::Bool);
        let mut current = ValueId(0);

        for next in 1..6 {
            let result = function.create_value(Type::Bool).id;
            let rhs = ValueId(next);
            let operation = match kind {
                "and" => Operation::And {
                    result,
                    lhs: current,
                    rhs,
                },
                "or" => Operation::Or {
                    result,
                    lhs: current,
                    rhs,
                },
                "xor" => Operation::Xor {
                    result,
                    lhs: current,
                    rhs,
                },
                _ => unreachable!(),
            };
            function.push_operation(operation);
            current = result;
        }

        function.push_operation(Operation::Return {
            value: Some(current),
        });
        function
    }

    #[test]
    fn deep_boolean_chains_preserve_all_truth_table_rows() {
        let hints = OptimizerHintSet {
            hints: vec![
                OptimizerHint {
                    kind: OptimizerHintKind::ReduceDepth,
                    priority: HintPriority::High,
                    target: TargetMetric::CircuitDepth,
                    current_value: 5.0,
                    reason: "deep chain".to_string(),
                },
                OptimizerHint {
                    kind: OptimizerHintKind::IncreaseParallelism,
                    priority: HintPriority::High,
                    target: TargetMetric::Parallelism,
                    current_value: 1.0,
                    reason: "expose parallel work".to_string(),
                },
            ],
        };
        let plan = build_optimization_plan(&hints);

        for kind in ["and", "or", "xor"] {
            let original = deep_boolean_chain(kind);
            let mut optimized = original.clone();

            for _ in 0..3 {
                optimize_function_with_plan(&mut optimized, &plan);
            }

            fhec_ir::verify_function(&optimized).expect("optimized IR is invalid");
            assert_equivalent(&original, &optimized);
        }
    }

    fn interleaved_chain(kind: &str, delayed_operand: bool) -> Function {
        let mut function = Function::new("interleaved", bool_parameters(5), Type::Bool);
        let first = function.create_value(Type::Bool).id;
        let middle = function.create_value(Type::Bool).id;
        let other = function.create_value(Type::Bool).id;
        let last = function.create_value(Type::Bool).id;

        let binary = |result, lhs, rhs| match kind {
            "and" => Operation::And { result, lhs, rhs },
            "or" => Operation::Or { result, lhs, rhs },
            "xor" => Operation::Xor { result, lhs, rhs },
            _ => unreachable!(),
        };

        function.push_operation(binary(first, ValueId(0), ValueId(1)));
        if !delayed_operand {
            function.push_operation(binary(other, ValueId(3), ValueId(4)));
        }
        function.push_operation(binary(middle, first, ValueId(2)));
        if delayed_operand {
            function.push_operation(binary(other, ValueId(3), ValueId(4)));
        }
        function.push_operation(binary(last, middle, other));
        function.push_operation(Operation::Return { value: Some(last) });
        function
    }

    #[test]
    fn depth_reduction_preserves_definition_order_and_truth_tables() {
        let hints = OptimizerHintSet {
            hints: vec![OptimizerHint {
                kind: OptimizerHintKind::ReduceDepth,
                priority: HintPriority::High,
                target: TargetMetric::CircuitDepth,
                current_value: 4.0,
                reason: "depth".to_string(),
            }],
        };
        let plan = build_optimization_plan(&hints);

        for kind in ["and", "or", "xor"] {
            for delayed in [false, true] {
                let original = interleaved_chain(kind, delayed);
                fhec_ir::verify_function(&original).expect("invalid original IR");
                let mut optimized = original.clone();

                for _ in 0..3 {
                    optimize_function_with_plan(&mut optimized, &plan);
                }

                fhec_ir::verify_function(&optimized)
                    .expect("rewrite introduced a forward reference");
                assert_equivalent(&original, &optimized);

                if delayed {
                    assert_eq!(optimized.operations, original.operations);
                } else {
                    assert_ne!(optimized.operations, original.operations);
                }
            }
        }
    }
}
