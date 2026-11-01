mod circuit_lowering;
mod inlining;
mod lowering;
mod lowering_error;
mod optimizer_equivalence;
mod v4;

use fhec_frontend::ast::Program;
use fhec_ir::Module;

pub use circuit_lowering::CircuitFunction;
pub use lowering_error::LoweringError;

pub use v4::{
    CircuitImplementation, CircuitSelection, CompilerCandidate, CompilerDecisionSummary,
    CompilerV4Error, render_compiler_v4_report,
};

pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    pub fn lower(&self, program: &Program) -> Result<Module, LoweringError> {
        lowering::lower_program(program)
    }

    pub fn lower_to_circuit(&self, function: &fhec_ir::Function) -> CircuitFunction {
        circuit_lowering::lower_function_to_circuit(function)
    }

    pub fn try_lower_to_circuit(
        &self,
        function: &fhec_ir::Function,
    ) -> Result<CircuitFunction, CircuitLoweringError> {
        circuit_lowering::try_lower_function_to_circuit(function)
    }

    pub fn lower_to_circuit_candidates(
        &self,
        function: &fhec_ir::Function,
    ) -> Vec<CompilerCandidate> {
        v4::lower_to_candidates(function)
    }

    pub fn lower_to_circuit_v4(
        &self,
        function: &fhec_ir::Function,
        model: &fhec_analyzer::v4::V4CostModel,
    ) -> Result<CircuitSelection, CompilerV4Error> {
        v4::select_circuit(function, model)
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

pub fn lower_function_to_circuit(function: &fhec_ir::Function) -> CircuitFunction {
    circuit_lowering::lower_function_to_circuit(function)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_can_be_created() {
        let _compiler = Compiler::new();
    }

    #[test]
    fn compiler_can_lower_empty_program() {
        let program = Program {
            functions: Vec::new(),
        };

        let compiler = Compiler::new();

        let module = compiler.lower(&program).expect("lowering failed");

        assert_eq!(module.function_count(), 0);
    }
}

pub use circuit_lowering::{CircuitLoweringError, try_lower_function_to_circuit};

#[cfg(test)]
mod differential_tests;

#[cfg(test)]
mod optimizer_stress;

#[cfg(test)]
mod optimizer_effectiveness;

#[cfg(test)]
mod selection_invariants;

#[cfg(test)]
mod release_cost_policy_tests;
