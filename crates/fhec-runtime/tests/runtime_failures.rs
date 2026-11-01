use fhec_analyzer::{CostModel, v4::V4CostModel};
use fhec_backend::{
    ExecutionError, FheBackend, execute_circuit, execute_circuit_chunked_parallel,
    execute_circuit_parallel,
};
use fhec_circuit::{Circuit, WireId};
use fhec_runtime::{
    RuntimeError, execute_auto,
    v4::{V4RuntimeError, execute_auto_v4},
};

struct FailureBackend;

impl FheBackend for FailureBackend {
    type Ciphertext = bool;
    type Error = &'static str;

    fn encrypt_bool(&self, value: bool) -> Result<bool, Self::Error> {
        Ok(value)
    }
    fn decrypt_bool(&self, value: &bool) -> Result<bool, Self::Error> {
        Ok(*value)
    }
    fn not(&self, value: &bool) -> Result<bool, Self::Error> {
        Ok(!*value)
    }
    fn and(&self, lhs: &bool, rhs: &bool) -> Result<bool, Self::Error> {
        Ok(*lhs && *rhs)
    }
    fn or(&self, lhs: &bool, rhs: &bool) -> Result<bool, Self::Error> {
        Ok(*lhs || *rhs)
    }
    fn xor(&self, _lhs: &bool, _rhs: &bool) -> Result<bool, Self::Error> {
        Err("injected-failure")
    }
    fn mux(&self, condition: &bool, lhs: &bool, rhs: &bool) -> Result<bool, Self::Error> {
        Ok(if *condition { *lhs } else { *rhs })
    }
}

fn circuit() -> (Circuit, WireId) {
    let mut circuit = Circuit::new();
    let lhs = circuit.add_input();
    let rhs = circuit.add_input();
    let out = circuit.add_xor(lhs, rhs);
    (circuit, out)
}

#[test]
fn backend_failure_is_propagated_by_all_execution_strategies() {
    let (circuit, out) = circuit();
    let inputs = [true, false];
    let backend = FailureBackend;

    assert!(matches!(
        execute_circuit(&backend, &circuit, &inputs, &[out]),
        Err(ExecutionError::Backend("injected-failure"))
    ));
    assert!(matches!(
        execute_circuit_parallel(&backend, &circuit, &inputs, &[out]),
        Err(ExecutionError::Backend("injected-failure"))
    ));
    assert!(matches!(
        execute_circuit_chunked_parallel(&backend, &circuit, &inputs, &[out], 0),
        Err(ExecutionError::Backend("injected-failure"))
    ));
    assert!(matches!(
        execute_auto(&backend, &circuit, &inputs, &[out], &CostModel::default()),
        Err(RuntimeError::Execution(ExecutionError::Backend(
            "injected-failure"
        )))
    ));
    assert!(matches!(
        execute_auto_v4(&backend, &circuit, &inputs, &[out], &V4CostModel::default()),
        Err(V4RuntimeError::Execution(ExecutionError::Backend(
            "injected-failure"
        )))
    ));
}

#[test]
fn insufficient_inputs_return_typed_errors_without_running_backend() {
    let (circuit, out) = circuit();
    let inputs = [true];
    let backend = FailureBackend;
    assert!(matches!(
        execute_circuit(&backend, &circuit, &inputs, &[out]),
        Err(ExecutionError::InvalidInputCount {
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        execute_circuit_parallel(&backend, &circuit, &inputs, &[out]),
        Err(ExecutionError::InvalidInputCount {
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        execute_circuit_chunked_parallel(&backend, &circuit, &inputs, &[out], 3),
        Err(ExecutionError::InvalidInputCount {
            expected: 2,
            actual: 1
        })
    ));
}

#[test]
fn invalid_output_returns_error_instead_of_panicking() {
    let mut circuit = Circuit::new();
    circuit.add_input();
    let inputs = [true];
    let output = WireId(99);
    let backend = FailureBackend;
    assert!(matches!(
        execute_circuit(&backend, &circuit, &inputs, &[output]),
        Err(ExecutionError::MissingWire(WireId(99)))
    ));
    assert!(matches!(
        execute_circuit_parallel(&backend, &circuit, &inputs, &[output]),
        Err(ExecutionError::MissingWire(WireId(99)))
    ));
    assert!(matches!(
        execute_circuit_chunked_parallel(&backend, &circuit, &inputs, &[output], 1),
        Err(ExecutionError::MissingWire(WireId(99)))
    ));
    assert!(matches!(
        execute_auto_v4(
            &backend,
            &circuit,
            &inputs,
            &[output],
            &V4CostModel::default()
        ),
        Err(V4RuntimeError::Analysis(_))
    ));
}
