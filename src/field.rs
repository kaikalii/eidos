use std::{fmt, iter, mem::swap, ops::*};

use enum_iterator::Sequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resampler {
    Offset,
    Scale,
    Flip,
}

impl Resampler {
    pub fn sample_value(&self, x: f32, factor: f32) -> f32 {
        match self {
            Resampler::Offset => x - factor,
            Resampler::Scale => x / factor,
            Resampler::Flip => 2.0 * factor - x,
        }
    }
    pub fn range_value(&self, range: RangeInclusive<f32>, factor: f32) -> RangeInclusive<f32> {
        let start = *range.start();
        let end = *range.end();
        let (mut start, mut end) = match self {
            Resampler::Offset => (start + factor, end + factor),
            Resampler::Scale => (start * factor, end * factor),
            Resampler::Flip => (2.0 * factor - end, 2.0 * factor - start),
        };
        if end < start {
            swap(&mut start, &mut end);
        }
        start..=end
    }
}

#[derive(Debug, Clone)]
pub enum Field {
    Array { data: Vec<f32>, shape: Vec<usize> },
    Identity,
    Un(UnOp, Box<Self>),
    Zip(BinOp, Box<Self>, Box<Self>),
    Square(BinOp, Box<Self>, Box<Self>),
    Resample(Box<Self>, Resampler, f32),
}

impl Field {
    fn from_array_data(
        data: impl IntoIterator<Item = f32>,
        shape: impl IntoIterator<Item = usize>,
    ) -> Self {
        Field::Array {
            data: data.into_iter().collect(),
            shape: shape.into_iter().collect(),
        }
    }
    pub fn uniform(f: f32) -> Self {
        Field::from_array_data([f], [])
    }
    pub fn list(items: impl IntoIterator<Item = impl Into<f32>>) -> Self {
        let data: Vec<f32> = items.into_iter().map(Into::into).collect();
        let shape = [data.len()];
        Field::from_array_data(data, shape)
    }
    pub fn array2d<const N: usize>(columns: impl IntoIterator<Item = [impl Into<f32>; N]>) -> Self {
        let data: Vec<f32> = columns.into_iter().flatten().map(Into::into).collect();
        let shape = [data.len() / N, N];
        Field::from_array_data(data, shape)
    }
    pub fn as_scalar(&self) -> Option<f32> {
        match self {
            Field::Array { data, shape } if shape.is_empty() => Some(data[0]),
            _ => None,
        }
    }
    pub fn rank(&self) -> usize {
        match self {
            Field::Array { shape, .. } => shape.len(),
            Field::Identity => 1,
            Field::Un(_, field) => field.rank(),
            Field::Zip(_, a, b) => a.rank().max(b.rank()),
            Field::Square(_, a, b) => a.rank() + b.rank(),
            Field::Resample(field, _, _) => field.rank(),
        }
    }
    pub fn sample(&self, x: f32) -> Field {
        match self {
            Field::Identity => Field::uniform(x),
            Field::Array { data, shape } => {
                let index = x.round() as usize;
                if shape.is_empty() {
                    return self.clone();
                }
                let subshape = shape[1..].to_vec();
                let frame_size: usize = subshape.iter().product();
                let start = index * frame_size;
                let end = (index + 1) * frame_size;
                let mut subdata = Vec::with_capacity(end - start);
                for i in start..end {
                    subdata.push(data.get(i).copied().unwrap_or(0.0));
                }
                Field::Array {
                    data: subdata,
                    shape: subshape,
                }
            }
            Field::Un(op, field) => {
                let field = field.sample(x);
                if let Some(s) = field.as_scalar() {
                    Field::uniform(s)
                } else {
                    Field::Un(*op, field.sample(x).into())
                }
            }
            Field::Zip(op, a, b) => {
                let a = a.sample(x);
                let b = b.sample(x);
                a.zip(*op, b)
            }
            Field::Square(op, a, b) => {
                if let Some(a) = a.as_scalar() {
                    let b = b.sample(x);
                    if let Some(b) = b.as_scalar() {
                        Field::uniform(op.operate(a, b))
                    } else {
                        Field::uniform(a).zip(*op, b)
                    }
                } else {
                    Field::Square(*op, a.clone(), b.clone())
                }
            }
            Field::Resample(field, resampler, factor) => {
                let x = resampler.sample_value(x, *factor);
                field.sample(x)
            }
        }
    }
    pub fn un(self, op: UnOp) -> Self {
        Field::Un(op, self.into())
    }
    pub fn zip(self, op: BinOp, other: Self) -> Self {
        if let (Some(a), Some(b)) = (self.as_scalar(), other.as_scalar()) {
            Field::uniform(op.operate(a, b))
        } else {
            Field::Zip(op, self.into(), other.into())
        }
    }
    pub fn square(self, op: BinOp, other: Self) -> Self {
        if let (Some(a), Some(b)) = (self.as_scalar(), other.as_scalar()) {
            Field::uniform(op.operate(a, b))
        } else {
            Field::Square(op, self.into(), other.into())
        }
    }
    pub fn sample_range(
        &self,
        range: impl RangeBounds<f32> + 'static,
        step: f32,
    ) -> impl Iterator<Item = Field> + '_ {
        let mut i = match range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start + step,
            Bound::Unbounded => 0.0,
        };
        iter::from_fn(move || {
            let ret = match range.end_bound() {
                Bound::Included(end) => &i <= end,
                Bound::Excluded(end) => &i < end,
                Bound::Unbounded => true,
            };
            if !ret {
                return None;
            }
            let value = self.sample(i);
            i += step;
            Some(value)
        })
    }
}

impl Neg for Field {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self.un(UnOp::Neg)
    }
}

macro_rules! bin_op {
    ($trait:ident, $method:ident) => {
        impl $trait for Field {
            type Output = Self;
            fn $method(self, other: Self) -> Self::Output {
                self.zip(BinOp::$trait, other)
            }
        }

        impl $trait<f32> for Field {
            type Output = Self;
            fn $method(self, other: f32) -> Self::Output {
                self.zip(BinOp::$trait, Field::uniform(other))
            }
        }
    };
}

bin_op!(Add, add);
bin_op!(Sub, sub);
bin_op!(Mul, mul);
bin_op!(Div, div);

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Field::Array { data, shape } => display_array(data, shape, f),
            Field::Identity => "Identity".fmt(f),
            Field::Un(op, field) => write!(f, "({op:?} {field})"),
            Field::Zip(op, a, b) => write!(f, "({op:?} {a} {b})"),
            Field::Square(op, a, b) => write!(f, "(square {op:?} {a} {b}"),
            Field::Resample(field, res, factor) => write!(f, "({res:?} {factor} {field})"),
        }
    }
}

struct ArrayFormatter<'a> {
    data: &'a [f32],
    shape: &'a [usize],
}

impl<'a> fmt::Debug for ArrayFormatter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.shape.is_empty() {
            write!(f, "{}", self.data[0])
        } else {
            let subshape = &self.shape[1..];
            let frame_size: usize = subshape.iter().product();
            f.debug_list()
                .entries((0..self.shape[0]).map(|i| {
                    let start = i * frame_size;
                    let end = (i + 1) * frame_size;
                    let subdata = &self.data[start..end];
                    ArrayFormatter {
                        data: subdata,
                        shape: subshape,
                    }
                }))
                .finish()
        }
    }
}

fn display_array(data: &[f32], shape: &[usize], f: &mut fmt::Formatter) -> fmt::Result {
    if shape.is_empty() {
        write!(f, "{}", data[0])
    } else {
        let subshape = &shape[1..];
        let frame_size: usize = subshape.iter().product();
        write!(f, "[")?;
        for i in 0..shape[0] {
            if i > 0 {
                write!(f, " ")?;
            }
            let start = i * frame_size;
            let end = (i + 1) * frame_size;
            let subdata = &data[start..end];
            display_array(subdata, subshape, f)?;
        }
        write!(f, "]")
    }
}
