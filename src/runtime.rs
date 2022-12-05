use std::fmt;

use crate::{
    Combinator1, Combinator2, EidosError, Field, Function, Modifier, UnBinModifier, Value,
};

pub type Stack = Vec<Value>;

#[derive(Default)]
pub struct Runtime {
    pub stack: Stack,
}

#[derive(Debug)]
pub enum Instr {
    Number(f32),
    List(Vec<f32>),
    Function(Function),
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::Number(n) => n.fmt(f),
            Instr::List(nums) => {
                write!(f, "[")?;
                for (i, num) in nums.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    num.fmt(f)?;
                }
                write!(f, "]")
            }
            Instr::Function(function) => function.fmt(f),
        }
    }
}

impl Runtime {
    pub fn validate_function_use(&self, function: Function) -> Result<(), EidosError> {
        function.validate_use(&self.stack)
    }
    #[track_caller]
    pub fn pop_field(&mut self) -> Field {
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
            Instr::List(nums) => self.stack.push(Field::list(nums.clone()).into()),
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
        match function {
            Function::Modifier(modifier) => self.stack.push((modifier).into()),
            Function::Nullary(nullary) => self.stack.push(nullary.value()),
            Function::UnaryField(un) => {
                let value = self.pop_field();
                self.stack.push(value.un(un).into());
            }
            Function::BinaryField(bin) => {
                let b = self.pop();
                if let Value::Modifier(Modifier::UnBin(m)) = b {
                    let b = self.pop_field();
                    let a = self.pop_field();
                    match m {
                        UnBinModifier::Square => self.stack.push(a.square(bin, b).into()),
                    }
                } else {
                    let b = b.unwrap_field();
                    let a = self.pop_field();
                    self.stack.push(a.zip(bin, b).into());
                }
            }
            Function::Combinator1(com1) => {
                let value = self.pop();
                match com1 {
                    Combinator1::Duplicate => {
                        self.stack.push(value.clone());
                        self.stack.push(value);
                    }
                    Combinator1::Drop => {}
                }
            }
            Function::Combinator2(com2) => {
                let b = self.pop();
                let a = self.pop();
                match com2 {
                    Combinator2::Apply => {
                        self.stack.push(a);
                        self.call_value(b)?;
                    }
                    Combinator2::Swap => {
                        self.stack.push(b);
                        self.stack.push(a);
                    }
                    Combinator2::Over => {
                        self.stack.push(a.clone());
                        self.stack.push(b);
                        self.stack.push(a);
                    }
                }
            }
            Function::Resample(res) => {
                let b = self.pop_field();
                let a = self.pop_field();
                self.stack
                    .push(a.resample(res, b.as_scalar().unwrap()).into());
            }
        }
        Ok(())
    }
}
