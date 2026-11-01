use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WireId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GateId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constant {
    False,
    True,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    Constant {
        id: GateId,
        wire: WireId,
        value: Constant,
    },
    Input {
        id: GateId,
        wire: WireId,
    },
    Not {
        id: GateId,
        wire: WireId,
        operand: WireId,
    },
    And {
        id: GateId,
        wire: WireId,
        lhs: WireId,
        rhs: WireId,
    },
    Or {
        id: GateId,
        wire: WireId,
        lhs: WireId,
        rhs: WireId,
    },
    Xor {
        id: GateId,
        wire: WireId,
        lhs: WireId,
        rhs: WireId,
    },
    Mux {
        id: GateId,
        wire: WireId,
        condition: WireId,
        if_true: WireId,
        if_false: WireId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
    InvalidGateId { expected: GateId, actual: GateId },
    InvalidWireId { expected: WireId, actual: WireId },
    DuplicateWire(WireId),
    MissingWire(WireId),
    InvalidGateReference { gate: GateId, wire: WireId },
    ExpectedInputWire(WireId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    InvalidInputCount { expected: usize, actual: usize },
    InvalidInputWire(WireId),
    InvalidOutputWire(WireId),
}

#[derive(Debug, Default, Clone)]
pub struct Circuit {
    gates: Vec<Gate>,
    next_gate_id: u32,
    next_wire_id: u32,
}

impl Circuit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_input(&mut self) -> WireId {
        let id = self.next_gate();
        let wire = self.next_wire();

        self.gates.push(Gate::Input { id, wire });

        wire
    }

    pub fn add_constant(&mut self, value: Constant) -> WireId {
        let id = self.next_gate();
        let wire = self.next_wire();

        self.gates.push(Gate::Constant { id, wire, value });

        wire
    }

    pub fn add_not(&mut self, operand: WireId) -> WireId {
        let id = self.next_gate();
        let wire = self.next_wire();

        self.gates.push(Gate::Not { id, wire, operand });

        wire
    }

    pub fn add_and(&mut self, lhs: WireId, rhs: WireId) -> WireId {
        let id = self.next_gate();
        let wire = self.next_wire();

        self.gates.push(Gate::And { id, wire, lhs, rhs });

        wire
    }

    pub fn add_or(&mut self, lhs: WireId, rhs: WireId) -> WireId {
        let id = self.next_gate();
        let wire = self.next_wire();

        self.gates.push(Gate::Or { id, wire, lhs, rhs });

        wire
    }

    pub fn add_xor(&mut self, lhs: WireId, rhs: WireId) -> WireId {
        let id = self.next_gate();
        let wire = self.next_wire();

        self.gates.push(Gate::Xor { id, wire, lhs, rhs });

        wire
    }

    pub fn add_mux(&mut self, condition: WireId, if_true: WireId, if_false: WireId) -> WireId {
        let id = self.next_gate();
        let wire = self.next_wire();

        self.gates.push(Gate::Mux {
            id,
            wire,
            condition,
            if_true,
            if_false,
        });

        wire
    }

    pub fn gates(&self) -> &[Gate] {
        &self.gates
    }

    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    pub fn wire_count(&self) -> usize {
        self.next_wire_id as usize
    }

    pub fn contains_wire(&self, wire: WireId) -> bool {
        wire.0 < self.next_wire_id
    }

    pub fn is_input_wire(&self, wire: WireId) -> bool {
        self.gates
            .iter()
            .any(|gate| matches!(gate, Gate::Input { wire: candidate, .. } if *candidate == wire))
    }

    pub fn verify(&self) -> Result<(), VerificationError> {
        let mut defined_wires = vec![false; self.next_wire_id as usize];

        for (index, gate) in self.gates.iter().enumerate() {
            let expected_gate_id = GateId(index as u32);
            let actual_gate_id = gate.id();

            if actual_gate_id != expected_gate_id {
                return Err(VerificationError::InvalidGateId {
                    expected: expected_gate_id,
                    actual: actual_gate_id,
                });
            }

            let expected_wire_id = WireId(index as u32);
            let actual_wire_id = gate.wire();

            if actual_wire_id != expected_wire_id {
                return Err(VerificationError::InvalidWireId {
                    expected: expected_wire_id,
                    actual: actual_wire_id,
                });
            }

            let wire_index = actual_wire_id.0 as usize;

            if defined_wires[wire_index] {
                return Err(VerificationError::DuplicateWire(actual_wire_id));
            }

            defined_wires[wire_index] = true;

            for operand in gate.operands() {
                if operand.0 as usize >= defined_wires.len() || !defined_wires[operand.0 as usize] {
                    return Err(VerificationError::InvalidGateReference {
                        gate: actual_gate_id,
                        wire: operand,
                    });
                }
            }
        }

        Ok(())
    }

    pub fn evaluate(
        &self,
        inputs: &[bool],
        outputs: &[WireId],
    ) -> Result<Vec<bool>, EvaluationError> {
        let input_wires: Vec<WireId> = self
            .gates
            .iter()
            .filter_map(|gate| match gate {
                Gate::Input { wire, .. } => Some(*wire),
                _ => None,
            })
            .collect();

        if inputs.len() != input_wires.len() {
            return Err(EvaluationError::InvalidInputCount {
                expected: input_wires.len(),
                actual: inputs.len(),
            });
        }

        for wire in outputs {
            if !self.contains_wire(*wire) {
                return Err(EvaluationError::InvalidOutputWire(*wire));
            }
        }

        let mut values = vec![false; self.next_wire_id as usize];

        for (index, gate) in self.gates.iter().enumerate() {
            let wire = WireId(index as u32);

            match gate {
                Gate::Constant { value, .. } => {
                    values[wire.0 as usize] = matches!(value, Constant::True);
                }

                Gate::Input { .. } => {
                    let input_index = input_wires
                        .iter()
                        .position(|candidate| *candidate == wire)
                        .unwrap();

                    values[wire.0 as usize] = inputs[input_index];
                }

                Gate::Not { operand, .. } => {
                    values[wire.0 as usize] = !values[operand.0 as usize];
                }

                Gate::And { lhs, rhs, .. } => {
                    values[wire.0 as usize] = values[lhs.0 as usize] & values[rhs.0 as usize];
                }

                Gate::Or { lhs, rhs, .. } => {
                    values[wire.0 as usize] = values[lhs.0 as usize] | values[rhs.0 as usize];
                }

                Gate::Xor { lhs, rhs, .. } => {
                    values[wire.0 as usize] = values[lhs.0 as usize] ^ values[rhs.0 as usize];
                }

                Gate::Mux {
                    condition,
                    if_true,
                    if_false,
                    ..
                } => {
                    values[wire.0 as usize] = if values[condition.0 as usize] {
                        values[if_true.0 as usize]
                    } else {
                        values[if_false.0 as usize]
                    };
                }
            }
        }

        Ok(outputs.iter().map(|wire| values[wire.0 as usize]).collect())
    }

    fn next_gate(&mut self) -> GateId {
        let id = GateId(self.next_gate_id);
        self.next_gate_id += 1;
        id
    }

    fn next_wire(&mut self) -> WireId {
        let id = WireId(self.next_wire_id);
        self.next_wire_id += 1;
        id
    }
}

impl Gate {
    fn id(&self) -> GateId {
        match self {
            Self::Constant { id, .. }
            | Self::Input { id, .. }
            | Self::Not { id, .. }
            | Self::And { id, .. }
            | Self::Or { id, .. }
            | Self::Xor { id, .. }
            | Self::Mux { id, .. } => *id,
        }
    }

    fn wire(&self) -> WireId {
        match self {
            Self::Constant { wire, .. }
            | Self::Input { wire, .. }
            | Self::Not { wire, .. }
            | Self::And { wire, .. }
            | Self::Or { wire, .. }
            | Self::Xor { wire, .. }
            | Self::Mux { wire, .. } => *wire,
        }
    }

    fn operands(&self) -> Vec<WireId> {
        match self {
            Self::Constant { .. } | Self::Input { .. } => Vec::new(),

            Self::Not { operand, .. } => vec![*operand],

            Self::And { lhs, rhs, .. } | Self::Or { lhs, rhs, .. } | Self::Xor { lhs, rhs, .. } => {
                vec![*lhs, *rhs]
            }

            Self::Mux {
                condition,
                if_true,
                if_false,
                ..
            } => vec![*condition, *if_true, *if_false],
        }
    }
}

impl fmt::Display for WireId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "w{}", self.0)
    }
}

impl fmt::Display for GateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "g{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_circuit_is_valid() {
        let circuit = Circuit::new();

        assert_eq!(circuit.gate_count(), 0);
        assert_eq!(circuit.wire_count(), 0);
        assert_eq!(circuit.verify(), Ok(()));
    }

    #[test]
    fn input_creates_wire() {
        let mut circuit = Circuit::new();

        let wire = circuit.add_input();

        assert_eq!(wire, WireId(0));
        assert_eq!(circuit.gate_count(), 1);
        assert_eq!(circuit.wire_count(), 1);
        assert!(circuit.contains_wire(wire));
        assert!(circuit.is_input_wire(wire));
    }

    #[test]
    fn constants_create_wires() {
        let mut circuit = Circuit::new();

        let false_wire = circuit.add_constant(Constant::False);
        let true_wire = circuit.add_constant(Constant::True);

        assert_ne!(false_wire, true_wire);
        assert_eq!(circuit.gate_count(), 2);
        assert_eq!(circuit.wire_count(), 2);
        assert!(!circuit.is_input_wire(false_wire));
        assert!(!circuit.is_input_wire(true_wire));
    }

    #[test]
    fn mux_creates_gate() {
        let mut circuit = Circuit::new();

        let condition = circuit.add_input();
        let if_true = circuit.add_input();
        let if_false = circuit.add_input();
        let result = circuit.add_mux(condition, if_true, if_false);

        assert_eq!(result, WireId(3));
        assert_eq!(circuit.gate_count(), 4);
    }

    #[test]
    fn boolean_chain_creates_expected_gates() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();
        let b = circuit.add_input();
        let x = circuit.add_xor(a, b);
        let n = circuit.add_not(x);
        let y = circuit.add_and(n, a);

        assert_eq!(circuit.gate_count(), 5);
        assert_eq!(y, WireId(4));
    }

    #[test]
    fn valid_boolean_circuit_verifies() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();
        let b = circuit.add_input();
        let x = circuit.add_xor(a, b);
        let y = circuit.add_not(x);

        assert_eq!(circuit.verify(), Ok(()));
        assert_eq!(y, WireId(3));
    }

    #[test]
    fn valid_mux_verifies() {
        let mut circuit = Circuit::new();

        let condition = circuit.add_input();
        let if_true = circuit.add_input();
        let if_false = circuit.add_input();

        circuit.add_mux(condition, if_true, if_false);

        assert_eq!(circuit.verify(), Ok(()));
    }

    #[test]
    fn verifier_rejects_invalid_gate_reference() {
        let mut circuit = Circuit::new();

        let invalid_wire = WireId(99);

        circuit.gates.push(Gate::Not {
            id: GateId(0),
            wire: WireId(0),
            operand: invalid_wire,
        });
        circuit.next_gate_id = 1;
        circuit.next_wire_id = 1;

        assert_eq!(
            circuit.verify(),
            Err(VerificationError::InvalidGateReference {
                gate: GateId(0),
                wire: invalid_wire,
            })
        );
    }

    #[test]
    fn verifier_accepts_constant_chain() {
        let mut circuit = Circuit::new();

        let constant = circuit.add_constant(Constant::True);
        let inverted = circuit.add_not(constant);
        circuit.add_and(constant, inverted);

        assert_eq!(circuit.verify(), Ok(()));
    }

    #[test]
    fn evaluator_handles_not() {
        let mut circuit = Circuit::new();

        let input = circuit.add_input();
        let output = circuit.add_not(input);

        assert_eq!(circuit.evaluate(&[false], &[output]), Ok(vec![true]));
        assert_eq!(circuit.evaluate(&[true], &[output]), Ok(vec![false]));
    }

    #[test]
    fn evaluator_handles_boolean_gates() {
        let mut circuit = Circuit::new();

        let a = circuit.add_input();
        let b = circuit.add_input();
        let and = circuit.add_and(a, b);
        let or = circuit.add_or(a, b);
        let xor = circuit.add_xor(a, b);

        assert_eq!(
            circuit.evaluate(&[false, false], &[and, or, xor]),
            Ok(vec![false, false, false])
        );

        assert_eq!(
            circuit.evaluate(&[false, true], &[and, or, xor]),
            Ok(vec![false, true, true])
        );

        assert_eq!(
            circuit.evaluate(&[true, true], &[and, or, xor]),
            Ok(vec![true, true, false])
        );
    }

    #[test]
    fn evaluator_handles_mux() {
        let mut circuit = Circuit::new();

        let condition = circuit.add_input();
        let if_true = circuit.add_input();
        let if_false = circuit.add_input();
        let output = circuit.add_mux(condition, if_true, if_false);

        assert_eq!(
            circuit.evaluate(&[true, true, false], &[output]),
            Ok(vec![true])
        );

        assert_eq!(
            circuit.evaluate(&[false, true, false], &[output]),
            Ok(vec![false])
        );
    }

    #[test]
    fn evaluator_handles_constants() {
        let mut circuit = Circuit::new();

        let false_wire = circuit.add_constant(Constant::False);
        let true_wire = circuit.add_constant(Constant::True);

        assert_eq!(
            circuit.evaluate(&[], &[false_wire, true_wire]),
            Ok(vec![false, true])
        );
    }

    #[test]
    fn evaluator_rejects_wrong_input_count() {
        let mut circuit = Circuit::new();

        circuit.add_input();
        circuit.add_input();

        assert_eq!(
            circuit.evaluate(&[true], &[]),
            Err(EvaluationError::InvalidInputCount {
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn evaluator_rejects_invalid_output_wire() {
        let circuit = Circuit::new();

        assert_eq!(
            circuit.evaluate(&[], &[WireId(99)]),
            Err(EvaluationError::InvalidOutputWire(WireId(99)))
        );
    }
}
