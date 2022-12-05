use derive_more::Display;
use enum_iterator::{all, Sequence};

use crate::{BinOp, EidosError, Field, Resampler, Type, UnOp, Value};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Nullary(Nullary),
    UnaryField(UnOp),
    BinaryField(BinaryField),
    Combinator1(Combinator1),
    Combinator2(Combinator2),
    Resample(Resampler),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Nullary {
    Identity,
    Zero,
    One,
}

impl Nullary {
    pub fn value(&self) -> Value {
        match self {
            Nullary::Identity => Value::Field(Field::Identity),
            Nullary::Zero => Value::Field(Field::uniform(0.0)),
            Nullary::One => Value::Field(Field::uniform(1.0)),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum BinaryField {
    Op(BinOp),
    Sample,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator1 {
    Duplicate,
    Drop,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator2 {
    Swap,
    Over,
}

#[derive(Debug, Display, Sequence)]
pub enum FunctionCategory {
    Nullary,
    Unary,
    Binary,
    Combinator,
    Resample,
}

impl FunctionCategory {
    pub fn functions(&self) -> Box<dyn Iterator<Item = Function>> {
        match self {
            FunctionCategory::Nullary => Box::new(all::<Nullary>().map(Function::Nullary)),
            FunctionCategory::Unary => Box::new(all::<UnOp>().map(Function::UnaryField)),
            FunctionCategory::Binary => Box::new(all::<BinaryField>().map(Function::BinaryField)),
            FunctionCategory::Combinator => Box::new(
                all::<Combinator1>()
                    .map(Function::Combinator1)
                    .chain(all::<Combinator2>().map(Function::Combinator2)),
            ),
            FunctionCategory::Resample => Box::new(all::<Resampler>().map(Function::Resample)),
        }
    }
}

impl Function {
    pub fn validate_use(&self, stack: &[Value]) -> Result<(), EidosError> {
        #[derive(Clone, Copy)]
        enum TypeConstraint {
            Exact(Type),
            Field,
            Any,
        }
        use TypeConstraint::*;
        let mut args = 0;
        let mut constraints = [Any; 2];
        match self {
            Function::Nullary(_) => {}
            Function::UnaryField(_) => {
                args = 1;
                constraints[0] = Field;
            }
            Function::BinaryField(_) => {
                args = 2;
                constraints = [Field; 2];
            }
            Function::Combinator1(_) => args = 1,
            Function::Combinator2(_) => args = 2,
            Function::Resample(_) => {
                args = 2;
                constraints = [Field, Exact(Type::Field(0))];
            }
        }
        if stack.len() < args {
            return Err(EidosError::NotEnoughArguments {
                function: *self,
                expected: args,
                stack_size: stack.len(),
            });
        }
        for (i, (constraint, value)) in constraints
            .iter()
            .rev()
            .zip(stack.iter().rev())
            .rev()
            .take(args)
            .enumerate()
        {
            match (constraint, value) {
                (Any, _) => {}
                (Field, Value::Field(_)) => {}
                (Exact(ty), value) if ty == &value.ty() => {}
                _ => {
                    return Err(EidosError::InvalidArgument {
                        function: *self,
                        position: i + 1,
                        found_type: value.ty(),
                    })
                }
            }
        }
        Ok(())
    }
}
