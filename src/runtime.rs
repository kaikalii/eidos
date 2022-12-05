use std::fmt;

use crate::{BinaryField, Combinator1, Combinator2, EidosError, Field, Function, Value};

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
    pub fn validate_function_use(&self, function: &Function) -> Result<(), EidosError> {
        function.validate_use(&self.stack)
    }
    pub fn do_instr(&mut self, instr: &Instr) -> Result<(), EidosError> {
        match instr {
            Instr::Number(f) => self.stack.push((*f).into()),
            Instr::List(nums) => self.stack.push(Field::list(nums.clone()).into()),
            Instr::Function(function) => {
                self.validate_function_use(function)?;
                match function {
                    Function::Nullary(nullary) => self.stack.push(nullary.value()),
                    Function::UnaryField(un) => {
                        let value = self.stack.pop().unwrap();
                        match value {
                            Value::Field(f) => self.stack.push(f.un(*un).into()),
                            _ => unreachable!(),
                        }
                    }
                    Function::BinaryField(bin) => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match (a, b) {
                            (Value::Field(a), Value::Field(b)) => self.stack.push(match bin {
                                BinaryField::Op(op) => a.zip(*op, b).into(),
                                BinaryField::Sample => a.sample_field(b).into(),
                            }),
                            _ => unreachable!(),
                        }
                    }
                    Function::Combinator1(com1) => {
                        let value = self.stack.pop().unwrap();
                        match com1 {
                            Combinator1::Duplicate => {
                                self.stack.push(value.clone());
                                self.stack.push(value);
                            }
                            Combinator1::Drop => {}
                        }
                    }
                    Function::Combinator2(com2) => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        match com2 {
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
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(match (a, b) {
                            (Value::Field(a), Value::Field(b)) => {
                                a.resample(*res, b.as_scalar().unwrap()).into()
                            }
                            _ => unreachable!(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
