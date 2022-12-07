use derive_more::{Display, From};
use eframe::epaint::{Pos2, Vec2};
use enum_iterator::Sequence;

use crate::{function::*, world::World};

#[derive(Debug, Clone, From)]
pub enum GenericField {
    #[from(types(f32))]
    Scalar(ScalarField),
    #[from(types(Vec2))]
    Vector(VectorField),
}

impl GenericField {
    pub fn ty(&self) -> Type {
        match self {
            GenericField::Scalar(_) => Type::Scalar,
            GenericField::Vector(_) => Type::Vector,
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Scalar,
    Vector,
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
pub enum ScalarField {
    Uniform(f32),
    X,
    Y,
    ScalarUn(UnOp<ScalarUnOp>, Box<Self>),
    VectorUn(VectorUnScalarOp, Box<VectorField>),
    Bin(BinOp<HomoBinOp>, Box<Self>, Box<Self>),
    Index(Box<VectorField>, Box<Self>),
    World(GenericScalarFieldKind),
}

#[derive(Debug, Clone, From)]
pub enum VectorField {
    Uniform(Vec2),
    Un(UnOp<VectorUnVectorOp>, Box<Self>),
    BinSV(BinOp<NoOp<Vec2>>, ScalarField, Box<Self>),
    BinVS(BinOp<NoOp<Vec2>>, Box<Self>, ScalarField),
    BinVV(BinOp<HomoBinOp>, Box<Self>, Box<Self>),
    Index(Box<Self>, Box<Self>),
    World(GenericVectorFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Sequence)]
pub enum GenericFieldKind {
    #[from(types(ScalarInputFieldKind, ScalarOutputFieldKind))]
    Scalar(GenericScalarFieldKind),
    #[from(types(VectorInputFieldKind, VectorOutputFieldKind))]
    Vector(GenericVectorFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Sequence)]
pub enum GenericInputFieldKind {
    Scalar(ScalarInputFieldKind),
    Vector(VectorInputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Sequence)]
pub enum GenericOutputFieldKind {
    Scalar(ScalarOutputFieldKind),
    Vector(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Sequence)]
pub enum GenericScalarFieldKind {
    Input(ScalarInputFieldKind),
    Output(ScalarOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Sequence)]
pub enum GenericVectorFieldKind {
    Input(VectorInputFieldKind),
    Output(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum ScalarInputFieldKind {
    #[display(fmt = "ρ Density")]
    Density,
    #[display(fmt = "🗻Elevation")]
    Elevation,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum VectorInputFieldKind {}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum ScalarOutputFieldKind {}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Sequence)]
pub enum VectorOutputFieldKind {
    #[display(fmt = "↗ Force")]
    Force,
}

impl ScalarField {
    pub fn sample(&self, world: &World, pos: Pos2) -> f32 {
        match self {
            ScalarField::Uniform(v) => *v,
            ScalarField::X => pos.x - world.player_pos.x,
            ScalarField::Y => pos.y - world.player_pos.y,
            ScalarField::ScalarUn(op, field) => op.operate(field.sample(world, pos)),
            ScalarField::VectorUn(op, field) => op.operate(field.sample(world, pos)),
            ScalarField::Bin(op, a, b) => op.operate(a.sample(world, pos), b.sample(world, pos)),
            ScalarField::Index(index, field) => {
                field.sample(world, index.sample(world, pos).to_pos2())
            }
            ScalarField::World(kind) => world.sample_scalar_field(*kind, pos),
        }
    }
    fn uniform(&self) -> Option<f32> {
        match self {
            ScalarField::Uniform(n) => Some(*n),
            _ => None,
        }
    }
    pub fn reduce(self) -> Self {
        match self {
            ScalarField::ScalarUn(op, field) => {
                if let Some(n) = field.uniform() {
                    ScalarField::Uniform(op.operate(n))
                } else {
                    ScalarField::ScalarUn(op, field)
                }
            }
            ScalarField::Bin(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    ScalarField::Uniform(op.operate(a, b))
                } else {
                    ScalarField::Bin(op, a, b)
                }
            }
            ScalarField::VectorUn(op, field) => {
                if let Some(n) = field.uniform() {
                    ScalarField::Uniform(op.operate(n))
                } else {
                    ScalarField::VectorUn(op, field)
                }
            }
            field => field,
        }
    }
}

impl VectorField {
    pub fn sample(&self, world: &World, pos: Pos2) -> Vec2 {
        match self {
            VectorField::Uniform(v) => *v,
            VectorField::Un(op, field) => op.operate(field.sample(world, pos)),
            VectorField::BinSV(op, a, b) => op.operate(a.sample(world, pos), b.sample(world, pos)),
            VectorField::BinVS(op, a, b) => op.operate(a.sample(world, pos), b.sample(world, pos)),
            VectorField::BinVV(op, a, b) => op.operate(a.sample(world, pos), b.sample(world, pos)),
            VectorField::Index(index, field) => {
                field.sample(world, index.sample(world, pos).to_pos2())
            }
            VectorField::World(kind) => world.sample_vector_field(*kind, pos),
        }
    }
    fn uniform(&self) -> Option<Vec2> {
        match self {
            VectorField::Uniform(v) => Some(*v),
            _ => None,
        }
    }
    pub fn reduce(self) -> Self {
        match self {
            VectorField::Un(op, field) => {
                if let Some(v) = field.uniform() {
                    VectorField::Uniform(op.operate(v))
                } else {
                    VectorField::Un(op, field)
                }
            }
            VectorField::BinSV(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    VectorField::Uniform(op.operate(a, b))
                } else {
                    VectorField::BinSV(op, a, b)
                }
            }
            VectorField::BinVV(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    VectorField::Uniform(op.operate(a, b))
                } else {
                    VectorField::BinVV(op, a, b)
                }
            }
            VectorField::BinVS(op, a, b) => {
                if let (Some(a), Some(b)) = (a.uniform(), b.uniform()) {
                    VectorField::Uniform(op.operate(a, b))
                } else {
                    VectorField::BinVS(op, a, b)
                }
            }
            field => field,
        }
    }
}
