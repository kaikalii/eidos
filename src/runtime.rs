use std::fmt;

use crate::{
    Combinator1, Combinator2, EidosError, Function, GenericField, GenericUnOp, GenericValue,
    ScalarField, UnOp, UnOperator, Value, VectorField,
};

pub type Stack<'a> = Vec<Value<'a>>;

#[derive(Default)]
pub struct Runtime<'a> {
    pub stack: Stack<'a>,
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

impl<'a> Runtime<'a> {
    pub fn validate_function_use(&self, function: Function) -> Result<(), EidosError> {
        function.validate_use(&self.stack)
    }
    #[track_caller]
    pub fn pop_field(&mut self) -> GenericField<'a> {
        match self.stack.pop() {
            Some(Value::Field(field)) => field,
            Some(value) => panic!("Popped value was a {} instead of a field", value.ty()),
            None => panic!("Nothing to pop"),
        }
    }
    #[track_caller]
    pub fn pop(&mut self) -> Value<'a> {
        self.stack.pop().expect("Nothing to pop")
    }
    pub fn push(&mut self, value: impl Into<Value<'a>>) {
        self.stack.push(value.into())
    }
    pub fn do_instr(&mut self, instr: &Instr) -> Result<(), EidosError> {
        match instr {
            Instr::Number(f) => self.stack.push((*f).into()),
            Instr::Function(function) => self.call(*function)?,
        }
        Ok(())
    }
    pub fn call_value(&mut self, value: Value<'a>) -> Result<(), EidosError> {
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
            Function::Nullary(nullary) => self.push(nullary.value()),
            Function::Combinator1(com1) => {
                let a = self.pop();
                match com1 {
                    Combinator1::Duplicate => {
                        self.push(a.clone());
                        self.push(a);
                    }
                    Combinator1::Drop => {}
                }
            }
            Function::Combinator2(com2) => {
                let b = self.pop();
                let a = self.pop();
                match com2 {
                    Combinator2::Apply => {
                        self.push(a);
                        self.call_value(b)?;
                    }
                    Combinator2::Swap => {
                        self.push(b);
                        self.push(a);
                    }
                    Combinator2::Over => {
                        self.push(a.clone());
                        self.push(b);
                        self.push(a);
                    }
                }
            }
            Function::Un(op) => {
                let a = self.pop();
                match op {
                    GenericUnOp::Math(op) => match a {
                        Value::Value(v) => self.push(v.un(op)),
                        Value::Field(GenericField::Scalar(f)) => {
                            self.push(ScalarField::ScalarUn(UnOp::Math(op), f.into()))
                        }
                        Value::Field(GenericField::Vector(f)) => {
                            self.push(VectorField::Un(UnOp::Math(op), f.into()))
                        }
                        _ => unreachable!(),
                    },
                    GenericUnOp::Scalar(op) => match a {
                        Value::Value(GenericValue::Scalar(v)) => self.push(op.operate(v)),
                        Value::Field(GenericField::Scalar(f)) => {
                            self.push(ScalarField::ScalarUn(UnOp::Typed(op), f.into()))
                        }
                        _ => unreachable!(),
                    },
                    GenericUnOp::VectorScalar(_) => todo!(),
                    GenericUnOp::VectorVector(_) => todo!(),
                }
            }
        }
        Ok(())
    }
}
