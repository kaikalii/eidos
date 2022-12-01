use std::ops::*;

#[derive(Debug, Clone)]
pub enum Field {
    Identity,
    Number(f32),
    List(Vec<f32>),
    Offset(Box<Self>, f32),
    Un(Box<Self>, UnOp),
    Linear(Box<Self>, BinOp, Box<Self>),
}

pub enum Field2 {
    Number(f32),
}

impl From<f32> for Field {
    fn from(f: f32) -> Self {
        Field::Number(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Abs,
}

impl UnOp {
    pub fn linear(self, field: impl Into<Field>) -> Field {
        match (self, field.into()) {
            (op, Field::Number(n)) => Field::Number(op.operate(n)),
            (UnOp::Neg, Field::Un(field, UnOp::Neg)) => *field,
            (op, field) => Field::Un(field.into(), op),
        }
    }
    pub fn operate(&self, x: f32) -> f32 {
        match self {
            UnOp::Neg => -x,
            UnOp::Abs => x.abs(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Min,
    Max,
}

impl BinOp {
    pub fn on(self, a: impl Into<Field>, b: impl Into<Field>) -> Field {
        match (self, a.into(), b.into()) {
            (op, Field::Number(a), Field::Number(b)) => Field::Number(op.operate(a, b)),
            (op, a, b) => Field::Linear(a.into(), op, b.into()),
        }
    }
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

impl Field {
    pub fn sample(&self, x: f32) -> f32 {
        match self {
            Field::Identity => x,
            Field::Number(n) => *n,
            Field::List(list) => list.get(x.round() as usize).copied().unwrap_or(0.0),
            Field::Offset(field, by) => field.sample(x + *by),
            Field::Un(field, op) => op.operate(field.sample(x)),
            Field::Linear(a, op, b) => op.operate(a.sample(x), b.sample(x)),
        }
    }
    pub fn range(&self) -> Option<RangeInclusive<f32>> {
        match self {
            Field::Identity => None,
            Field::Number(_) => Some(0.0..=0.0),
            Field::List(list) => Some(0.0..=(list.len() - 1) as f32),
            Field::Offset(field, by) => field
                .range()
                .map(|range| (*range.start() + *by..=*range.end() + *by)),
            Field::Un(field, _) => field.range(),
            Field::Linear(a, _, b) => match (a.range(), b.range()) {
                (None, None) => None,
                (None, Some(range)) | (Some(range), None) => Some(range),
                (Some(a), Some(b)) => Some(a.start().min(*b.start())..=a.end().max(*b.end())),
            },
        }
    }
}

macro_rules! bin_op {
    ($trait:ident, $method:ident) => {
        impl $trait for Field {
            type Output = Self;
            fn $method(self, other: Self) -> Self::Output {
                BinOp::$trait.on(self, other)
            }
        }
    };
}

bin_op!(Add, add);
bin_op!(Sub, sub);
bin_op!(Mul, mul);
bin_op!(Div, div);

impl Neg for Field {
    type Output = Self;
    fn neg(self) -> Self::Output {
        UnOp::Neg.linear(self)
    }
}
