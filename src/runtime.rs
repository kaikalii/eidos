use std::fmt;

use crate::{EidosError, Field, Function, Type, Value};

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
    pub fn function_ret_type(&self, function: &Function) -> Result<Type, EidosError> {
        function.ret_type(&self.stack)
    }
    pub fn do_instr(&mut self, instr: &Instr) -> Result<(), EidosError> {
        match instr {
            Instr::Number(f) => self.stack.push((*f).into()),
            Instr::List(nums) => self.stack.push(Field::list(nums.clone()).into()),
            Instr::Function(function) => {
                self.function_ret_type(function)?;
                match function {
                    Function::Un(op) => {
                        let value = self.stack.pop().unwrap();
                        self.stack.push(match value {
                            Value::Field(f) => f.un(*op).into(),
                            Value::Function(_) => unreachable!(),
                        })
                    }
                    Function::Bin(op) => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(match (a, b) {
                            (Value::Field(a), Value::Field(b)) => a.zip(*op, b).into(),
                            _ => unreachable!(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
