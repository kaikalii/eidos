use std::fmt;

use crate::{EidosError, Function, GenericField, Value};

pub type Stack = Vec<Value>;

#[derive(Default)]
pub struct Runtime {
    pub stack: Stack,
}

#[derive(Debug)]
pub enum Instr {
    Number(f32),
    Function(Function),
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::Number(n) => n.fmt(f),
            Instr::Function(function) => function.fmt(f),
        }
    }
}

impl Runtime {
    pub fn validate_function_use(&self, function: Function) -> Result<(), EidosError> {
        function.validate_use(&self.stack)
    }
    #[track_caller]
    pub fn pop_field(&mut self) -> GenericField {
        match self.stack.pop() {
            Some(Value::Field(field)) => field,
            Some(value) => panic!("Popped value was a {} instead of a field", value.ty()),
            None => panic!("Nothing to pop"),
        }
    }
    #[track_caller]
    pub fn pop(&mut self) -> Value {
        self.stack.pop().expect("Nothing to pop")
    }
    pub fn do_instr(&mut self, instr: &Instr) -> Result<(), EidosError> {
        match instr {
            Instr::Number(f) => self.stack.push((*f).into()),
            Instr::Function(function) => self.call(*function)?,
        }
        Ok(())
    }
    pub fn call_value(&mut self, value: Value) -> Result<(), EidosError> {
        if let Value::Function(function) = value {
            self.call(function)
        } else {
            self.stack.push(value);
            Ok(())
        }
    }
    pub fn call(&mut self, function: Function) -> Result<(), EidosError> {
        self.validate_function_use(function)?;

        Ok(())
    }
}
