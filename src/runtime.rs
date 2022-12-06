use crate::{error::EidosError, field::*, function::*, world::World};

pub type Stack = Vec<GenericField>;

#[derive(Default)]
pub struct Runtime {
    pub stack: Stack,
}

impl Runtime {
    pub fn validate_function_use(&self, function: Function) -> Result<(), EidosError> {
        function.validate_use(&self.stack)
    }
    #[track_caller]
    pub fn pop(&mut self) -> GenericField {
        self.stack.pop().expect("Nothing to pop")
    }
    pub fn push(&mut self, value: impl Into<GenericField>) {
        self.stack.push(value.into())
    }
    pub fn top(&self) -> Option<&GenericField> {
        self.stack.last()
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
                GenericFieldKind::Scalar(kind) => self.push(ScalarField::World(kind)),
                GenericFieldKind::Vector(kind) => self.push(VectorField::World(kind)),
            },
            Function::WriteField(field_kind) => {
                let field = self.pop();
                if write_outputs {
                    match (field_kind, field) {
                        (GenericOutputFieldKind::Vector(kind), GenericField::Vector(field)) => {
                            world.outputs.vectors.insert(kind, field);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Function::Nullary(nullary) => self.push(nullary.field()),
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
                let b = self.pop();
                let a = self.pop();
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
