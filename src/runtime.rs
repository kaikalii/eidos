use std::fmt;

use crate::{EidosError, FieldTrait, Function, Type, Value};

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
    pub fn function_ret_type(&self, function: &Function) -> Result<Type, EidosError> {
        function.ret_type(&self.stack)
    }
    pub fn do_instr(&mut self, instr: &Instr) -> Result<(), EidosError> {
        match instr {
            Instr::Number(f) => self.stack.push(Value::Atom(*f)),
            Instr::Function(function) => {
                self.function_ret_type(function)?;
                match function {
                    Function::Un(op) => {
                        let value = self.stack.pop().unwrap();
                        self.stack.push(match value {
                            Value::Atom(x) => Value::Atom(x.un_op(*op)),
                            Value::F1(x) => Value::F1(x.un_op(*op)),
                            Value::F2(x) => Value::F2(x.un_op(*op)),
                            Value::Function(_) => unreachable!(),
                        })
                    }
                    Function::Bin(op) => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(match (a, b) {
                            (Value::Atom(a), Value::Atom(b)) => Value::Atom(a.zip(*op, b)),
                            (Value::F1(a), Value::F1(b)) => Value::F1(a.zip(*op, b)),
                            (Value::F2(a), Value::F2(b)) => Value::F2(a.zip(*op, b)),
                            _ => unreachable!(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
