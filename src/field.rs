use std::{fmt, iter, ops::*};

use crate::{BinFieldFunction, BinOp, Resampler, UnOp};

#[derive(Debug, Clone)]
pub enum Field {
    Array { data: Vec<f32>, shape: Vec<usize> },
    Identity,
    Un(UnOp, Box<Self>),
    Zip(BinFieldFunction, Box<Self>, Box<Self>),
    Square(BinFieldFunction, Box<Self>, Box<Self>),
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
    pub fn is_scalar(&self) -> bool {
        self.as_scalar().is_some()
    }
    pub fn rank(&self) -> usize {
        match self {
            Field::Array { shape, .. } => shape.len(),
            Field::Identity => 1,
            Field::Un(_, field) => field.rank(),
            Field::Zip(BinFieldFunction::Op(_), a, b) => a.rank().max(b.rank()),
            Field::Zip(BinFieldFunction::Sample, a, b) => a.rank() + b.rank().saturating_sub(1),
            Field::Square(BinFieldFunction::Op(_), a, b) => a.rank() + b.rank(),
            Field::Square(BinFieldFunction::Sample, a, b) => a.rank() + b.rank(), // Probably wrong
            Field::Resample(field, _, _) => field.rank(),
        }
    }
}

impl Field {
    pub fn sample(&self, x: f32) -> Field {
        match self {
            Field::Identity => Field::uniform(x),
            Field::Array { data, shape } => {
                if shape.is_empty() {
                    return self.clone();
                }
                let x = x.round();
                if x.is_nan() || x.is_infinite() || x < 0.0 || x >= shape[0] as f32 {
                    return Field::uniform(0.0);
                }
                let index = x as usize;
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
            Field::Un(op, field) => field.sample(x).un(*op),
            Field::Zip(f, a, b) => {
                if let Some(a) = a.as_scalar() {
                    f.on_scalar_and_field(a, b).sample(x)
                } else {
                    a.sample(x).zip(*f, (**b).clone()).sample(x)
                }
            }
            Field::Square(f, a, b) => {
                if let Some(a) = a.as_scalar() {
                    f.on_scalar_and_field(a, b).sample(x)
                } else {
                    a.sample(x).square(*f, (**b).clone())
                }
            }
            Field::Resample(field, resampler, factor) => {
                let x = resampler.sample_value(x, *factor);
                field.sample(x)
            }
        }
    }
    pub fn un(self, op: UnOp) -> Self {
        if let Some(a) = self.as_scalar() {
            Field::uniform(op.operate(a))
        } else {
            Field::Un(op, self.into())
        }
    }
    pub fn zip(self, f: BinFieldFunction, other: Self) -> Self {
        if let Some(a) = self.as_scalar() {
            f.on_scalar_and_field(a, &other)
        } else {
            Field::Zip(f, self.into(), other.into())
        }
    }
    pub fn square(self, f: BinFieldFunction, other: Self) -> Self {
        if let Some(a) = self.as_scalar() {
            f.on_scalar_and_field(a, &other)
        } else {
            Field::Square(f, self.into(), other.into())
        }
    }
    pub fn resample(self, resampler: Resampler, factor: f32) -> Self {
        if self.as_scalar().is_some() {
            self
        } else {
            Field::Resample(self.into(), resampler, factor)
        }
    }
    pub fn sample_field(self, field: Self) -> Self {
        if let Some(x) = self.as_scalar() {
            field.sample(x)
        } else {
            Field::Zip(BinFieldFunction::Sample, self.into(), field.into())
        }
    }
    pub fn default_range(&self) -> Option<RangeInclusive<f32>> {
        match self {
            Field::Array { data, shape } => {
                if shape.is_empty() {
                    None
                } else {
                    Some(0.0..=((data.len() as f32 / shape.len() as f32 - 1.0).max(1.0)))
                }
            }
            Field::Identity => None,
            Field::Un(_, field) => field.default_range(),
            Field::Zip(BinFieldFunction::Op(_), a, b) => {
                let a = a.default_range();
                let b = b.default_range();
                match (a, b) {
                    (Some(a), Some(b)) => Some(a.start().min(*b.start())..=(a.end().max(*b.end()))),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                }
            }
            Field::Zip(BinFieldFunction::Sample, a, b) => {
                if a.is_scalar() {
                    b.default_range()
                } else {
                    a.default_range()
                }
            }
            Field::Square(_, a, b) => {
                if a.is_scalar() {
                    b.default_range()
                } else {
                    a.default_range()
                }
            }
            Field::Resample(field, res, factor) => {
                let range = field.default_range()?;
                let a = res.sample_value(*range.start(), *factor);
                let b = res.sample_value(*range.end(), *factor);
                Some(a.min(b)..=a.max(b))
            }
        }
    }
    pub fn min_max(&self) -> Option<(f32, f32)> {
        Some(match self {
            Field::Array { data, shape } if shape.is_empty() => (data[0].abs(), data[0].abs()),
            Field::Array { data, shape } if shape.len() == 1 => {
                let min = *data
                    .iter()
                    .filter(|f| !f.is_nan())
                    .min_by(|a, b| a.partial_cmp(b).unwrap())?;
                let max = *data
                    .iter()
                    .filter(|f| !f.is_nan())
                    .max_by(|a, b| a.partial_cmp(b).unwrap())?;
                (min, max)
            }
            Field::Un(UnOp::Sin | UnOp::Cos, _) => (-1.0, 1.0),
            Field::Un(UnOp::Neg, field) => {
                let (min, max) = field.min_max()?;
                (-max, -min)
            }
            _ => return None,
        })
    }
    pub fn sample_range_step(
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
    pub fn sample_range_count(
        &self,
        range: RangeInclusive<f32>,
        count: usize,
    ) -> impl Iterator<Item = Field> + '_ {
        let start = *range.start();
        let end = *range.end();
        let step = (end - start) / count as f32;
        (0..count).map(move |i| self.sample(i as f32 * step))
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
                self.zip(BinFieldFunction::Op(BinOp::$trait), other)
            }
        }

        impl $trait<f32> for Field {
            type Output = Self;
            fn $method(self, other: f32) -> Self::Output {
                self.zip(BinFieldFunction::Op(BinOp::$trait), Field::uniform(other))
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
            Field::Square(op, a, b) => write!(f, "(square {op:?} {a} {b})"),
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
