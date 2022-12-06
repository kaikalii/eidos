use std::fmt;

use derive_more::{Display, From};
use eframe::epaint::Vec2;
use enum_iterator::Sequence;

use crate::{function::*, world::World};

#[derive(Debug, Clone, From)]
pub enum GenericField {
    #[from(types(f32, "CommonField<f32>"))]
    Scalar(ScalarField),
    #[from(types(Vec2, "CommonField<Vec2>"))]
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
    #[from(types(f32))]
    Common(CommonField<f32>),
    ScalarUn(UnOp<ScalarUnOp>, Box<Self>),
    VectorUn(VectorUnScalarOp, Box<VectorField>),
    Bin(BinOp<HomoBinOp>, Box<Self>, Box<Self>),
    World(ScalarInputFieldKind),
}

#[derive(Debug, Clone, From)]
pub enum VectorField {
    #[from(types(Vec2))]
    Common(CommonField<Vec2>),
    Un(UnOp<VectorUnVectorOp>, Box<Self>),
    BinSV(BinOp<NoOp<Vec2>>, ScalarField, Box<Self>),
    BinVS(BinOp<NoOp<Vec2>>, Box<Self>, ScalarField),
    BinVV(BinOp<HomoBinOp>, Box<Self>, Box<Self>),
    World(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum FieldKind<T> {
    Uncasted,
    Typed(T),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum GenericInputFieldKind {
    Scalar(ScalarInputFieldKind),
    Vector(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum GenericOutputFieldKind {
    Vector(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum ScalarInputFieldKind {
    Density,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum VectorOutputFieldKind {
    Force,
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
    pub fn sample(&self, world: &World, x: f32, y: f32) -> f32 {
        match self {
            ScalarField::Common(field) => field.sample(x, y),
            ScalarField::ScalarUn(op, field) => op.operate(field.sample(world, x, y)),
            ScalarField::VectorUn(op, field) => op.operate(field.sample(world, x, y)),
            ScalarField::Bin(op, a, b) => op.operate(a.sample(world, x, y), b.sample(world, x, y)),
            ScalarField::World(kind) => world.sample_scalar_field(*kind, x, y),
        }
    }
    fn uniform(&self) -> Option<f32> {
        match self {
            ScalarField::Common(CommonField::Uniform(n)) => Some(*n),
            _ => None,
        }
    }
    pub fn reduce(self) -> Self {
        match self {
            ScalarField::ScalarUn(op, field) => {
                if let Some(n) = field.uniform() {
                    CommonField::Uniform(op.operate(n)).into()
                } else {
                    ScalarField::ScalarUn(op, field)
                }
            }
            ScalarField::Bin(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    CommonField::Uniform(op.operate(a, b)).into()
                } else {
                    ScalarField::Bin(op, a, b)
                }
            }
            ScalarField::VectorUn(op, field) => {
                if let Some(n) = field.uniform() {
                    CommonField::Uniform(op.operate(n)).into()
                } else {
                    ScalarField::VectorUn(op, field)
                }
            }
            field => field,
        }
    }
}

impl VectorField {
    pub fn sample(&self, world: &World, x: f32, y: f32) -> Vec2 {
        match self {
            VectorField::Common(field) => field.sample(x, y),
            VectorField::Un(op, field) => op.operate(field.sample(world, x, y)),
            VectorField::BinSV(op, a, b) => {
                op.operate(a.sample(world, x, y), b.sample(world, x, y))
            }
            VectorField::BinVS(op, a, b) => {
                op.operate(a.sample(world, x, y), b.sample(world, x, y))
            }
            VectorField::BinVV(op, a, b) => {
                op.operate(a.sample(world, x, y), b.sample(world, x, y))
            }
            VectorField::World(kind) => world.sample_vector_field(*kind, x, y),
        }
    }
    fn uniform(&self) -> Option<Vec2> {
        match self {
            VectorField::Common(CommonField::Uniform(v)) => Some(*v),
            _ => None,
        }
    }
    pub fn reduce(self) -> Self {
        match self {
            VectorField::Un(op, field) => {
                if let Some(v) = field.uniform() {
                    CommonField::Uniform(op.operate(v)).into()
                } else {
                    VectorField::Un(op, field)
                }
            }
            VectorField::BinSV(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    CommonField::Uniform(op.operate(a, b)).into()
                } else {
                    VectorField::BinSV(op, a, b)
                }
            }
            VectorField::BinVV(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    CommonField::Uniform(op.operate(a, b)).into()
                } else {
                    VectorField::BinVV(op, a, b)
                }
            }
            VectorField::BinVS(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    CommonField::Uniform(op.operate(a, b)).into()
                } else {
                    VectorField::BinVS(op, a, b)
                }
            }
            field => field,
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
