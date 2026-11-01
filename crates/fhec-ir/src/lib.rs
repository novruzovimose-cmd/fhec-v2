mod function;
mod module;
mod operation;
mod types;
mod value;
mod verifier;

pub use function::Function;
pub use module::Module;
pub use operation::Operation;
pub use types::Type;
pub use value::{Constant, Value, ValueId};
pub use verifier::{VerificationError, verify_function};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_construct_encrypted_function() {
        let parameter = Value::new(ValueId(0), Type::encrypted(Type::uint(8)));

        let function = Function::new("test", vec![parameter], Type::encrypted(Type::uint(8)));

        assert_eq!(function.name, "test");
        assert_eq!(function.parameters.len(), 1);
        assert_eq!(function.return_type, Type::encrypted(Type::uint(8)));
    }

    #[test]
    fn can_create_mux_operation() {
        let operation = Operation::Mux {
            result: ValueId(3),
            condition: ValueId(2),
            if_true: ValueId(0),
            if_false: ValueId(1),
        };

        match operation {
            Operation::Mux {
                result,
                condition,
                if_true,
                if_false,
            } => {
                assert_eq!(result, ValueId(3));
                assert_eq!(condition, ValueId(2));
                assert_eq!(if_true, ValueId(0));
                assert_eq!(if_false, ValueId(1));
            }
            _ => panic!("expected Mux operation"),
        }
    }

    #[test]
    fn module_can_contain_functions() {
        let mut module = Module::new();

        let function = Function::new("identity", Vec::new(), Type::Unit);

        module.add_function(function);

        assert_eq!(module.function_count(), 1);
    }
}
