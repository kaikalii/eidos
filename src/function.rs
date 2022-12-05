use std::{mem::swap, ops::RangeInclusive};

use derive_more::Display;
use enum_iterator::{all, Sequence};

use crate::{EidosError, Field, Type, Value};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Modifier(Modifier),
    Nullary(Nullary),
    UnaryField(UnOp),
    BinaryField(BinaryFieldFunction),
    Combinator1(Combinator1),
    Combinator2(Combinator2),
    Resample(Resampler),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Modifier {
    Square,
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
pub enum BinaryFieldFunction {
    Op(BinOp),
    Sample,
}

impl BinaryFieldFunction {
    pub fn on_scalar_and_field(&self, a: f32, b: &Field) -> Field {
        match self {
            BinaryFieldFunction::Op(op) => {
                if let Some(b) = b.as_scalar() {
                    Field::uniform(op.operate(a, b))
                } else {
                    Field::Zip(*self, Field::uniform(a).into(), b.clone().into())
                }
            }
            BinaryFieldFunction::Sample => b.sample(a),
        }
    }
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

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum UnOp {
    Neg,
    Abs,
    Sign,
    Sin,
    Cos,
    Tan,
}

impl UnOp {
    pub fn operate(&self, x: f32) -> f32 {
        match self {
            UnOp::Neg => -x,
            UnOp::Abs => x.abs(),
            UnOp::Sign if x == 0.0 => 0.0,
            UnOp::Sign => x.signum(),
            UnOp::Sin => x.sin(),
            UnOp::Cos => x.cos(),
            UnOp::Tan => x.tan(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Min,
    Max,
}

impl BinOp {
    pub fn operate(&self, a: f32, b: f32) -> f32 {
        match self {
            BinOp::Add => a + b,
            BinOp::Sub => a - b,
            BinOp::Mul => a * b,
            BinOp::Div => a / b,
            BinOp::Min => a.min(b),
            BinOp::Max => a.max(b),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Resampler {
    Offset,
    Scale,
    Flip,
}

impl Resampler {
    pub fn sample_value(&self, x: f32, factor: f32) -> f32 {
        match self {
            Resampler::Offset => x - factor,
            Resampler::Scale => x * factor,
            Resampler::Flip => 2.0 * factor - x,
        }
    }
    pub fn range_value(&self, range: RangeInclusive<f32>, factor: f32) -> RangeInclusive<f32> {
        let start = *range.start();
        let end = *range.end();
        let (mut start, mut end) = match self {
            Resampler::Offset => (start + factor, end + factor),
            Resampler::Scale => (start / factor, end / factor),
            Resampler::Flip => (2.0 * factor - end, 2.0 * factor - start),
        };
        if end < start {
            swap(&mut start, &mut end);
        }
        start..=end
    }
}

#[derive(Debug, Display, Sequence)]
pub enum FunctionCategory {
    Modifier,
    Nullary,
    Unary,
    Binary,
    Combinator,
    Resample,
}

impl FunctionCategory {
    pub fn functions(&self) -> Box<dyn Iterator<Item = Function>> {
        match self {
            FunctionCategory::Modifier => Box::new(all::<Modifier>().map(Function::Modifier)),
            FunctionCategory::Nullary => Box::new(all::<Nullary>().map(Function::Nullary)),
            FunctionCategory::Unary => Box::new(all::<UnOp>().map(Function::UnaryField)),
            FunctionCategory::Binary => {
                Box::new(all::<BinaryFieldFunction>().map(Function::BinaryField))
            }
            FunctionCategory::Combinator => Box::new(
                all::<Combinator1>()
                    .map(Function::Combinator1)
                    .chain(all::<Combinator2>().map(Function::Combinator2)),
            ),
            FunctionCategory::Resample => Box::new(all::<Resampler>().map(Function::Resample)),
        }
    }
}

#[derive(Debug, Display, Clone, Copy)]
pub enum TypeConstraint {
    Exact(Type),
    Field,
    Any,
}

impl Function {
    pub fn validate_use(&self, mut stack: &[Value]) -> Result<(), EidosError> {
        // Adjust for modifiers
        if let Some(Value::Modifier(_)) = stack.last() {
            stack = &stack[..stack.len() - 1];
        }
        // Collect constraints
        use TypeConstraint::*;
        let constraints = match self {
            Function::Modifier(_) => vec![],
            Function::Nullary(_) => vec![],
            Function::UnaryField(_) => vec![Field],
            Function::BinaryField(_) => vec![Field; 2],
            Function::Combinator1(_) => vec![Any],
            Function::Combinator2(_) => vec![Any; 2],
            Function::Resample(_) => vec![Field, Exact(Type::Field(0))],
        };
        // Validate stack size
        if stack.len() < constraints.len() {
            return Err(EidosError::NotEnoughArguments {
                function: *self,
                expected: constraints.len(),
                stack_size: stack.len(),
            });
        }
        // Validate constraints
        for (i, (constraint, value)) in constraints
            .into_iter()
            .rev()
            .zip(stack.iter().rev())
            .rev()
            .enumerate()
        {
            match (constraint, value) {
                (Any, _) => {}
                (Field, Value::Field(_)) => {}
                (Exact(ty), value) if ty == value.ty() => {}
                _ => {
                    return Err(EidosError::InvalidArgument {
                        function: *self,
                        position: i + 1,
                        expected: constraint,
                        found: value.ty(),
                    })
                }
            }
        }
        Ok(())
    }
}
