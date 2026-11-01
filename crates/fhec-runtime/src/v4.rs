use std::time::Instant;

use fhec_analyzer::{
    AnalysisError,
    v4::{
        PredictionSample, PredictionTracker, V4CostModel, V4ExecutionStrategy, V4StrategyAnalysis,
        analyze_v4,
    },
};

use fhec_backend::{
    ExecutionError, FheBackend, execute_circuit, execute_circuit_chunked_parallel,
    execute_circuit_parallel,
};

use fhec_circuit::{Circuit, WireId};

#[derive(Debug, Clone, PartialEq)]
pub struct V4RuntimeDecision {
    pub strategy: V4ExecutionStrategy,

    pub analysis: V4StrategyAnalysis,
}

#[derive(Debug)]
pub enum V4RuntimeError<E> {
    Analysis(AnalysisError),

    Execution(ExecutionError<E>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct V4ExecutionProfile {
    pub decision: V4RuntimeDecision,

    pub elapsed_ns: f64,

    pub prediction: PredictionSample,
}

pub type V4RuntimeExecutionResult<B> = Result<
    (Vec<<B as FheBackend>::Ciphertext>, V4RuntimeDecision),
    V4RuntimeError<<B as FheBackend>::Error>,
>;

pub type V4ProfiledExecutionResult<B> = Result<
    (Vec<<B as FheBackend>::Ciphertext>, V4ExecutionProfile),
    V4RuntimeError<<B as FheBackend>::Error>,
>;

pub fn choose_execution_strategy_v4(
    circuit: &Circuit,
    outputs: &[WireId],
    model: &V4CostModel,
) -> Result<V4RuntimeDecision, AnalysisError> {
    let analysis = analyze_v4(circuit, outputs, model)?;

    Ok(V4RuntimeDecision {
        strategy: analysis.recommended_strategy,

        analysis,
    })
}

pub fn execute_auto_v4<B>(
    backend: &B,
    circuit: &Circuit,
    inputs: &[B::Ciphertext],
    outputs: &[WireId],
    model: &V4CostModel,
) -> V4RuntimeExecutionResult<B>
where
    B: FheBackend + Sync,
    B::Ciphertext: Clone + Send + Sync,
    B::Error: Send,
{
    let decision =
        choose_execution_strategy_v4(circuit, outputs, model).map_err(V4RuntimeError::Analysis)?;

    let result = match decision.strategy {
        V4ExecutionStrategy::Sequential => execute_circuit(backend, circuit, inputs, outputs),

        V4ExecutionStrategy::ChunkedParallel => {
            execute_circuit_chunked_parallel(backend, circuit, inputs, outputs, model.chunk_size)
        }

        V4ExecutionStrategy::Parallel => {
            execute_circuit_parallel(backend, circuit, inputs, outputs)
        }
    }
    .map_err(V4RuntimeError::Execution)?;

    Ok((result, decision))
}

pub fn execute_auto_v4_profiled<B>(
    backend: &B,
    circuit: &Circuit,
    inputs: &[B::Ciphertext],
    outputs: &[WireId],
    model: &V4CostModel,
    tracker: &mut PredictionTracker,
) -> V4ProfiledExecutionResult<B>
where
    B: FheBackend + Sync,
    B::Ciphertext: Clone + Send + Sync,
    B::Error: Send,
{
    let decision =
        choose_execution_strategy_v4(circuit, outputs, model).map_err(V4RuntimeError::Analysis)?;

    let predicted_ns = decision.analysis.selected_ns();

    let start = Instant::now();

    let result = match decision.strategy {
        V4ExecutionStrategy::Sequential => execute_circuit(backend, circuit, inputs, outputs),

        V4ExecutionStrategy::ChunkedParallel => {
            execute_circuit_chunked_parallel(backend, circuit, inputs, outputs, model.chunk_size)
        }

        V4ExecutionStrategy::Parallel => {
            execute_circuit_parallel(backend, circuit, inputs, outputs)
        }
    }
    .map_err(V4RuntimeError::Execution)?;

    let elapsed_ns = start.elapsed().as_nanos() as f64;

    let prediction = tracker.record(predicted_ns, elapsed_ns);

    Ok((
        result,
        V4ExecutionProfile {
            decision,
            elapsed_ns,
            prediction,
        },
    ))
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
    fn v4_runtime_executes() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();

        let b = circuit.add_input();

        let output = circuit.add_xor(a, b);

        let inputs = [MockCiphertext(true), MockCiphertext(false)];

        let (result, decision) = execute_auto_v4(
            &MockBackend,
            &circuit,
            &inputs,
            &[output],
            &V4CostModel::default(),
        )
        .unwrap();

        assert_eq!(result, vec![MockCiphertext(true)]);

        assert_eq!(decision.strategy, V4ExecutionStrategy::Sequential);
    }
}

#[cfg(test)]
#[path = "v4_strategy_tests.rs"]
mod strategy_tests;
