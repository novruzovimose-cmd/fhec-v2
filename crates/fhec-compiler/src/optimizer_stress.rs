use fhec_analyzer::hints::{
    HintPriority, OptimizerHint, OptimizerHintKind, OptimizerHintSet, TargetMetric,
};
use fhec_ir::{Function, Operation, Type, Value, ValueId};
use fhec_optimizer::{hints::build_optimization_plan, optimize_function_with_plan};

use crate::lower_function_to_circuit;

fn next_random(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn optimization_plan() -> fhec_optimizer::hints::OptimizationPlan {
    let configurations = [
        (OptimizerHintKind::ReduceDepth, TargetMetric::CircuitDepth),
        (
            OptimizerHintKind::IncreaseParallelism,
            TargetMetric::Parallelism,
        ),
        (
            OptimizerHintKind::ReduceCryptoOperations,
            TargetMetric::BootstrapCount,
        ),
        (
            OptimizerHintKind::ReplaceExpensiveGates,
            TargetMetric::GateCost,
        ),
    ];

    let hints = configurations
        .into_iter()
        .map(|(kind, target)| OptimizerHint {
            kind,
            priority: HintPriority::High,
            target,
            current_value: 10.0,
            reason: "optimizer equivalence stress test".to_string(),
        })
        .collect();

    build_optimization_plan(&OptimizerHintSet { hints })
}

fn generated_boolean_function(seed: u32) -> Function {
    let parameters = (0..5)
        .map(|index| Value::new(ValueId(index), Type::Bool))
        .collect();

    let mut function = Function::new(format!("generated_{seed}"), parameters, Type::Bool);

    let mut state = seed.wrapping_add(1);
    let mut available = (0..5).map(ValueId).collect::<Vec<_>>();

    for _ in 0..16 {
        let lhs_index = (next_random(&mut state) as usize) % available.len();
        let rhs_index = (next_random(&mut state) as usize) % available.len();
        let condition_index = (next_random(&mut state) as usize) % available.len();

        let lhs = available[lhs_index];
        let rhs = available[rhs_index];
        let condition = available[condition_index];

        let result = function.create_value(Type::Bool).id;

        let operation = match next_random(&mut state) % 5 {
            0 => Operation::And { result, lhs, rhs },
            1 => Operation::Or { result, lhs, rhs },
            2 => Operation::Xor { result, lhs, rhs },
            3 => Operation::Not {
                result,
                operand: lhs,
            },
            _ => Operation::Mux {
                result,
                condition,
                if_true: lhs,
                if_false: rhs,
            },
        };

        function.push_operation(operation);
        available.push(result);
    }

    function.push_operation(Operation::Return {
        value: available.last().copied(),
    });

    function
}

fn verify_equivalence(original: &Function, seed: u32) {
    let plan = optimization_plan();
    let mut optimized = original.clone();

    optimize_function_with_plan(&mut optimized, &plan);

    let original_circuit = lower_function_to_circuit(original);
    let optimized_circuit = lower_function_to_circuit(&optimized);

    original_circuit
        .verify(original)
        .expect("original circuit is invalid");

    optimized_circuit
        .verify(&optimized)
        .expect("optimized circuit is invalid");

    for mask in 0..32usize {
        let inputs = (0..5)
            .map(|bit| ((mask >> bit) & 1) != 0)
            .collect::<Vec<_>>();

        let before = original_circuit
            .circuit
            .evaluate(&inputs, &original_circuit.outputs)
            .expect("original evaluation failed");

        let after = optimized_circuit
            .circuit
            .evaluate(&inputs, &optimized_circuit.outputs)
            .expect("optimized evaluation failed");

        assert_eq!(
            before, after,
            "optimization mismatch: seed={seed}, input_mask={mask}"
        );
    }
}

#[test]
fn generated_boolean_programs_preserve_semantics() {
    for seed in 0..64u32 {
        let function = generated_boolean_function(seed);
        verify_equivalence(&function, seed);
    }
}

#[test]
fn repeated_combined_passes_preserve_semantics() {
    let plan = optimization_plan();

    for seed in 64..96u32 {
        let original = generated_boolean_function(seed);
        let mut optimized = original.clone();

        for _ in 0..3 {
            optimize_function_with_plan(&mut optimized, &plan);
        }

        let before = lower_function_to_circuit(&original);
        let after = lower_function_to_circuit(&optimized);

        before.verify(&original).expect("invalid baseline");
        after.verify(&optimized).expect("invalid optimized circuit");

        for mask in 0..32usize {
            let inputs = (0..5)
                .map(|bit| ((mask >> bit) & 1) != 0)
                .collect::<Vec<_>>();

            let baseline = before
                .circuit
                .evaluate(&inputs, &before.outputs)
                .expect("baseline evaluation failed");

            let result = after
                .circuit
                .evaluate(&inputs, &after.outputs)
                .expect("optimized evaluation failed");

            assert_eq!(
                baseline, result,
                "repeated-pass mismatch: seed={seed}, mask={mask}"
            );
        }
    }
}
