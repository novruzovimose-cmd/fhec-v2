use std::collections::HashMap;

use crate::{Operation, Type, Value, ValueId};

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Value>,
    pub return_type: Type,
    pub operations: Vec<Operation>,
    values: HashMap<ValueId, Type>,
    next_value_id: u32,
}

impl Function {
    pub fn new(name: impl Into<String>, parameters: Vec<Value>, return_type: Type) -> Self {
        let next_value_id = parameters
            .iter()
            .map(|value| value.id.0)
            .max()
            .map_or(0, |id| id + 1);

        let values = parameters
            .iter()
            .map(|value| (value.id, value.ty.clone()))
            .collect();

        Self {
            name: name.into(),
            parameters,
            return_type,
            operations: Vec::new(),
            values,
            next_value_id,
        }
    }

    pub fn create_value(&mut self, ty: Type) -> Value {
        let id = ValueId(self.next_value_id);
        self.next_value_id += 1;

        self.values.insert(id, ty.clone());

        Value::new(id, ty)
    }

    pub fn value_type(&self, id: ValueId) -> Option<&Type> {
        self.values.get(&id)
    }

    pub fn push_operation(&mut self, operation: Operation) {
        self.operations.push(operation);
    }
}
