use crate::Function;

#[derive(Debug, Default)]
pub struct Module {
    pub functions: Vec<Function>,
}

impl Module {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}
