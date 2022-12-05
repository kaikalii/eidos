use crate::{error::EidosError, field::*, function::*, game::FieldsSource, value::*};

pub type Stack<'a> = Vec<Value<'a>>;

#[derive(Default)]
pub struct Runtime<'a> {
    pub stack: Stack<'a>,
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
    pub fn top_field(&self) -> Option<&GenericField<'a>> {
        match self.stack.last() {
            Some(Value::Field(field)) => Some(field),
            _ => None,
        }
    }
    pub fn call_value(
        &mut self,
        source: FieldsSource<'a, ()>,
        value: Value<'a>,
    ) -> Result<(), EidosError> {
        if let Value::Function(function) = value {
            self.call(source, function)
        } else {
            self.stack.push(value);
            Ok(())
        }
    }
    pub fn call(
        &mut self,
        source: FieldsSource<'a, ()>,
        function: Function,
    ) -> Result<(), EidosError> {
        self.validate_function_use(function)?;
        match function {
            Function::ReadField(field_kind) => match field_kind {
                GenericFieldKind::Scalar(kind) => {
                    self.push(ScalarField::World(ScalarWorldField { kind, source }))
                }
                GenericFieldKind::Vector(kind) => {
                    self.push(VectorField::World(VectorWorldField { kind, source }))
                }
            },
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
                        self.call_value(source, b)?;
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
                            self.push(ScalarField::ScalarUn(UnOp::Math(op), f.into()))
                        }
                        GenericField::Vector(f) => {
                            self.push(VectorField::Un(UnOp::Math(op), f.into()))
                        }
                    },
                    GenericUnOp::Scalar(op) => match a {
                        GenericField::Scalar(f) => {
                            self.push(ScalarField::ScalarUn(UnOp::Typed(op), f.into()))
                        }
                        _ => unreachable!(),
                    },
                    GenericUnOp::VectorScalar(op) => match a {
                        GenericField::Vector(f) => self.push(ScalarField::VectorUn(op, f.into())),
                        _ => unreachable!(),
                    },
                    GenericUnOp::VectorVector(op) => match a {
                        GenericField::Vector(f) => {
                            self.push(VectorField::Un(UnOp::Typed(op), f.into()))
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
                            self.push(ScalarField::Bin(BinOp::Math(op), a.into(), b.into()));
                        }
                        (GenericField::Scalar(a), GenericField::Vector(b)) => {
                            self.push(VectorField::BinSV(BinOp::Math(op), a.into(), b.into()));
                        }
                        (GenericField::Vector(a), GenericField::Scalar(b)) => {
                            self.push(VectorField::BinVS(BinOp::Math(op), a.into(), b.into()));
                        }
                        (GenericField::Vector(a), GenericField::Vector(b)) => {
                            self.push(VectorField::BinVV(BinOp::Math(op), a.into(), b.into()));
                        }
                    },
                    GenericBinOp::Homo(op) => match (a, b) {
                        (GenericField::Scalar(a), GenericField::Scalar(b)) => {
                            self.push(ScalarField::Bin(BinOp::Typed(op), a.into(), b.into()))
                        }
                        (GenericField::Vector(a), GenericField::Vector(b)) => {
                            self.push(VectorField::BinVV(BinOp::Typed(op), a.into(), b.into()))
                        }
                        _ => unreachable!(),
                    },
                }
            }
        }
        Ok(())
    }
}
