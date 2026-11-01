mod chunked;

pub use chunked::execute_circuit_chunked_parallel;

use fhec_circuit::{Circuit, Constant, Gate, VerificationError, WireId};

use rayon::prelude::*;

pub trait FheBackend {
    type Ciphertext;
    type Error;

    fn encrypt_bool(&self, value: bool) -> Result<Self::Ciphertext, Self::Error>;

    fn decrypt_bool(&self, value: &Self::Ciphertext) -> Result<bool, Self::Error>;

    fn not(&self, value: &Self::Ciphertext) -> Result<Self::Ciphertext, Self::Error>;

    fn and(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error>;

    fn or(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error>;

    fn xor(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error>;

    fn mux(
        &self,
        condition: &Self::Ciphertext,
        if_true: &Self::Ciphertext,
        if_false: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error>;
}

#[derive(Debug)]
pub enum ExecutionError<E> {
    InvalidCircuit(VerificationError),
    InvalidInputCount { expected: usize, actual: usize },
    MissingWire(WireId),
    Backend(E),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSchedule {
    pub layers: Vec<Vec<usize>>,
}

impl ExecutionSchedule {
    pub fn depth(&self) -> usize {
        self.layers.len().saturating_sub(1)
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn max_parallelism(&self) -> usize {
        self.layers.iter().map(Vec::len).max().unwrap_or(0)
    }

    pub fn gate_count(&self) -> usize {
        self.layers.iter().map(Vec::len).sum()
    }
}

pub fn build_execution_schedule(circuit: &Circuit) -> Result<ExecutionSchedule, VerificationError> {
    circuit.verify()?;

    let mut wire_depths = vec![0usize; circuit.wire_count()];

    let mut layers: Vec<Vec<usize>> = Vec::new();

    for (gate_index, gate) in circuit.gates().iter().enumerate() {
        let (wire, depth) = match gate {
            Gate::Constant { wire, .. } => (*wire, 0),

            Gate::Input { wire, .. } => (*wire, 0),

            Gate::Not { wire, operand, .. } => (*wire, wire_depths[operand.0 as usize] + 1),

            Gate::And { wire, lhs, rhs, .. }
            | Gate::Or { wire, lhs, rhs, .. }
            | Gate::Xor { wire, lhs, rhs, .. } => (
                *wire,
                wire_depths[lhs.0 as usize].max(wire_depths[rhs.0 as usize]) + 1,
            ),

            Gate::Mux {
                wire,
                condition,
                if_true,
                if_false,
                ..
            } => {
                let parent_depth = wire_depths[condition.0 as usize]
                    .max(wire_depths[if_true.0 as usize])
                    .max(wire_depths[if_false.0 as usize]);

                (*wire, parent_depth + 1)
            }
        };

        wire_depths[wire.0 as usize] = depth;

        if layers.len() <= depth {
            layers.resize_with(depth + 1, Vec::new);
        }

        layers[depth].push(gate_index);
    }

    Ok(ExecutionSchedule { layers })
}

pub fn execute_circuit<B>(
    backend: &B,
    circuit: &Circuit,
    inputs: &[B::Ciphertext],
    outputs: &[WireId],
) -> Result<Vec<B::Ciphertext>, ExecutionError<B::Error>>
where
    B: FheBackend,
    B::Ciphertext: Clone,
{
    let schedule = build_execution_schedule(circuit).map_err(ExecutionError::InvalidCircuit)?;

    let expected_inputs = circuit
        .gates()
        .iter()
        .filter(|gate| matches!(gate, Gate::Input { .. }))
        .count();

    if inputs.len() != expected_inputs {
        return Err(ExecutionError::InvalidInputCount {
            expected: expected_inputs,
            actual: inputs.len(),
        });
    }

    let mut wires: Vec<Option<B::Ciphertext>> = vec![None; circuit.wire_count()];

    let mut input_index = 0usize;

    for layer in &schedule.layers {
        for gate_index in layer {
            let gate = &circuit.gates()[*gate_index];

            match gate {
                Gate::Constant { wire, value, .. } => {
                    let value = matches!(value, Constant::True);

                    let ciphertext = backend
                        .encrypt_bool(value)
                        .map_err(ExecutionError::Backend)?;

                    wires[wire.0 as usize] = Some(ciphertext);
                }

                Gate::Input { wire, .. } => {
                    wires[wire.0 as usize] = Some(inputs[input_index].clone());

                    input_index += 1;
                }

                Gate::Not { wire, operand, .. } => {
                    let operand = get_wire(&wires, *operand)?;

                    let result = backend.not(operand).map_err(ExecutionError::Backend)?;

                    wires[wire.0 as usize] = Some(result);
                }

                Gate::And { wire, lhs, rhs, .. } => {
                    let lhs = get_wire(&wires, *lhs)?;

                    let rhs = get_wire(&wires, *rhs)?;

                    let result = backend.and(lhs, rhs).map_err(ExecutionError::Backend)?;

                    wires[wire.0 as usize] = Some(result);
                }

                Gate::Or { wire, lhs, rhs, .. } => {
                    let lhs = get_wire(&wires, *lhs)?;

                    let rhs = get_wire(&wires, *rhs)?;

                    let result = backend.or(lhs, rhs).map_err(ExecutionError::Backend)?;

                    wires[wire.0 as usize] = Some(result);
                }

                Gate::Xor { wire, lhs, rhs, .. } => {
                    let lhs = get_wire(&wires, *lhs)?;

                    let rhs = get_wire(&wires, *rhs)?;

                    let result = backend.xor(lhs, rhs).map_err(ExecutionError::Backend)?;

                    wires[wire.0 as usize] = Some(result);
                }

                Gate::Mux {
                    wire,
                    condition,
                    if_true,
                    if_false,
                    ..
                } => {
                    let condition = get_wire(&wires, *condition)?;

                    let if_true = get_wire(&wires, *if_true)?;

                    let if_false = get_wire(&wires, *if_false)?;

                    let result = backend
                        .mux(condition, if_true, if_false)
                        .map_err(ExecutionError::Backend)?;

                    wires[wire.0 as usize] = Some(result);
                }
            }
        }
    }

    outputs
        .iter()
        .map(|wire| get_wire(&wires, *wire).cloned())
        .collect()
}

pub fn execute_circuit_parallel<B>(
    backend: &B,
    circuit: &Circuit,
    inputs: &[B::Ciphertext],
    outputs: &[WireId],
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

    for layer in &schedule.layers {
        let results = layer
            .par_iter()
            .filter_map(|gate_index| {
                let gate = &circuit.gates()[*gate_index];

                match gate {
                    Gate::Input { .. } | Gate::Constant { .. } => None,

                    _ => Some(gate),
                }
            })
            .map(|gate| evaluate_gate_parallel(backend, gate, &wires))
            .collect::<Result<Vec<_>, ExecutionError<B::Error>>>()?;

        for (wire, ciphertext) in results {
            wires[wire.0 as usize] = Some(ciphertext);
        }
    }

    outputs
        .iter()
        .map(|wire| get_wire(&wires, *wire).cloned())
        .collect()
}

fn evaluate_gate_parallel<B>(
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
            let value = get_wire(wires, *wire)?.clone();

            Ok((*wire, value))
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

        fn encrypt_bool(&self, value: bool) -> Result<Self::Ciphertext, Self::Error> {
            Ok(value)
        }

        fn decrypt_bool(&self, value: &Self::Ciphertext) -> Result<bool, Self::Error> {
            Ok(*value)
        }

        fn not(&self, value: &Self::Ciphertext) -> Result<Self::Ciphertext, Self::Error> {
            Ok(!*value)
        }

        fn and(
            &self,
            lhs: &Self::Ciphertext,
            rhs: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(*lhs && *rhs)
        }

        fn or(
            &self,
            lhs: &Self::Ciphertext,
            rhs: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(*lhs || *rhs)
        }

        fn xor(
            &self,
            lhs: &Self::Ciphertext,
            rhs: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(*lhs ^ *rhs)
        }

        fn mux(
            &self,
            condition: &Self::Ciphertext,
            if_true: &Self::Ciphertext,
            if_false: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(if *condition { *if_true } else { *if_false })
        }
    }

    #[test]
    fn execution_schedule_groups_independent_gates() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let and = circuit.add_and(a, b);

        circuit.add_or(xor, and);

        let schedule = build_execution_schedule(&circuit).unwrap();

        assert_eq!(schedule.layers.len(), 3);

        assert_eq!(schedule.layers[0].len(), 2);

        assert_eq!(schedule.layers[1].len(), 2);

        assert_eq!(schedule.layers[2].len(), 1);

        assert_eq!(schedule.depth(), 2);

        assert_eq!(schedule.max_parallelism(), 2);

        assert_eq!(schedule.gate_count(), 5);
    }

    #[test]
    fn execution_schedule_handles_mux() {
        let mut circuit = Circuit::new();

        let condition = circuit.add_input();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_mux(condition, a, b);

        let schedule = build_execution_schedule(&circuit).unwrap();

        assert_eq!(schedule.layers.len(), 2);

        assert_eq!(schedule.layers[0].len(), 3);

        assert_eq!(schedule.layers[1].len(), 1);

        assert_eq!(schedule.depth(), 1);

        assert_eq!(output, WireId(3));
    }

    #[test]
    fn executes_complete_circuit() {
        let backend = MockBackend;

        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let output = circuit.add_not(xor);

        let encrypted_inputs = vec![
            backend.encrypt_bool(true).unwrap(),
            backend.encrypt_bool(false).unwrap(),
        ];

        let encrypted_outputs =
            execute_circuit(&backend, &circuit, &encrypted_inputs, &[output]).unwrap();

        assert!(!backend.decrypt_bool(&encrypted_outputs[0],).unwrap());
    }

    #[test]
    fn layered_execution_preserves_parallel_branches() {
        let backend = MockBackend;

        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let and = circuit.add_and(a, b);

        let output = circuit.add_or(xor, and);

        let encrypted_inputs = vec![true, false];

        let encrypted_outputs =
            execute_circuit(&backend, &circuit, &encrypted_inputs, &[output]).unwrap();

        assert_eq!(encrypted_outputs, vec![true]);
    }

    #[test]
    fn parallel_executor_matches_sequential_executor() {
        let backend = MockBackend;

        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let and = circuit.add_and(a, b);

        let left = circuit.add_not(xor);

        let output = circuit.add_or(left, and);

        let inputs = vec![true, false];

        let sequential = execute_circuit(&backend, &circuit, &inputs, &[output]).unwrap();

        let parallel = execute_circuit_parallel(&backend, &circuit, &inputs, &[output]).unwrap();

        assert_eq!(sequential, parallel);
    }

    #[test]
    fn parallel_executor_handles_mux() {
        let backend = MockBackend;

        let mut circuit = Circuit::new();

        let condition = circuit.add_input();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_mux(condition, a, b);

        let inputs = vec![true, true, false];

        let result = execute_circuit_parallel(&backend, &circuit, &inputs, &[output]).unwrap();

        assert_eq!(result, vec![true]);
    }

    #[test]
    fn executor_handles_mux() {
        let backend = MockBackend;

        let mut circuit = Circuit::new();

        let condition = circuit.add_input();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_mux(condition, a, b);

        let encrypted_inputs = vec![
            backend.encrypt_bool(true).unwrap(),
            backend.encrypt_bool(true).unwrap(),
            backend.encrypt_bool(false).unwrap(),
        ];

        let encrypted_outputs =
            execute_circuit(&backend, &circuit, &encrypted_inputs, &[output]).unwrap();

        assert!(backend.decrypt_bool(&encrypted_outputs[0],).unwrap());
    }

    #[test]
    fn executor_rejects_wrong_input_count() {
        let backend = MockBackend;

        let mut circuit = Circuit::new();

        circuit.add_input();
        circuit.add_input();

        let result = execute_circuit(&backend, &circuit, &[true], &[]);

        assert!(matches!(
            result,
            Err(ExecutionError::InvalidInputCount {
                expected: 2,
                actual: 1
            })
        ));
    }

    #[test]
    fn parallel_executor_rejects_wrong_input_count() {
        let backend = MockBackend;

        let mut circuit = Circuit::new();

        circuit.add_input();
        circuit.add_input();

        let result = execute_circuit_parallel(&backend, &circuit, &[true], &[]);

        assert!(matches!(
            result,
            Err(ExecutionError::InvalidInputCount {
                expected: 2,
                actual: 1
            })
        ));
    }
}
