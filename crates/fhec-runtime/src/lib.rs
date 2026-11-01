pub mod v4;

use fhec_analyzer::{
    AnalysisError, CostModel, ExecutionStrategy, StrategyAnalysis, analyze_circuit,
    analyze_execution_strategy,
};
use fhec_backend::{ExecutionError, FheBackend, execute_circuit, execute_circuit_parallel};
use fhec_circuit::{Circuit, WireId};

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeDecision {
    pub strategy: ExecutionStrategy,
    pub analysis: StrategyAnalysis,
}

#[derive(Debug)]
pub enum RuntimeError<E> {
    Analysis(AnalysisError),
    Execution(ExecutionError<E>),
}

pub type RuntimeExecutionResult<B> = Result<
    (Vec<<B as FheBackend>::Ciphertext>, RuntimeDecision),
    RuntimeError<<B as FheBackend>::Error>,
>;

pub fn choose_execution_strategy(
    circuit: &Circuit,
    outputs: &[WireId],
    model: &CostModel,
) -> Result<RuntimeDecision, AnalysisError> {
    let circuit_analysis = analyze_circuit(circuit, outputs)?;

    let strategy_analysis = analyze_execution_strategy(&circuit_analysis, model);

    Ok(RuntimeDecision {
        strategy: strategy_analysis.recommended_strategy,
        analysis: strategy_analysis,
    })
}

pub fn execute_auto<B>(
    backend: &B,
    circuit: &Circuit,
    inputs: &[B::Ciphertext],
    outputs: &[WireId],
    model: &CostModel,
) -> RuntimeExecutionResult<B>
where
    B: FheBackend + Sync,
    B::Ciphertext: Clone + Send + Sync,
    B::Error: Send,
{
    let decision =
        choose_execution_strategy(circuit, outputs, model).map_err(RuntimeError::Analysis)?;

    let result = match decision.strategy {
        ExecutionStrategy::Sequential => execute_circuit(backend, circuit, inputs, outputs),

        ExecutionStrategy::Parallel => execute_circuit_parallel(backend, circuit, inputs, outputs),
    }
    .map_err(RuntimeError::Execution)?;

    Ok((result, decision))
}

pub fn execute_auto_default<B>(
    backend: &B,
    circuit: &Circuit,
    inputs: &[B::Ciphertext],
    outputs: &[WireId],
) -> RuntimeExecutionResult<B>
where
    B: FheBackend + Sync,
    B::Ciphertext: Clone + Send + Sync,
    B::Error: Send,
{
    execute_auto(backend, circuit, inputs, outputs, &CostModel::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockCiphertext(bool);

    struct MockBackend;

    impl FheBackend for MockBackend {
        type Ciphertext = MockCiphertext;

        type Error = ();

        fn encrypt_bool(&self, value: bool) -> Result<Self::Ciphertext, Self::Error> {
            Ok(MockCiphertext(value))
        }

        fn decrypt_bool(&self, value: &Self::Ciphertext) -> Result<bool, Self::Error> {
            Ok(value.0)
        }

        fn not(&self, value: &Self::Ciphertext) -> Result<Self::Ciphertext, Self::Error> {
            Ok(MockCiphertext(!value.0))
        }

        fn and(
            &self,
            lhs: &Self::Ciphertext,
            rhs: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(MockCiphertext(lhs.0 && rhs.0))
        }

        fn or(
            &self,
            lhs: &Self::Ciphertext,
            rhs: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(MockCiphertext(lhs.0 || rhs.0))
        }

        fn xor(
            &self,
            lhs: &Self::Ciphertext,
            rhs: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(MockCiphertext(lhs.0 ^ rhs.0))
        }

        fn mux(
            &self,
            condition: &Self::Ciphertext,
            if_true: &Self::Ciphertext,
            if_false: &Self::Ciphertext,
        ) -> Result<Self::Ciphertext, Self::Error> {
            Ok(if condition.0 {
                if_true.clone()
            } else {
                if_false.clone()
            })
        }
    }

    #[test]
    fn chooses_sequential_for_chain() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let first = circuit.add_xor(a, b);

        let second = circuit.add_not(first);

        let output = circuit.add_and(second, a);

        let decision =
            choose_execution_strategy(&circuit, &[output], &CostModel::default()).unwrap();

        assert_eq!(decision.strategy, ExecutionStrategy::Sequential);
    }

    #[test]
    fn chooses_parallel_for_wide_circuit() {
        let mut circuit = Circuit::new();

        let mut layer = Vec::new();

        for _ in 0..8 {
            let a = circuit.add_input();

            let b = circuit.add_input();

            layer.push(circuit.add_xor(a, b));
        }

        let mut current = layer;

        while current.len() > 1 {
            let mut next = Vec::new();

            for pair in current.chunks(2) {
                if pair.len() == 2 {
                    next.push(circuit.add_or(pair[0], pair[1]));
                } else {
                    next.push(pair[0]);
                }
            }

            current = next;
        }

        let output = current[0];

        let decision =
            choose_execution_strategy(&circuit, &[output], &CostModel::default()).unwrap();

        assert_eq!(decision.strategy, ExecutionStrategy::Parallel);
    }

    #[test]
    fn auto_executor_runs_sequential_circuit() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let xor = circuit.add_xor(a, b);

        let not = circuit.add_not(xor);

        let output = circuit.add_and(not, a);

        let backend = MockBackend;

        let inputs = vec![MockCiphertext(true), MockCiphertext(false)];

        let (result, decision) =
            execute_auto_default(&backend, &circuit, &inputs, &[output]).unwrap();

        assert_eq!(decision.strategy, ExecutionStrategy::Sequential);

        assert_eq!(result, vec![MockCiphertext(false)]);
    }

    #[test]
    fn auto_executor_runs_parallel_circuit() {
        let mut circuit = Circuit::new();

        let mut outputs = Vec::new();

        for _ in 0..8 {
            let a = circuit.add_input();

            let b = circuit.add_input();

            outputs.push(circuit.add_xor(a, b));
        }

        let backend = MockBackend;

        let mut inputs = Vec::new();

        for _ in 0..8 {
            inputs.push(MockCiphertext(true));

            inputs.push(MockCiphertext(false));
        }

        let (result, decision) =
            execute_auto_default(&backend, &circuit, &inputs, &outputs).unwrap();

        assert_eq!(decision.strategy, ExecutionStrategy::Parallel);

        assert_eq!(result.len(), 8);

        assert!(result.iter().all(|value| value.0));
    }

    #[test]
    fn runtime_returns_strategy_analysis() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_xor(a, b);

        let backend = MockBackend;

        let inputs = vec![MockCiphertext(true), MockCiphertext(false)];

        let (outputs, decision) =
            execute_auto_default(&backend, &circuit, &inputs, &[output]).unwrap();

        assert_eq!(outputs, vec![MockCiphertext(true)]);

        assert_eq!(decision.analysis.compute_gate_count, 1);

        assert_eq!(decision.strategy, ExecutionStrategy::Sequential);
    }
}
