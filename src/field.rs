use std::fmt;

use derive_more::From;
use eframe::epaint::Vec2;

use crate::{
    BinOp, BinOperator, HomoBinOp, NoOp, ScalarUnOp, UnOp, UnOperator, VectorUnScalarOp,
    VectorUnVectorOp,
};

#[derive(Debug, Clone, From)]
pub enum GenericField {
    #[from(types("CommonField<f32>"))]
    Scalar(ScalarField),
    #[from(types("CommonField<Vec2>"))]
    Vector(VectorField),
}

pub trait FieldValue: Copy + Default {
    fn x(x: f32) -> Self;
    fn y(y: f32) -> Self;
}

impl FieldValue for f32 {
    fn x(x: f32) -> Self {
        x
    }
    fn y(y: f32) -> Self {
        y
    }
}

impl FieldValue for Vec2 {
    fn x(x: f32) -> Self {
        Vec2::X * x
    }
    fn y(y: f32) -> Self {
        Vec2::Y * y
    }
}

#[derive(Debug, Clone, From)]
pub enum CommonField<T> {
    Uniform(T),
    X,
    Y,
    Array(Vec<Vec<T>>),
}

#[derive(Debug, Clone, From)]
pub enum ScalarField {
    Common(CommonField<f32>),
    ScalarUn(UnOp<ScalarUnOp>, Box<Self>),
    VectorUn(VectorUnScalarOp, Box<VectorField>),
    Bin(BinOp<HomoBinOp>, Box<Self>, Box<Self>),
}

#[derive(Debug, Clone, From)]
pub enum VectorField {
    Common(CommonField<Vec2>),
    Un(UnOp<VectorUnVectorOp>, Box<Self>),
    BinSV(BinOp<NoOp<Vec2>>, Box<ScalarField>, Box<Self>),
    BinVS(BinOp<NoOp<Vec2>>, Box<Self>, Box<ScalarField>),
    BinVV(BinOp<HomoBinOp>, Box<Self>, Box<Self>),
}

impl<T> CommonField<T>
where
    T: FieldValue,
{
    pub fn array<const N: usize>(columns: impl IntoIterator<Item = [impl Into<T>; N]>) -> Self {
        CommonField::Array(
            columns
                .into_iter()
                .map(|arr| arr.map(Into::into).into())
                .collect(),
        )
    }
    pub fn sample(&self, x: f32, y: f32) -> T {
        match self {
            CommonField::Uniform(k) => *k,
            CommonField::X => T::x(x),
            CommonField::Y => T::y(y),
            CommonField::Array(data) => {
                let x = x.round();
                let y = y.round();
                if x < 0.0 || y < 0.0 {
                    return T::default();
                }
                data.get(x as usize)
                    .and_then(|column| column.get(y as usize as usize))
                    .copied()
                    .unwrap_or_default()
            }
        }
    }
}

impl ScalarField {
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        match self {
            ScalarField::Common(field) => field.sample(x, y),
            ScalarField::ScalarUn(op, field) => op.operate(field.sample(x, y)),
            ScalarField::VectorUn(op, field) => op.operate(field.sample(x, y)),
            ScalarField::Bin(op, a, b) => op.operate(a.sample(x, y), b.sample(x, y)),
        }
    }
}

impl VectorField {
    pub fn sample(&self, x: f32, y: f32) -> Vec2 {
        match self {
            VectorField::Common(field) => field.sample(x, y),
            VectorField::Un(op, field) => op.operate(field.sample(x, y)),
            VectorField::BinSV(op, a, b) => op.operate(a.sample(x, y), b.sample(x, y)),
            VectorField::BinVS(op, a, b) => op.operate(a.sample(x, y), b.sample(x, y)),
            VectorField::BinVV(op, a, b) => op.operate(a.sample(x, y), b.sample(x, y)),
        }
    }
}

impl<T> fmt::Display for CommonField<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommonField::Array(_) => "Array".fmt(f),
            CommonField::X => "X".fmt(f),
            CommonField::Y => "Y".fmt(f),
            CommonField::Uniform(k) => write!(f, "{k:?}"),
        }
    }
}
