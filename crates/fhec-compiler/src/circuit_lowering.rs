use std::collections::HashMap;

use fhec_analyzer::attribution::AttributionRegion;

use fhec_circuit::{Circuit, Constant as CircuitConstant, WireId};
use fhec_ir::{Constant, Function, Operation, Type, ValueId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitValue {
    pub bits: Vec<WireId>,
}

impl CircuitValue {
    pub fn width(&self) -> usize {
        self.bits.len()
    }
}

#[derive(Debug)]
pub struct CircuitFunction {
    pub circuit: Circuit,
    pub inputs: HashMap<ValueId, CircuitValue>,
    pub outputs: Vec<WireId>,
    pub attribution_regions: Vec<AttributionRegion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitFunctionVerificationError {
    Circuit(fhec_circuit::VerificationError),
    MissingInput(ValueId),
    UnexpectedInput(ValueId),
    InputWidth {
        value: ValueId,
        expected: usize,
        actual: usize,
    },
    InvalidInputWire(WireId),
    InvalidOutputWire(WireId),
    OutputWidth {
        expected: usize,
        actual: usize,
    },
}

impl CircuitFunction {
    pub fn verify(&self, function: &Function) -> Result<(), CircuitFunctionVerificationError> {
        self.circuit
            .verify()
            .map_err(CircuitFunctionVerificationError::Circuit)?;

        for parameter in &function.parameters {
            let value = self
                .inputs
                .get(&parameter.id)
                .ok_or(CircuitFunctionVerificationError::MissingInput(parameter.id))?;

            let expected = type_width(&parameter.ty);
            let actual = value.width();

            if expected != actual {
                return Err(CircuitFunctionVerificationError::InputWidth {
                    value: parameter.id,
                    expected,
                    actual,
                });
            }

            for wire in &value.bits {
                if !self.circuit.contains_wire(*wire) {
                    return Err(CircuitFunctionVerificationError::InvalidInputWire(*wire));
                }

                if !self.circuit.is_input_wire(*wire) {
                    return Err(CircuitFunctionVerificationError::InvalidInputWire(*wire));
                }
            }
        }

        for value_id in self.inputs.keys() {
            if !function
                .parameters
                .iter()
                .any(|parameter| parameter.id == *value_id)
            {
                return Err(CircuitFunctionVerificationError::UnexpectedInput(*value_id));
            }
        }

        for wire in &self.outputs {
            if !self.circuit.contains_wire(*wire) {
                return Err(CircuitFunctionVerificationError::InvalidOutputWire(*wire));
            }
        }

        let expected_output_width = type_width(&function.return_type);
        let actual_output_width = self.outputs.len();

        if expected_output_width != actual_output_width {
            return Err(CircuitFunctionVerificationError::OutputWidth {
                expected: expected_output_width,
                actual: actual_output_width,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdderStrategy {
    ParallelPrefix,
    RippleCarry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitLoweringError {
    MissingOperand(ValueId),
    DuplicateValue(ValueId),
}

pub fn try_lower_function_to_circuit(
    function: &Function,
) -> Result<CircuitFunction, CircuitLoweringError> {
    validate_circuit_operands(function)?;
    Ok(lower_function_to_circuit(function))
}

pub(crate) fn validate_circuit_operands(function: &Function) -> Result<(), CircuitLoweringError> {
    let mut available = std::collections::HashSet::new();

    for parameter in &function.parameters {
        if !available.insert(parameter.id) {
            return Err(CircuitLoweringError::DuplicateValue(parameter.id));
        }
    }

    for operation in &function.operations {
        let operands: Vec<ValueId> = match operation {
            Operation::Constant { .. } | Operation::Argument { .. } => Vec::new(),
            Operation::Copy { operand, .. } | Operation::Not { operand, .. } => vec![*operand],
            Operation::Add { lhs, rhs, .. }
            | Operation::Sub { lhs, rhs, .. }
            | Operation::Mul { lhs, rhs, .. }
            | Operation::Eq { lhs, rhs, .. }
            | Operation::NotEqual { lhs, rhs, .. }
            | Operation::GreaterThan { lhs, rhs, .. }
            | Operation::GreaterEqual { lhs, rhs, .. }
            | Operation::LessThan { lhs, rhs, .. }
            | Operation::LessEqual { lhs, rhs, .. }
            | Operation::And { lhs, rhs, .. }
            | Operation::Or { lhs, rhs, .. }
            | Operation::Xor { lhs, rhs, .. } => vec![*lhs, *rhs],
            Operation::Mux {
                condition,
                if_true,
                if_false,
                ..
            } => vec![*condition, *if_true, *if_false],
            Operation::Return { value } => value.iter().copied().collect(),
        };

        for operand in operands {
            if !available.contains(&operand) {
                return Err(CircuitLoweringError::MissingOperand(operand));
            }
        }

        match operation {
            Operation::Constant { result, .. }
            | Operation::Copy { result, .. }
            | Operation::Add { result, .. }
            | Operation::Sub { result, .. }
            | Operation::Mul { result, .. }
            | Operation::Eq { result, .. }
            | Operation::NotEqual { result, .. }
            | Operation::GreaterThan { result, .. }
            | Operation::GreaterEqual { result, .. }
            | Operation::LessThan { result, .. }
            | Operation::LessEqual { result, .. }
            | Operation::Not { result, .. }
            | Operation::And { result, .. }
            | Operation::Or { result, .. }
            | Operation::Xor { result, .. }
            | Operation::Mux { result, .. }
            | Operation::Argument { result, .. } => {
                if !available.insert(*result) {
                    return Err(CircuitLoweringError::DuplicateValue(*result));
                }
            }
            Operation::Return { .. } => {}
        }
    }

    Ok(())
}

pub fn lower_function_to_circuit(function: &Function) -> CircuitFunction {
    lower_function_to_circuit_with_adder(function, AdderStrategy::ParallelPrefix)
}

pub(crate) fn lower_function_to_circuit_with_adder(
    function: &Function,
    adder_strategy: AdderStrategy,
) -> CircuitFunction {
    let mut circuit = Circuit::new();
    let mut values = HashMap::new();

    for parameter in &function.parameters {
        let width = type_width(&parameter.ty);
        let bits = (0..width).map(|_| circuit.add_input()).collect();

        values.insert(parameter.id, CircuitValue { bits });
    }

    let mut outputs = Vec::new();
    let mut attribution_regions = Vec::new();

    for (operation_index, operation) in function.operations.iter().enumerate() {
        let gate_start = circuit.gate_count();

        match operation {
            Operation::Constant { result, value } => {
                let value = lower_constant(&mut circuit, value);

                values.insert(*result, value);
            }

            Operation::Copy { result, operand } => {
                let value = values.get(operand).cloned().expect("missing Copy operand");

                values.insert(*result, value);
            }

            Operation::Add { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);
                let width = function
                    .value_type(*result)
                    .map(type_width)
                    .unwrap_or_else(|| lhs.width().max(rhs.width()));

                let value = match adder_strategy {
                    AdderStrategy::ParallelPrefix => {
                        add_bits(&mut circuit, &lhs.bits, &rhs.bits, width)
                    }

                    AdderStrategy::RippleCarry => {
                        add_bits_ripple(&mut circuit, &lhs.bits, &rhs.bits, width)
                    }
                };

                values.insert(*result, value);
            }

            Operation::Sub { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);
                let width = function
                    .value_type(*result)
                    .map(type_width)
                    .unwrap_or_else(|| lhs.width().max(rhs.width()));

                let value = sub_bits(&mut circuit, &lhs.bits, &rhs.bits, width);

                values.insert(*result, value);
            }

            Operation::Mul { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);
                let width = function
                    .value_type(*result)
                    .map(type_width)
                    .unwrap_or_else(|| lhs.width().max(rhs.width()));

                let value = mul_bits(&mut circuit, &lhs.bits, &rhs.bits, width);

                values.insert(*result, value);
            }

            Operation::Eq { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);

                let value = equal_bits(&mut circuit, &lhs.bits, &rhs.bits);

                values.insert(*result, value);
            }

            Operation::NotEqual { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);

                let equal = equal_bits(&mut circuit, &lhs.bits, &rhs.bits);

                let bit = circuit.add_not(equal.bits[0]);

                values.insert(*result, CircuitValue { bits: vec![bit] });
            }

            Operation::GreaterThan { result, lhs, rhs } => {
                let lhs_value = get_value(&values, *lhs);
                let rhs_value = get_value(&values, *rhs);

                let signed = function
                    .value_type(*lhs)
                    .map(is_signed_type)
                    .unwrap_or(false);

                let value = if signed {
                    greater_than_signed_bits(&mut circuit, &lhs_value.bits, &rhs_value.bits)
                } else {
                    greater_than_bits(&mut circuit, &lhs_value.bits, &rhs_value.bits)
                };

                values.insert(*result, value);
            }

            Operation::GreaterEqual { result, lhs, rhs } => {
                let lhs_value = get_value(&values, *lhs);
                let rhs_value = get_value(&values, *rhs);

                let signed = function
                    .value_type(*lhs)
                    .map(is_signed_type)
                    .unwrap_or(false);

                let greater = if signed {
                    greater_than_signed_bits(&mut circuit, &lhs_value.bits, &rhs_value.bits)
                } else {
                    greater_than_bits(&mut circuit, &lhs_value.bits, &rhs_value.bits)
                };

                let equal = equal_bits(&mut circuit, &lhs_value.bits, &rhs_value.bits);

                let bit = circuit.add_or(greater.bits[0], equal.bits[0]);

                values.insert(*result, CircuitValue { bits: vec![bit] });
            }

            Operation::LessThan { result, lhs, rhs } => {
                let lhs_value = get_value(&values, *lhs);
                let rhs_value = get_value(&values, *rhs);

                let signed = function
                    .value_type(*lhs)
                    .map(is_signed_type)
                    .unwrap_or(false);

                let value = if signed {
                    greater_than_signed_bits(&mut circuit, &rhs_value.bits, &lhs_value.bits)
                } else {
                    greater_than_bits(&mut circuit, &rhs_value.bits, &lhs_value.bits)
                };

                values.insert(*result, value);
            }

            Operation::LessEqual { result, lhs, rhs } => {
                let lhs_value = get_value(&values, *lhs);
                let rhs_value = get_value(&values, *rhs);

                let signed = function
                    .value_type(*lhs)
                    .map(is_signed_type)
                    .unwrap_or(false);

                let greater = if signed {
                    greater_than_signed_bits(&mut circuit, &lhs_value.bits, &rhs_value.bits)
                } else {
                    greater_than_bits(&mut circuit, &lhs_value.bits, &rhs_value.bits)
                };

                let bit = circuit.add_not(greater.bits[0]);

                values.insert(*result, CircuitValue { bits: vec![bit] });
            }

            Operation::Not { result, operand } => {
                let operand = get_value(&values, *operand);

                let bits = operand
                    .bits
                    .iter()
                    .map(|bit| circuit.add_not(*bit))
                    .collect();

                values.insert(*result, CircuitValue { bits });
            }

            Operation::And { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);

                let bits = lhs
                    .bits
                    .iter()
                    .zip(rhs.bits.iter())
                    .map(|(lhs, rhs)| circuit.add_and(*lhs, *rhs))
                    .collect();

                values.insert(*result, CircuitValue { bits });
            }

            Operation::Or { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);

                let bits = lhs
                    .bits
                    .iter()
                    .zip(rhs.bits.iter())
                    .map(|(lhs, rhs)| circuit.add_or(*lhs, *rhs))
                    .collect();

                values.insert(*result, CircuitValue { bits });
            }

            Operation::Xor { result, lhs, rhs } => {
                let lhs = get_value(&values, *lhs);
                let rhs = get_value(&values, *rhs);

                let bits = lhs
                    .bits
                    .iter()
                    .zip(rhs.bits.iter())
                    .map(|(lhs, rhs)| circuit.add_xor(*lhs, *rhs))
                    .collect();

                values.insert(*result, CircuitValue { bits });
            }

            Operation::Mux {
                result,
                condition,
                if_true,
                if_false,
            } => {
                let condition = get_value(&values, *condition);

                let if_true = get_value(&values, *if_true);

                let if_false = get_value(&values, *if_false);

                let bits = if_true
                    .bits
                    .iter()
                    .zip(if_false.bits.iter())
                    .map(|(if_true, if_false)| {
                        circuit.add_mux(condition.bits[0], *if_true, *if_false)
                    })
                    .collect();

                values.insert(*result, CircuitValue { bits });
            }

            Operation::Return { value } => {
                if let Some(value) = value {
                    let circuit_value = get_value(&values, *value);

                    outputs.extend(circuit_value.bits);
                }
            }

            Operation::Argument { result, ty } => {
                let width = type_width(ty);

                let bits = (0..width).map(|_| circuit.add_input()).collect();

                values.insert(*result, CircuitValue { bits });
            }
        }

        let gate_end = circuit.gate_count();

        if gate_end > gate_start {
            attribution_regions.push(AttributionRegion::new(
                operation_index,
                operation_kind(operation),
                gate_start,
                gate_end,
            ));
        }
    }

    CircuitFunction {
        circuit,
        inputs: values
            .iter()
            .filter_map(|(id, value)| {
                function
                    .parameters
                    .iter()
                    .find(|parameter| parameter.id == *id)
                    .map(|_| (*id, value.clone()))
            })
            .collect(),
        outputs,
        attribution_regions,
    }
}

fn lower_constant(circuit: &mut Circuit, constant: &Constant) -> CircuitValue {
    match constant {
        Constant::Bool(value) => {
            let wire = circuit.add_constant(if *value {
                CircuitConstant::True
            } else {
                CircuitConstant::False
            });

            CircuitValue { bits: vec![wire] }
        }

        Constant::UInt { value, width } => {
            let bits = (0..*width)
                .map(|index| {
                    circuit.add_constant(if ((*value >> index) & 1) == 1 {
                        CircuitConstant::True
                    } else {
                        CircuitConstant::False
                    })
                })
                .collect();

            CircuitValue { bits }
        }

        Constant::Int { value, width } => {
            let bits = (0..*width)
                .map(|index| {
                    circuit.add_constant(if ((*value >> index) & 1) != 0 {
                        CircuitConstant::True
                    } else {
                        CircuitConstant::False
                    })
                })
                .collect();

            CircuitValue { bits }
        }
    }
}

fn add_bits_ripple(
    circuit: &mut Circuit,
    lhs: &[WireId],
    rhs: &[WireId],
    width: usize,
) -> CircuitValue {
    let false_wire = circuit.add_constant(CircuitConstant::False);

    if width == 0 {
        return CircuitValue { bits: Vec::new() };
    }

    let mut carry = false_wire;

    let mut result = Vec::with_capacity(width);

    for index in 0..width {
        let a = lhs.get(index).copied().unwrap_or(false_wire);

        let b = rhs.get(index).copied().unwrap_or(false_wire);

        let propagate = circuit.add_xor(a, b);

        let sum = circuit.add_xor(propagate, carry);

        let generate = circuit.add_and(a, b);

        let propagated_carry = circuit.add_and(propagate, carry);

        carry = circuit.add_or(generate, propagated_carry);

        result.push(sum);
    }

    CircuitValue { bits: result }
}
fn add_bits(circuit: &mut Circuit, lhs: &[WireId], rhs: &[WireId], width: usize) -> CircuitValue {
    let false_wire = circuit.add_constant(CircuitConstant::False);

    if width == 0 {
        return CircuitValue { bits: Vec::new() };
    }

    let mut original_propagate = Vec::with_capacity(width);

    let mut propagate = Vec::with_capacity(width);

    let mut generate = Vec::with_capacity(width);

    for index in 0..width {
        let a = lhs.get(index).copied().unwrap_or(false_wire);

        let b = rhs.get(index).copied().unwrap_or(false_wire);

        let p = circuit.add_xor(a, b);
        let g = circuit.add_and(a, b);

        original_propagate.push(p);
        propagate.push(p);
        generate.push(g);
    }

    let mut distance = 1;

    while distance < width {
        let previous_propagate = propagate.clone();

        let previous_generate = generate.clone();

        for index in distance..width {
            let propagated_generate = circuit.add_and(
                previous_propagate[index],
                previous_generate[index - distance],
            );

            generate[index] = circuit.add_or(previous_generate[index], propagated_generate);

            propagate[index] = circuit.add_and(
                previous_propagate[index],
                previous_propagate[index - distance],
            );
        }

        distance *= 2;
    }

    let mut result = Vec::with_capacity(width);

    for index in 0..width {
        let carry = if index == 0 {
            false_wire
        } else {
            generate[index - 1]
        };

        let sum = circuit.add_xor(original_propagate[index], carry);

        result.push(sum);
    }

    CircuitValue { bits: result }
}

fn sub_bits(circuit: &mut Circuit, lhs: &[WireId], rhs: &[WireId], width: usize) -> CircuitValue {
    let false_wire = circuit.add_constant(CircuitConstant::False);

    let mut borrow = false_wire;
    let mut result = Vec::with_capacity(width);

    for index in 0..width {
        let a = lhs.get(index).copied().unwrap_or(false_wire);

        let b = rhs.get(index).copied().unwrap_or(false_wire);

        let axb = circuit.add_xor(a, b);
        let difference = circuit.add_xor(axb, borrow);

        let not_a = circuit.add_not(a);

        let borrow_from_ab = circuit.add_and(not_a, b);

        let not_axb = circuit.add_not(axb);

        let borrow_from_sub = circuit.add_and(not_axb, borrow);

        borrow = circuit.add_or(borrow_from_ab, borrow_from_sub);

        result.push(difference);
    }

    CircuitValue { bits: result }
}

fn mul_bits(circuit: &mut Circuit, lhs: &[WireId], rhs: &[WireId], width: usize) -> CircuitValue {
    let false_wire = circuit.add_constant(CircuitConstant::False);

    let mut result = vec![false_wire; width];

    for (i, &rhs_bit) in rhs.iter().enumerate() {
        let mut partial = vec![false_wire; width];

        for (j, &lhs_bit) in lhs.iter().enumerate() {
            let index = i + j;

            if index >= width {
                break;
            }

            partial[index] = circuit.add_and(lhs_bit, rhs_bit);
        }

        result = add_bits(circuit, &result, &partial, width).bits;
    }

    CircuitValue { bits: result }
}

fn equal_bits(circuit: &mut Circuit, lhs: &[WireId], rhs: &[WireId]) -> CircuitValue {
    let width = lhs.len().max(rhs.len());

    let false_wire = circuit.add_constant(CircuitConstant::False);

    let true_wire = circuit.add_constant(CircuitConstant::True);

    let mut result = true_wire;

    for index in 0..width {
        let a = lhs.get(index).copied().unwrap_or(false_wire);

        let b = rhs.get(index).copied().unwrap_or(false_wire);

        let difference = circuit.add_xor(a, b);
        let same = circuit.add_not(difference);

        result = circuit.add_and(result, same);
    }

    CircuitValue { bits: vec![result] }
}

fn greater_than_bits(circuit: &mut Circuit, lhs: &[WireId], rhs: &[WireId]) -> CircuitValue {
    let width = lhs.len().max(rhs.len());

    let false_wire = circuit.add_constant(CircuitConstant::False);

    let mut greater = false_wire;

    let mut equal = circuit.add_constant(CircuitConstant::True);

    for index in (0..width).rev() {
        let a = lhs.get(index).copied().unwrap_or(false_wire);

        let b = rhs.get(index).copied().unwrap_or(false_wire);

        let not_b = circuit.add_not(b);

        let a_greater = circuit.add_and(a, not_b);

        let difference = circuit.add_xor(a, b);
        let same = circuit.add_not(difference);

        let candidate = circuit.add_and(equal, a_greater);

        greater = circuit.add_or(greater, candidate);

        equal = circuit.add_and(equal, same);
    }

    CircuitValue {
        bits: vec![greater],
    }
}

fn greater_than_signed_bits(circuit: &mut Circuit, lhs: &[WireId], rhs: &[WireId]) -> CircuitValue {
    let false_wire = circuit.add_constant(CircuitConstant::False);

    let lhs_sign = lhs.last().copied().unwrap_or(false_wire);

    let rhs_sign = rhs.last().copied().unwrap_or(false_wire);

    let signs_differ = circuit.add_xor(lhs_sign, rhs_sign);

    let lhs_not_negative = circuit.add_not(lhs_sign);

    let lhs_positive_rhs_negative = circuit.add_and(lhs_not_negative, rhs_sign);

    let unsigned_greater = greater_than_bits(circuit, lhs, rhs);

    let result = circuit.add_mux(
        signs_differ,
        lhs_positive_rhs_negative,
        unsigned_greater.bits[0],
    );

    CircuitValue { bits: vec![result] }
}

fn is_signed_type(ty: &Type) -> bool {
    match ty {
        Type::Int { .. } => true,

        Type::Encrypted(inner) | Type::Plaintext(inner) => is_signed_type(inner),

        _ => false,
    }
}
fn operation_kind(operation: &Operation) -> &'static str {
    match operation {
        Operation::Constant { .. } => "Constant",
        Operation::Copy { .. } => "Copy",
        Operation::Add { .. } => "Add",
        Operation::Sub { .. } => "Sub",
        Operation::Mul { .. } => "Mul",
        Operation::Eq { .. } => "Eq",
        Operation::NotEqual { .. } => "NotEqual",
        Operation::GreaterThan { .. } => "GreaterThan",
        Operation::GreaterEqual { .. } => "GreaterEqual",
        Operation::LessThan { .. } => "LessThan",
        Operation::LessEqual { .. } => "LessEqual",
        Operation::Not { .. } => "Not",
        Operation::And { .. } => "And",
        Operation::Or { .. } => "Or",
        Operation::Xor { .. } => "Xor",
        Operation::Mux { .. } => "Mux",
        Operation::Return { .. } => "Return",
        Operation::Argument { .. } => "Argument",
    }
}

fn get_value(values: &HashMap<ValueId, CircuitValue>, id: ValueId) -> CircuitValue {
    values
        .get(&id)
        .cloned()
        .unwrap_or_else(|| panic!("missing circuit value for {:?}", id))
}

fn type_width(ty: &Type) -> usize {
    match ty {
        Type::Bool => 1,
        Type::UInt { width } => *width as usize,
        Type::Int { width } => *width as usize,
        Type::Encrypted(inner) => type_width(inner),
        Type::Plaintext(inner) => type_width(inner),
        Type::Unit => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fhec_frontend::lexer::Lexer;
    use fhec_frontend::parser::Parser;
    use fhec_ir::Module;

    use crate::lowering::lower_program;

    #[test]
    fn lowers_encrypted_addition_to_circuit() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module: Module = lower_program(&program).expect("lowering failed");
        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 2);
        assert_eq!(result.outputs.len(), 8);
        assert!(result.circuit.gate_count() > 0);
        assert!(result.verify(function).is_ok());
    }

    fn bits_lsb(value: u64, width: usize) -> Vec<bool> {
        (0..width).map(|bit| ((value >> bit) & 1) == 1).collect()
    }

    fn decode_lsb(bits: &[bool]) -> u64 {
        bits.iter().enumerate().fold(
            0u64,
            |value, (bit, set)| {
                if *set { value | (1u64 << bit) } else { value }
            },
        )
    }

    #[test]
    fn lowers_encrypted_subtraction_to_circuit() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Sub { .. }))
        );

        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 2);
        assert_eq!(result.outputs.len(), 8);
        assert!(result.verify(function).is_ok());

        let region = result
            .attribution_regions
            .iter()
            .find(|region| region.operation_kind == "Sub")
            .expect("missing Sub attribution");

        assert!(region.gate_count() > 0);
    }

    #[test]
    fn subtraction_evaluates_correctly() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(13, 8);
        inputs.extend(bits_lsb(5, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("circuit evaluation failed");

        assert_eq!(decode_lsb(&outputs), 8);
    }

    #[test]
    fn subtraction_propagates_borrow() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(16, 8);
        inputs.extend(bits_lsb(1, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("circuit evaluation failed");

        assert_eq!(decode_lsb(&outputs), 15);
    }

    #[test]
    fn subtraction_wraps_at_result_width() {
        let source = r#"
            fn subtract(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(3, 8);
        inputs.extend(bits_lsb(5, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("circuit evaluation failed");

        assert_eq!(decode_lsb(&outputs), 254);
    }

    #[test]
    fn lowers_encrypted_multiplication_to_circuit() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Mul { .. }))
        );

        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 2);
        assert_eq!(result.outputs.len(), 8);
        assert!(result.verify(function).is_ok());

        let region = result
            .attribution_regions
            .iter()
            .find(|region| region.operation_kind == "Mul")
            .expect("missing Mul attribution");

        assert!(region.gate_count() > 0);
    }

    #[test]
    fn multiplication_evaluates_correctly() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(6, 8);
        inputs.extend(bits_lsb(7, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("circuit evaluation failed");

        assert_eq!(decode_lsb(&outputs), 42);
    }

    #[test]
    fn multiplication_by_zero_is_zero() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(173, 8);
        inputs.extend(bits_lsb(0, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("circuit evaluation failed");

        assert_eq!(decode_lsb(&outputs), 0);
    }

    #[test]
    fn multiplication_by_one_preserves_value() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(173, 8);
        inputs.extend(bits_lsb(1, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("circuit evaluation failed");

        assert_eq!(decode_lsb(&outputs), 173);
    }

    #[test]
    fn multiplication_wraps_at_result_width() {
        let source = r#"
            fn multiply(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");
        let program = Parser::new(tokens).parse().expect("parsing failed");
        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(20, 8);
        inputs.extend(bits_lsb(20, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("circuit evaluation failed");

        assert_eq!(decode_lsb(&outputs), 144);
    }
    fn evaluate_comparison(operator: &str, lhs: u8, rhs: u8) -> bool {
        let source = format!(
            r#"
            fn compare(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<bool> {{
                return a {} b;
            }}
        "#,
            operator
        );

        let tokens = Lexer::new(&source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 2);
        assert_eq!(result.outputs.len(), 1);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(lhs as u64, 8);
        inputs.extend(bits_lsb(rhs as u64, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("comparison evaluation failed");

        outputs[0]
    }

    #[test]
    fn equality_comparison_is_correct() {
        assert!(evaluate_comparison("==", 42, 42));
        assert!(!evaluate_comparison("==", 42, 41));
        assert!(evaluate_comparison("==", 0, 0));
        assert!(evaluate_comparison("==", 255, 255));
        assert!(!evaluate_comparison("==", 0, 255));
    }

    #[test]
    fn inequality_comparison_is_correct() {
        assert!(evaluate_comparison("!=", 42, 41));
        assert!(!evaluate_comparison("!=", 42, 42));
        assert!(evaluate_comparison("!=", 0, 255));
        assert!(!evaluate_comparison("!=", 255, 255));
    }

    #[test]
    fn greater_than_comparison_is_correct() {
        assert!(evaluate_comparison(">", 200, 12));
        assert!(!evaluate_comparison(">", 12, 200));
        assert!(!evaluate_comparison(">", 42, 42));
        assert!(evaluate_comparison(">", 255, 0));
        assert!(!evaluate_comparison(">", 0, 255));
    }

    #[test]
    fn greater_equal_comparison_is_correct() {
        assert!(evaluate_comparison(">=", 200, 12));
        assert!(evaluate_comparison(">=", 42, 42));
        assert!(!evaluate_comparison(">=", 12, 200));
        assert!(evaluate_comparison(">=", 255, 255));
        assert!(evaluate_comparison(">=", 255, 0));
    }

    #[test]
    fn less_than_comparison_is_correct() {
        assert!(evaluate_comparison("<", 12, 200));
        assert!(!evaluate_comparison("<", 200, 12));
        assert!(!evaluate_comparison("<", 42, 42));
        assert!(evaluate_comparison("<", 0, 255));
        assert!(!evaluate_comparison("<", 255, 0));
    }

    #[test]
    fn less_equal_comparison_is_correct() {
        assert!(evaluate_comparison("<=", 12, 200));
        assert!(evaluate_comparison("<=", 42, 42));
        assert!(!evaluate_comparison("<=", 200, 12));
        assert!(evaluate_comparison("<=", 0, 255));
        assert!(evaluate_comparison("<=", 255, 255));
    }

    #[test]
    fn comparison_boundary_neighbors_are_correct() {
        assert!(evaluate_comparison("<", 0, 1));
        assert!(evaluate_comparison(">", 1, 0));

        assert!(evaluate_comparison("<", 254, 255));
        assert!(evaluate_comparison(">", 255, 254));

        assert!(evaluate_comparison("<=", 254, 255));
        assert!(evaluate_comparison(">=", 255, 254));
    }

    #[test]
    fn comparisons_record_gate_attribution() {
        let cases = [
            ("==", "Eq"),
            ("!=", "NotEqual"),
            (">", "GreaterThan"),
            (">=", "GreaterEqual"),
            ("<", "LessThan"),
            ("<=", "LessEqual"),
        ];

        for (operator, expected_kind) in cases {
            let source = format!(
                r#"
                fn compare(
                    a: encrypted<u8>,
                    b: encrypted<u8>
                ) -> encrypted<bool> {{
                    return a {} b;
                }}
            "#,
                operator
            );

            let tokens = Lexer::new(&source).tokenize().expect("lexing failed");

            let program = Parser::new(tokens).parse().expect("parsing failed");

            let module = lower_program(&program).expect("lowering failed");

            let function = &module.functions[0];
            let result = lower_function_to_circuit(function);

            let region = result
                .attribution_regions
                .iter()
                .find(|region| region.operation_kind == expected_kind)
                .expect("missing comparison attribution");

            assert!(region.gate_count() > 0);
        }
    }
    #[test]
    fn nested_encrypted_arithmetic_expression_is_correct() {
        let source = r#"
            fn nested(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<u8> {
                return (a + b) * (c - d);
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Add { .. }))
        );

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Sub { .. }))
        );

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Mul { .. }))
        );

        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 4);
        assert_eq!(result.outputs.len(), 8);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(3, 8);
        inputs.extend(bits_lsb(4, 8));
        inputs.extend(bits_lsb(9, 8));
        inputs.extend(bits_lsb(2, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("nested arithmetic evaluation failed");

        assert_eq!(decode_lsb(&outputs), 49);
    }

    #[test]
    fn nested_encrypted_arithmetic_comparison_is_correct() {
        let source = r#"
            fn nested_compare(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<bool> {
                return (a + b) > (c - d);
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 1);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(20, 8);
        inputs.extend(bits_lsb(5, 8));
        inputs.extend(bits_lsb(15, 8));
        inputs.extend(bits_lsb(2, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("nested comparison evaluation failed");

        assert_eq!(outputs, vec![true]);
    }

    #[test]
    fn nested_expression_preserves_operation_attribution() {
        let source = r#"
            fn nested(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<u8> {
                return (a + b) * (c - d);
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        for expected in ["Add", "Sub", "Mul"] {
            let region = result
                .attribution_regions
                .iter()
                .find(|region| region.operation_kind == expected)
                .expect("missing nested-operation attribution");

            assert!(region.gate_count() > 0);
        }
    }

    #[test]
    fn nested_encrypted_conditional_is_correct() {
        let source = r#"
            fn nested_select(
                first: encrypted<bool>,
                second: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>
            ) -> encrypted<u8> {
                return if first {
                    if second {
                        a
                    } else {
                        b
                    }
                } else {
                    c
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 5);
        assert_eq!(result.outputs.len(), 8);
        assert!(result.verify(function).is_ok());

        let mut inputs = vec![true, false];
        inputs.extend(bits_lsb(10, 8));
        inputs.extend(bits_lsb(20, 8));
        inputs.extend(bits_lsb(30, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("nested conditional evaluation failed");

        assert_eq!(decode_lsb(&outputs), 20);
    }

    #[test]
    fn nested_encrypted_conditional_all_paths_are_correct() {
        let source = r#"
            fn nested_select(
                first: encrypted<bool>,
                second: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>
            ) -> encrypted<u8> {
                return if first {
                    if second {
                        a
                    } else {
                        b
                    }
                } else {
                    c
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let cases = [
            (true, true, 10),
            (true, false, 20),
            (false, true, 30),
            (false, false, 30),
        ];

        for (first, second, expected) in cases {
            let mut inputs = vec![first, second];

            inputs.extend(bits_lsb(10, 8));
            inputs.extend(bits_lsb(20, 8));
            inputs.extend(bits_lsb(30, 8));

            let outputs = result
                .circuit
                .evaluate(&inputs, &result.outputs)
                .expect("nested conditional evaluation failed");

            assert_eq!(decode_lsb(&outputs), expected);
        }
    }

    #[test]
    fn nested_conditional_generates_multiple_mux_regions() {
        let source = r#"
            fn nested_select(
                first: encrypted<bool>,
                second: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>
            ) -> encrypted<u8> {
                return if first {
                    if second {
                        a
                    } else {
                        b
                    }
                } else {
                    c
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mux_regions = result
            .attribution_regions
            .iter()
            .filter(|region| region.operation_kind == "Mux")
            .count();

        assert!(mux_regions >= 2);
    }
    #[test]
    fn large_multi_operation_program_is_correct() {
        let source = r#"
            fn compute(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<u8> {
                let sum = a + b;
                let difference = c - d;
                let product = sum * difference;
                let adjustment = a + d;
                return product + adjustment;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 4);
        assert_eq!(result.outputs.len(), 8);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(3, 8);
        inputs.extend(bits_lsb(4, 8));
        inputs.extend(bits_lsb(9, 8));
        inputs.extend(bits_lsb(2, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("multi-operation evaluation failed");

        assert_eq!(decode_lsb(&outputs), 54);
    }

    #[test]
    fn multi_operation_program_preserves_intermediate_values() {
        let source = r#"
            fn compute(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>
            ) -> encrypted<u8> {
                let first = a + b;
                let second = first * c;
                let third = second - a;
                return third + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let result = lower_function_to_circuit(function);

        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(5, 8);
        inputs.extend(bits_lsb(3, 8));
        inputs.extend(bits_lsb(4, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("multi-operation evaluation failed");

        assert_eq!(decode_lsb(&outputs), 30);
    }

    #[test]
    fn multi_operation_program_can_finish_with_comparison() {
        let source = r#"
            fn compute(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<bool> {
                let left = a + b;
                let right = c * d;
                return left >= right;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 1);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(20, 8);
        inputs.extend(bits_lsb(5, 8));
        inputs.extend(bits_lsb(4, 8));
        inputs.extend(bits_lsb(6, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("comparison evaluation failed");

        assert_eq!(outputs, vec![true]);
    }

    #[test]
    fn multi_operation_program_records_all_major_operations() {
        let source = r#"
            fn compute(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<bool> {
                let sum = a + b;
                let difference = c - d;
                let product = sum * difference;
                return product > a;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let result = lower_function_to_circuit(function);

        for expected in ["Add", "Sub", "Mul", "GreaterThan"] {
            assert!(
                result
                    .attribution_regions
                    .iter()
                    .any(|region| { region.operation_kind == expected }),
                "missing attribution for {}",
                expected
            );
        }
    }
    #[test]
    fn full_source_to_circuit_regression_is_correct() {
        let source = r#"
            fn evaluate(
                condition: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>,
                d: encrypted<u8>
            ) -> encrypted<u8> {
                let sum = a + b;
                let difference = c - d;
                let product = sum * difference;

                return if condition {
                    product + a
                } else {
                    product - b
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        assert_eq!(module.functions.len(), 1);

        let function = &module.functions[0];

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Add { .. }))
        );

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Sub { .. }))
        );

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Mul { .. }))
        );

        assert!(
            function
                .operations
                .iter()
                .any(|operation| matches!(operation, Operation::Mux { .. }))
        );

        let result = lower_function_to_circuit(function);

        assert_eq!(result.inputs.len(), 5);
        assert_eq!(result.outputs.len(), 8);
        assert!(result.verify(function).is_ok());

        let mut true_inputs = vec![true];
        true_inputs.extend(bits_lsb(3, 8));
        true_inputs.extend(bits_lsb(4, 8));
        true_inputs.extend(bits_lsb(9, 8));
        true_inputs.extend(bits_lsb(2, 8));

        let true_outputs = result
            .circuit
            .evaluate(&true_inputs, &result.outputs)
            .expect("true-path evaluation failed");

        assert_eq!(decode_lsb(&true_outputs), 52);

        let mut false_inputs = vec![false];
        false_inputs.extend(bits_lsb(3, 8));
        false_inputs.extend(bits_lsb(4, 8));
        false_inputs.extend(bits_lsb(9, 8));
        false_inputs.extend(bits_lsb(2, 8));

        let false_outputs = result
            .circuit
            .evaluate(&false_inputs, &result.outputs)
            .expect("false-path evaluation failed");

        assert_eq!(decode_lsb(&false_outputs), 45);
    }

    #[test]
    fn full_pipeline_handles_u16_arithmetic() {
        let source = r#"
            fn compute(
                a: encrypted<u16>,
                b: encrypted<u16>
            ) -> encrypted<u16> {
                return (a + b) - 5;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 16);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(1000, 16);
        inputs.extend(bits_lsb(250, 16));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("u16 evaluation failed");

        assert_eq!(decode_lsb(&outputs), 1245);
    }

    #[test]
    fn full_pipeline_handles_u16_comparison() {
        let source = r#"
            fn compare(
                a: encrypted<u16>,
                b: encrypted<u16>
            ) -> encrypted<bool> {
                return a > b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 1);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(50000, 16);
        inputs.extend(bits_lsb(49999, 16));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("u16 comparison evaluation failed");

        assert_eq!(outputs, vec![true]);
    }

    #[test]
    fn u8_overflow_is_truncated_to_result_width() {
        let source = r#"
            fn overflow(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(255, 8);
        inputs.extend(bits_lsb(1, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("overflow evaluation failed");

        assert_eq!(decode_lsb(&outputs), 0);
    }

    #[test]
    fn source_to_ir_to_circuit_keeps_attribution_consistent() {
        let source = r#"
            fn compute(
                a: encrypted<u8>,
                b: encrypted<u8>,
                c: encrypted<u8>
            ) -> encrypted<bool> {
                let sum = a + b;
                let product = sum * c;
                let difference = product - a;

                return difference >= b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert!(result.verify(function).is_ok());

        for expected in ["Add", "Mul", "Sub", "GreaterEqual"] {
            let count = result
                .attribution_regions
                .iter()
                .filter(|region| region.operation_kind == expected)
                .count();

            assert_eq!(count, 1, "unexpected attribution count for {}", expected);
        }
    }
    fn evaluate_signed_comparison(operator: &str, lhs: u8, rhs: u8) -> bool {
        let source = format!(
            r#"
            fn compare(
                a: encrypted<i8>,
                b: encrypted<i8>
            ) -> encrypted<bool> {{
                return a {} b;
            }}
        "#,
            operator
        );

        let tokens = Lexer::new(&source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 1);
        assert!(result.verify(function).is_ok());

        let mut inputs = bits_lsb(lhs as u64, 8);

        inputs.extend(bits_lsb(rhs as u64, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("signed comparison failed");

        outputs[0]
    }

    #[test]
    fn signed_comparisons_handle_negative_values() {
        assert!(!evaluate_signed_comparison(">", 0xff, 0x01,));

        assert!(evaluate_signed_comparison("<", 0xff, 0x01,));

        assert!(evaluate_signed_comparison(">", 0x01, 0xff,));

        assert!(evaluate_signed_comparison(">", 0xff, 0xfe,));
    }

    #[test]
    fn signed_comparisons_handle_i8_boundaries() {
        assert!(evaluate_signed_comparison("<", 0x80, 0x7f,));

        assert!(evaluate_signed_comparison(">", 0x7f, 0x80,));

        assert!(evaluate_signed_comparison("<=", 0x80, 0x80,));

        assert!(evaluate_signed_comparison(">=", 0x7f, 0x7f,));
    }

    #[test]
    fn signed_equality_is_bit_exact() {
        assert!(evaluate_signed_comparison("==", 0xff, 0xff,));

        assert!(evaluate_signed_comparison("!=", 0xff, 0xfe,));
    }

    #[test]
    fn signed_addition_uses_twos_complement_bits() {
        let source = r#"
            fn add(
                a: encrypted<i8>,
                b: encrypted<i8>
            ) -> encrypted<i8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(0xff, 8);

        inputs.extend(bits_lsb(0x02, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("signed addition failed");

        assert_eq!(decode_lsb(&outputs), 0x01);
    }

    #[test]
    fn signed_subtraction_wraps_twos_complement() {
        let source = r#"
            fn subtract(
                a: encrypted<i8>,
                b: encrypted<i8>
            ) -> encrypted<i8> {
                return a - b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(0x80, 8);

        inputs.extend(bits_lsb(0x01, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("signed subtraction failed");

        assert_eq!(decode_lsb(&outputs), 0x7f);
    }

    #[test]
    fn signed_multiplication_preserves_twos_complement_bits() {
        let source = r#"
            fn multiply(
                a: encrypted<i8>,
                b: encrypted<i8>
            ) -> encrypted<i8> {
                return a * b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        let mut inputs = bits_lsb(0xfd, 8);

        inputs.extend(bits_lsb(0x02, 8));

        let outputs = result
            .circuit
            .evaluate(&inputs, &result.outputs)
            .expect("signed multiplication failed");

        assert_eq!(decode_lsb(&outputs), 0xfa);
    }
    #[test]
    fn records_ir_gate_attribution() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");

        let function = &module.functions[0];

        let result = lower_function_to_circuit(function);

        let add_region = result
            .attribution_regions
            .iter()
            .find(|region| region.operation_kind == "Add")
            .expect("missing Add attribution");

        assert!(add_region.gate_count() > 0);

        assert!(add_region.gate_end <= result.circuit.gate_count());
    }

    #[test]
    fn lowers_encrypted_addition_to_result_width() {
        let source = r#"
            fn add_constant(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a + 5;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");
        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 8);
        assert!(result.verify(function).is_ok());
    }

    #[test]
    fn lowers_if_to_bit_muxes() {
        let source = r#"
            fn select(
                condition: encrypted<bool>,
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return if condition {
                    a
                } else {
                    b
                };
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");
        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 8);
        assert!(
            result
                .circuit
                .gates()
                .iter()
                .any(|gate| matches!(gate, fhec_circuit::Gate::Mux { .. }))
        );
        assert!(result.verify(function).is_ok());
    }

    #[test]
    fn lowers_boolean_operation() {
        let source = r#"
            fn select(
                a: encrypted<bool>,
                b: encrypted<bool>
            ) -> encrypted<bool> {
                return a;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");
        let function = &module.functions[0];
        let result = lower_function_to_circuit(function);

        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.inputs.len(), 2);
        assert!(result.verify(function).is_ok());
    }

    #[test]
    fn circuit_verification_rejects_wrong_output_width() {
        let source = r#"
            fn add(
                a: encrypted<u8>,
                b: encrypted<u8>
            ) -> encrypted<u8> {
                return a + b;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");
        let function = &module.functions[0];
        let mut result = lower_function_to_circuit(function);

        result.outputs.pop();

        let error = result
            .verify(function)
            .expect_err("verification should fail");

        assert!(matches!(
            error,
            CircuitFunctionVerificationError::OutputWidth {
                expected: 8,
                actual: 7
            }
        ));
    }

    #[test]
    fn circuit_verification_rejects_wrong_input_width() {
        let source = r#"
            fn identity(
                a: encrypted<u8>
            ) -> encrypted<u8> {
                return a;
            }
        "#;

        let tokens = Lexer::new(source).tokenize().expect("lexing failed");

        let program = Parser::new(tokens).parse().expect("parsing failed");

        let module = lower_program(&program).expect("lowering failed");
        let function = &module.functions[0];
        let mut result = lower_function_to_circuit(function);

        result.inputs.get_mut(&ValueId(0)).unwrap().bits.pop();

        let error = result
            .verify(function)
            .expect_err("verification should fail");

        assert!(matches!(
            error,
            CircuitFunctionVerificationError::InputWidth {
                value: ValueId(0),
                expected: 8,
                actual: 7
            }
        ));
    }
}

#[cfg(test)]
mod release_value_definition_tests {
    use super::*;
    use fhec_ir::{Type, Value};

    #[test]
    fn rejects_redefinition_of_an_ir_result() {
        let mut function = Function::new(
            "duplicate_result",
            vec![Value::new(ValueId(0), Type::Bool)],
            Type::Bool,
        );
        let value = function.create_value(Type::Bool);
        function.push_operation(Operation::Not {
            result: value.id,
            operand: ValueId(0),
        });
        function.push_operation(Operation::Not {
            result: value.id,
            operand: ValueId(0),
        });
        function.push_operation(Operation::Return {
            value: Some(value.id),
        });

        assert!(matches!(
            try_lower_function_to_circuit(&function),
            Err(CircuitLoweringError::DuplicateValue(id)) if id == value.id
        ));
    }

    #[test]
    fn rejects_duplicate_parameter_ids() {
        let function = Function::new(
            "duplicate_parameters",
            vec![
                Value::new(ValueId(0), Type::Bool),
                Value::new(ValueId(0), Type::Bool),
            ],
            Type::Bool,
        );

        assert_eq!(
            validate_circuit_operands(&function),
            Err(CircuitLoweringError::DuplicateValue(ValueId(0)))
        );
    }
}
