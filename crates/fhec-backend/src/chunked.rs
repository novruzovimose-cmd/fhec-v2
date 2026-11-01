use fhec_circuit::{Circuit, Constant, Gate, WireId};

use rayon::prelude::*;

use crate::{ExecutionError, FheBackend, build_execution_schedule};

pub fn execute_circuit_chunked_parallel<B>(
    backend: &B,
    circuit: &Circuit,
    inputs: &[B::Ciphertext],
    outputs: &[WireId],
    chunk_size: usize,
) -> Result<Vec<B::Ciphertext>, ExecutionError<B::Error>>
where
    B: FheBackend + Sync,
    B::Ciphertext: Clone + Send + Sync,
    B::Error: Send,
{
    let schedule = build_execution_schedule(circuit).map_err(ExecutionError::InvalidCircuit)?;

    let input_wires = circuit
        .gates()
        .iter()
        .filter_map(|gate| match gate {
            Gate::Input { wire, .. } => Some(*wire),

            _ => None,
        })
        .collect::<Vec<_>>();

    if inputs.len() != input_wires.len() {
        return Err(ExecutionError::InvalidInputCount {
            expected: input_wires.len(),

            actual: inputs.len(),
        });
    }

    let mut wires: Vec<Option<B::Ciphertext>> = vec![None; circuit.wire_count()];

    for (wire, input) in input_wires.iter().zip(inputs.iter()) {
        wires[wire.0 as usize] = Some(input.clone());
    }

    for gate in circuit.gates() {
        if let Gate::Constant { wire, value, .. } = gate {
            let value = matches!(value, Constant::True);

            let ciphertext = backend
                .encrypt_bool(value)
                .map_err(ExecutionError::Backend)?;

            wires[wire.0 as usize] = Some(ciphertext);
        }
    }

    let chunk_size = chunk_size.max(1);

    for layer in &schedule.layers {
        let compute_indices = layer
            .iter()
            .copied()
            .filter(|gate_index| {
                !matches!(
                    circuit.gates()[*gate_index],
                    Gate::Input { .. } | Gate::Constant { .. }
                )
            })
            .collect::<Vec<_>>();

        let chunks = compute_indices
            .par_chunks(chunk_size)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|gate_index| evaluate_gate(backend, &circuit.gates()[*gate_index], &wires))
                    .collect::<Result<Vec<_>, ExecutionError<B::Error>>>()
            })
            .collect::<Result<Vec<_>, ExecutionError<B::Error>>>()?;

        for chunk in chunks {
            for (wire, ciphertext) in chunk {
                wires[wire.0 as usize] = Some(ciphertext);
            }
        }
    }

    outputs
        .iter()
        .map(|wire| get_wire(&wires, *wire).cloned())
        .collect()
}

fn evaluate_gate<B>(
    backend: &B,
    gate: &Gate,
    wires: &[Option<B::Ciphertext>],
) -> Result<(WireId, B::Ciphertext), ExecutionError<B::Error>>
where
    B: FheBackend + Sync,
    B::Ciphertext: Clone + Send + Sync,
    B::Error: Send,
{
    match gate {
        Gate::Not { wire, operand, .. } => {
            let operand = get_wire(wires, *operand)?;

            let result = backend.not(operand).map_err(ExecutionError::Backend)?;

            Ok((*wire, result))
        }

        Gate::And { wire, lhs, rhs, .. } => {
            let lhs = get_wire(wires, *lhs)?;

            let rhs = get_wire(wires, *rhs)?;

            let result = backend.and(lhs, rhs).map_err(ExecutionError::Backend)?;

            Ok((*wire, result))
        }

        Gate::Or { wire, lhs, rhs, .. } => {
            let lhs = get_wire(wires, *lhs)?;

            let rhs = get_wire(wires, *rhs)?;

            let result = backend.or(lhs, rhs).map_err(ExecutionError::Backend)?;

            Ok((*wire, result))
        }

        Gate::Xor { wire, lhs, rhs, .. } => {
            let lhs = get_wire(wires, *lhs)?;

            let rhs = get_wire(wires, *rhs)?;

            let result = backend.xor(lhs, rhs).map_err(ExecutionError::Backend)?;

            Ok((*wire, result))
        }

        Gate::Mux {
            wire,
            condition,
            if_true,
            if_false,
            ..
        } => {
            let condition = get_wire(wires, *condition)?;

            let if_true = get_wire(wires, *if_true)?;

            let if_false = get_wire(wires, *if_false)?;

            let result = backend
                .mux(condition, if_true, if_false)
                .map_err(ExecutionError::Backend)?;

            Ok((*wire, result))
        }

        Gate::Input { wire, .. } | Gate::Constant { wire, .. } => {
            Ok((*wire, get_wire(wires, *wire)?.clone()))
        }
    }
}

fn get_wire<C, E>(wires: &[Option<C>], wire: WireId) -> Result<&C, ExecutionError<E>> {
    wires
        .get(wire.0 as usize)
        .and_then(Option::as_ref)
        .ok_or(ExecutionError::MissingWire(wire))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBackend;

    impl FheBackend for MockBackend {
        type Ciphertext = bool;

        type Error = ();

        fn encrypt_bool(&self, value: bool) -> Result<bool, ()> {
            Ok(value)
        }

        fn decrypt_bool(&self, value: &bool) -> Result<bool, ()> {
            Ok(*value)
        }

        fn not(&self, value: &bool) -> Result<bool, ()> {
            Ok(!*value)
        }

        fn and(&self, lhs: &bool, rhs: &bool) -> Result<bool, ()> {
            Ok(*lhs && *rhs)
        }

        fn or(&self, lhs: &bool, rhs: &bool) -> Result<bool, ()> {
            Ok(*lhs || *rhs)
        }

        fn xor(&self, lhs: &bool, rhs: &bool) -> Result<bool, ()> {
            Ok(*lhs ^ *rhs)
        }

        fn mux(&self, condition: &bool, if_true: &bool, if_false: &bool) -> Result<bool, ()> {
            Ok(if *condition { *if_true } else { *if_false })
        }
    }

    #[test]
    fn chunked_executor_matches_boolean_result() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let x = circuit.add_xor(a, b);

        let y = circuit.add_and(a, b);

        let output = circuit.add_or(x, y);

        let result =
            execute_circuit_chunked_parallel(&MockBackend, &circuit, &[true, false], &[output], 2)
                .unwrap();

        assert_eq!(result, vec![true]);
    }
}
