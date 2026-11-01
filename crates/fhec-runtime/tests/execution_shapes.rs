use fhec_analyzer::v4::V4CostModel;
use fhec_backend::{
    FheBackend, execute_circuit, execute_circuit_chunked_parallel, execute_circuit_parallel,
};
use fhec_circuit::{Circuit, WireId};
use fhec_runtime::v4::execute_auto_v4;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Cipher(bool);

struct MockBackend;

impl FheBackend for MockBackend {
    type Ciphertext = Cipher;
    type Error = ();

    fn encrypt_bool(&self, value: bool) -> Result<Self::Ciphertext, Self::Error> {
        Ok(Cipher(value))
    }

    fn decrypt_bool(&self, value: &Self::Ciphertext) -> Result<bool, Self::Error> {
        Ok(value.0)
    }

    fn not(&self, value: &Self::Ciphertext) -> Result<Self::Ciphertext, Self::Error> {
        Ok(Cipher(!value.0))
    }

    fn and(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(Cipher(lhs.0 && rhs.0))
    }

    fn or(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(Cipher(lhs.0 || rhs.0))
    }

    fn xor(
        &self,
        lhs: &Self::Ciphertext,
        rhs: &Self::Ciphertext,
    ) -> Result<Self::Ciphertext, Self::Error> {
        Ok(Cipher(lhs.0 ^ rhs.0))
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

fn check_all_strategies(
    circuit: &Circuit,
    inputs: &[Cipher],
    outputs: &[WireId],
    expected: &[Cipher],
) {
    let backend = MockBackend;

    let sequential = execute_circuit(&backend, circuit, inputs, outputs).unwrap();
    let parallel = execute_circuit_parallel(&backend, circuit, inputs, outputs).unwrap();
    let chunked = execute_circuit_chunked_parallel(&backend, circuit, inputs, outputs, 3).unwrap();
    let (automatic, _) =
        execute_auto_v4(&backend, circuit, inputs, outputs, &V4CostModel::default()).unwrap();

    assert_eq!(sequential, expected);
    assert_eq!(parallel, expected);
    assert_eq!(chunked, expected);
    assert_eq!(automatic, expected);
}

fn bits(value: u16, width: usize) -> Vec<bool> {
    (0..width)
        .map(|index| ((value >> index) & 1) != 0)
        .collect()
}

#[test]
fn wide_and_deep_shape_consistency() {
    let mut circuit = Circuit::new();
    let inputs = bits(37, 8)
        .into_iter()
        .chain(std::iter::once(true))
        .chain(bits((-7i8) as u8 as u16, 8))
        .chain(bits(50_000, 16))
        .map(Cipher)
        .collect::<Vec<_>>();

    assert_eq!(inputs.len(), 33);

    let mut outputs = Vec::new();
    for _ in &inputs {
        let input = circuit.add_input();
        let once = circuit.add_not(input);
        let twice = circuit.add_not(once);
        outputs.push(twice);
    }

    check_all_strategies(&circuit, &inputs, &outputs, &inputs);

    let mut narrow = Circuit::new();
    let input = narrow.add_input();
    let mut output = input;
    for _ in 0..97 {
        output = narrow.add_not(output);
    }
    check_all_strategies(&narrow, &[Cipher(true)], &[output], &[Cipher(false)]);
}
