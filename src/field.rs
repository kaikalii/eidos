use derive_more::{Display, From};
use eframe::epaint::{Pos2, Vec2};
use enum_iterator::Sequence;
use serde::Deserialize;

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
    pub fn controls(&self) -> Vec<ControlKind> {
        match self {
            GenericField::Scalar(field) => field.controls(),
            GenericField::Vector(field) => field.controls(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Scalar,
    Vector,
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
    Control(ControlKind),
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

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
pub enum GenericFieldKind {
    #[from(types(ScalarInputFieldKind, ScalarOutputFieldKind))]
    Scalar(GenericScalarFieldKind),
    #[from(types(VectorInputFieldKind, VectorOutputFieldKind))]
    Vector(GenericVectorFieldKind),
}

impl From<GenericInputFieldKind> for GenericFieldKind {
    fn from(kind: GenericInputFieldKind) -> Self {
        match kind {
            GenericInputFieldKind::Scalar(kind) => kind.into(),
            GenericInputFieldKind::Vector(kind) => kind.into(),
        }
    }
}

impl From<GenericOutputFieldKind> for GenericFieldKind {
    fn from(kind: GenericOutputFieldKind) -> Self {
        match kind {
            GenericOutputFieldKind::Scalar(kind) => kind.into(),
            GenericOutputFieldKind::Vector(kind) => kind.into(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence)]
pub enum GenericInputFieldKind {
    Scalar(ScalarInputFieldKind),
    Vector(VectorInputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence)]
pub enum GenericOutputFieldKind {
    Scalar(ScalarOutputFieldKind),
    Vector(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
pub enum GenericScalarFieldKind {
    Input(ScalarInputFieldKind),
    Output(ScalarOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
pub enum GenericVectorFieldKind {
    Input(VectorInputFieldKind),
    Output(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum ScalarInputFieldKind {
    #[display(fmt = "ρ Density")]
    Density,
    #[display(fmt = "🗻Elevation")]
    Elevation,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum VectorInputFieldKind {}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum ScalarOutputFieldKind {}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum VectorOutputFieldKind {
    #[display(fmt = "↗ Force")]
    Force,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControlKind {
    XSlider,
    YSlider,
}

impl ScalarField {
    pub fn sample(&self, world: &World, pos: Pos2) -> f32 {
        puffin::profile_function!();
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
            ScalarField::Control(kind) => world.controls.get(*kind),
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
    pub fn controls(&self) -> Vec<ControlKind> {
        match self {
            ScalarField::ScalarUn(_, field) => field.controls(),
            ScalarField::VectorUn(_, field) => field.controls(),
            ScalarField::Bin(_, a, b) => {
                [a.controls(), b.controls()].into_iter().flatten().collect()
            }
            ScalarField::Index(a, b) => {
                [a.controls(), b.controls()].into_iter().flatten().collect()
            }
            ScalarField::Control(kind) => vec![*kind],
            _ => Vec::new(),
        }
    }
}

impl VectorField {
    pub fn sample(&self, world: &World, pos: Pos2) -> Vec2 {
        puffin::profile_function!();
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
    pub fn controls(&self) -> Vec<ControlKind> {
        match self {
            VectorField::Un(_, field) => field.controls(),
            VectorField::BinSV(_, a, b) => {
                [a.controls(), b.controls()].into_iter().flatten().collect()
            }
            VectorField::BinVS(_, a, b) => {
                [a.controls(), b.controls()].into_iter().flatten().collect()
            }
            VectorField::BinVV(_, a, b) => {
                [a.controls(), b.controls()].into_iter().flatten().collect()
            }
            VectorField::Index(a, b) => {
                [a.controls(), b.controls()].into_iter().flatten().collect()
            }
            _ => Vec::new(),
        }
    }
}
