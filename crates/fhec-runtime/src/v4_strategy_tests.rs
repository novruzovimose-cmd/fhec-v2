use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestCiphertext(bool);

struct TestBackend;

impl FheBackend for TestBackend {
    type Ciphertext = TestCiphertext;
    type Error = ();

    fn encrypt_bool(&self, value: bool) -> Result<Self::Ciphertext, Self::Error> {
        Ok(TestCiphertext(value))
    }

    fn decrypt_bool(&self, value: &Self::Ciphertext) -> Result<bool, Self::Error> {
        Ok(value.0)
    }

    fn not(&self, value: &Self::Ciphertext) -> Result<Self::Ciphertext, Self::Error> {
        Ok(TestCiphertext(!value.0))
    }

    fn and(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(TestCiphertext(lhs.0 && rhs.0))
    }

    fn or(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(TestCiphertext(lhs.0 || rhs.0))
    }

    fn xor(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(TestCiphertext(lhs.0 ^ rhs.0))
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

fn assert_strategies_agree(circuit: &Circuit, outputs: &[WireId], input_count: usize) {
    let backend = TestBackend;

    for case in 0..32u64 {
        let inputs = (0..input_count)
            .map(|index| {
                let mixed = case
                    .wrapping_mul(0x9E3779B97F4A7C15)
                    .rotate_left((index % 63) as u32)
                    ^ (index as u64).wrapping_mul(0xD1B54A32D192ED03);
                TestCiphertext(mixed.count_ones() % 2 == 1)
            })
            .collect::<Vec<_>>();

        let sequential = execute_circuit(&backend, circuit, &inputs, outputs).unwrap();
        let parallel = execute_circuit_parallel(&backend, circuit, &inputs, outputs).unwrap();
        assert_eq!(sequential, parallel, "parallel mismatch in case {case}");

        for chunk_size in [1, 2, 3, 8, 32] {
            let chunked =
                execute_circuit_chunked_parallel(&backend, circuit, &inputs, outputs, chunk_size)
                    .unwrap();
            assert_eq!(
                sequential, chunked,
                "chunked mismatch in case {case} with chunk size {chunk_size}"
            );
        }
    }
}

#[test]
fn strategies_agree_on_dependent_logic_and_mux() {
    let mut circuit = Circuit::new();
    let a = circuit.add_input();
    let b = circuit.add_input();
    let condition = circuit.add_input();
    let xor = circuit.add_xor(a, b);
    let inverted = circuit.add_not(xor);
    let and = circuit.add_and(inverted, a);
    let or = circuit.add_or(xor, b);
    let mux = circuit.add_mux(condition, and, or);
    let result = circuit.add_xor(mux, inverted);

    assert_strategies_agree(&circuit, &[result, mux, and, or], 3);
}

#[test]
fn strategies_agree_on_wide_and_deep_circuits() {
    let mut circuit = Circuit::new();
    let mut layer = Vec::new();

    for index in 0..24 {
        let a = circuit.add_input();
        let b = circuit.add_input();
        layer.push(if index % 3 == 0 {
            circuit.add_and(a, b)
        } else if index % 3 == 1 {
            circuit.add_or(a, b)
        } else {
            circuit.add_xor(a, b)
        });
    }

    let wide_outputs = layer.clone();
    while layer.len() > 1 {
        let mut next = Vec::new();
        for pair in layer.chunks(2) {
            if pair.len() == 2 {
                next.push(circuit.add_xor(pair[0], pair[1]));
            } else {
                next.push(pair[0]);
            }
        }
        layer = next;
    }

    let mut outputs = wide_outputs;
    outputs.push(layer[0]);
    assert_strategies_agree(&circuit, &outputs, 48);
}

#[test]
fn strategies_agree_on_long_dependency_chain() {
    let mut circuit = Circuit::new();
    let a = circuit.add_input();
    let b = circuit.add_input();
    let mut current = a;

    for index in 0..64 {
        current = if index % 2 == 0 {
            circuit.add_xor(current, b)
        } else {
            circuit.add_not(current)
        };
    }

    assert_strategies_agree(&circuit, &[current, a, b], 2);
}
