use crate::{error::EidosError, field::*, function::*, value::*, world::World};

pub type Stack = Vec<Value>;

#[derive(Default)]
pub struct Runtime {
    pub stack: Stack,
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
    pub fn push(&mut self, value: impl Into<Value>) {
        self.stack.push(value.into())
    }
    pub fn top_field(&self) -> Option<&GenericField> {
        match self.stack.last() {
            Some(Value::Field(field)) => Some(field),
            _ => None,
        }
    }
    pub fn call_value(
        &mut self,
        world: &mut World,
        value: Value,
        write_outputs: bool,
    ) -> Result<(), EidosError> {
        if let Value::Function(function) = value {
            self.call(world, function, write_outputs)
        } else {
            self.stack.push(value);
            Ok(())
        }
    }
    pub fn call(
        &mut self,
        world: &mut World,
        function: Function,
        write_outputs: bool,
    ) -> Result<(), EidosError> {
        self.validate_function_use(function)?;
        match function {
            Function::ReadField(field_kind) => match field_kind {
                GenericInputFieldKind::Scalar(kind) => self.push(ScalarField::World(kind)),
                GenericInputFieldKind::Vector(kind) => self.push(VectorField::World(kind)),
            },
            Function::WriteField(field_kind) => {
                let field = self.pop_field();
                if write_outputs {
                    match (field_kind, field) {
                        (GenericOutputFieldKind::Vector(kind), GenericField::Vector(field)) => {
                            world.outputs.vectors.insert(kind, field);
                        }
                        _ => unreachable!(),
                    }
                }
            }
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
                        self.call_value(world, b, write_outputs)?;
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
                let a = self.pop_field();
                match op {
                    GenericUnOp::Math(op) => match a {
                        GenericField::Scalar(f) => {
                            self.push(ScalarField::ScalarUn(UnOp::Math(op), f.into()).reduce())
                        }
                        GenericField::Vector(f) => {
                            self.push(VectorField::Un(UnOp::Math(op), f.into()).reduce())
                        }
                    },
                    GenericUnOp::Scalar(op) => match a {
                        GenericField::Scalar(f) => {
                            self.push(ScalarField::ScalarUn(UnOp::Typed(op), f.into()).reduce())
                        }
                        _ => unreachable!(),
                    },
                    GenericUnOp::VectorScalar(op) => match a {
                        GenericField::Vector(f) => {
                            self.push(ScalarField::VectorUn(op, f.into()).reduce())
                        }
                        _ => unreachable!(),
                    },
                    GenericUnOp::VectorVector(op) => match a {
                        GenericField::Vector(f) => {
                            self.push(VectorField::Un(UnOp::Typed(op), f.into()).reduce())
                        }
                        _ => unreachable!(),
                    },
                }
            }
            Function::Bin(op) => {
                let b = self.pop_field();
                let a = self.pop_field();
                match op {
                    GenericBinOp::Math(op) => match (a, b) {
                        (GenericField::Scalar(a), GenericField::Scalar(b)) => {
                            self.push(
                                ScalarField::Bin(BinOp::Math(op), a.into(), b.into()).reduce(),
                            );
                        }
                        (GenericField::Scalar(a), GenericField::Vector(b)) => {
                            self.push(VectorField::BinSV(BinOp::Math(op), a, b.into()).reduce());
                        }
                        (GenericField::Vector(a), GenericField::Scalar(b)) => {
                            self.push(VectorField::BinVS(BinOp::Math(op), a.into(), b).reduce());
                        }
                        (GenericField::Vector(a), GenericField::Vector(b)) => {
                            self.push(
                                VectorField::BinVV(BinOp::Math(op), a.into(), b.into()).reduce(),
                            );
                        }
                    },
                    GenericBinOp::Homo(op) => match (a, b) {
                        (GenericField::Scalar(a), GenericField::Scalar(b)) => self
                            .push(ScalarField::Bin(BinOp::Typed(op), a.into(), b.into()).reduce()),
                        (GenericField::Vector(a), GenericField::Vector(b)) => self.push(
                            VectorField::BinVV(BinOp::Typed(op), a.into(), b.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                }
            }
        }
        Ok(())
    }
}
