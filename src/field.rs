use std::{borrow::Cow, fmt, iter, ops::*};

pub trait FieldTrait: Clone + fmt::Debug {
    type Sample: FieldTrait;
    type Squared: FieldTrait;
    fn uniform(f: Self::Sample) -> Self;
    fn sample(&self, x: f32) -> Cow<Self::Sample>;
    fn range(&self) -> Option<RangeInclusive<f32>>;
    fn un_op(self, op: UnOp) -> Self;
    fn zip(self, op: BinOp, other: Self) -> Self;
    fn square(self, op: BinOp, other: Self) -> Self::Squared;
    fn superuniform(x: f32) -> Self {
        Self::uniform(Self::Sample::superuniform(x))
    }
    fn sample_range(
        &self,
        range: impl RangeBounds<f32> + 'static,
        step: f32,
    ) -> Box<dyn Iterator<Item = Cow<Self::Sample>> + '_> {
        let mut i = match range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start + step,
            Bound::Unbounded => 0.0,
        };
        Box::new(iter::from_fn(move || {
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
        }))
    }
}

pub type Field1 = Field<f32>;
pub type Field2 = Field<Field1>;

impl From<f32> for Field1 {
    fn from(f: f32) -> Self {
        Field1::Uniform(f)
    }
}

#[derive(Debug, Clone)]
pub enum Field<S>
where
    S: FieldTrait,
{
    Identity,
    Uniform(S),
    List(Vec<S>),
    Offset(Box<Self>, f32),
    Un(Box<Self>, UnOp),
    Zip(BinOp, Box<Self>, Box<Self>),
    Square(BinOp, S, S),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Abs,
}

impl UnOp {
    pub fn on<F>(self, field: Field<F>) -> Field<F>
    where
        F: FieldTrait,
    {
        match (self, field) {
            (op, Field::Uniform(f)) => Field::Uniform(f.un_op(op)),
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
    pub fn zip<F>(self, a: Field<F>, b: Field<F>) -> Field<F>
    where
        F: FieldTrait,
    {
        match (self, a, b) {
            (op, Field::Uniform(a), Field::Uniform(b)) => Field::Uniform(a.zip(op, b)),
            (op, a, b) => Field::Zip(op, a.into(), b.into()),
        }
    }
    pub fn square<F>(self, a: F, b: F) -> Field<F>
    where
        F: FieldTrait,
    {
        Field::Square(self, a, b)
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

impl FieldTrait for f32 {
    type Sample = f32;
    type Squared = Field1;
    fn uniform(x: f32) -> Self {
        x
    }
    fn sample(&self, _x: f32) -> Cow<Self::Sample> {
        Cow::Owned(*self)
    }
    fn range(&self) -> Option<RangeInclusive<f32>> {
        Some(0.0..=0.0)
    }
    fn un_op(self, op: UnOp) -> Self {
        op.operate(self)
    }
    fn zip(self, op: BinOp, other: Self) -> Self {
        op.operate(self, other)
    }
    fn square(self, op: BinOp, other: Self) -> Self::Squared {
        Field1::Uniform(op.operate(self, other))
    }
    fn superuniform(x: f32) -> Self {
        x
    }
}

impl<S> FieldTrait for Field<S>
where
    S: FieldTrait + Clone,
{
    type Sample = S;
    type Squared = Field<Self>;
    fn uniform(f: Self::Sample) -> Self {
        Field::Uniform(f)
    }
    fn sample(&self, x: f32) -> Cow<Self::Sample> {
        match self {
            Field::Identity => Cow::Owned(S::superuniform(x)),
            Field::Uniform(f) => Cow::Borrowed(f),
            Field::List(list) => list
                .get(x.round() as usize)
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(S::superuniform(0.0))),
            Field::Offset(field, by) => field.sample(x + *by),
            Field::Un(field, op) => Cow::Owned(field.sample(x).into_owned().un_op(*op)),
            Field::Zip(op, a, b) => {
                Cow::Owned(a.sample(x).into_owned().zip(*op, b.sample(x).into_owned()))
            }
            Field::Square(op, a, b) => {
                Cow::Owned(S::uniform(a.sample(x).into_owned()).zip(*op, b.clone()))
            }
        }
    }
    fn range(&self) -> Option<RangeInclusive<f32>> {
        match self {
            Field::Identity => None,
            Field::Uniform(_) => Some(0.0..=0.0),
            Field::List(list) => Some(0.0..=(list.len() - 1) as f32),
            Field::Offset(field, by) => field
                .range()
                .map(|range| (*range.start() + *by..=*range.end() + *by)),
            Field::Un(field, _) => field.range(),
            Field::Zip(_, a, b) => match (a.range(), b.range()) {
                (None, None) => None,
                (None, Some(range)) | (Some(range), None) => Some(range),
                (Some(a), Some(b)) => Some(a.start().min(*b.start())..=a.end().max(*b.end())),
            },
            Field::Square(_, a, _) => a.range(),
        }
    }
    fn un_op(self, op: UnOp) -> Self {
        op.on(self)
    }
    fn zip(self, op: BinOp, other: Self) -> Self {
        op.zip(self, other)
    }
    fn square(self, op: BinOp, other: Self) -> Self::Squared {
        Field::Square(op, self, other)
    }
}

impl<S> Field<S>
where
    S: FieldTrait,
{
    pub fn list(items: impl IntoIterator<Item = S>) -> Self {
        Field::List(items.into_iter().collect())
    }
}

impl<S> Neg for Field<S>
where
    S: FieldTrait,
{
    type Output = Self;
    fn neg(self) -> Self::Output {
        UnOp::Neg.on(self)
    }
}

macro_rules! bin_op {
    ($trait:ident, $method:ident) => {
        impl<S> $trait for Field<S>
        where
            S: FieldTrait,
        {
            type Output = Self;
            fn $method(self, other: Self) -> Self::Output {
                BinOp::$trait.zip(self, other)
            }
        }

        impl<S> $trait<S> for Field<S>
        where
            S: FieldTrait,
        {
            type Output = Self;
            fn $method(self, other: S) -> Self::Output {
                BinOp::$trait.zip(self, Field::Uniform(other))
            }
        }
    };
}

bin_op!(Add, add);
bin_op!(Sub, sub);
bin_op!(Mul, mul);
bin_op!(Div, div);
