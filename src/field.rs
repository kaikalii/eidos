use derive_more::{Display, From};
use eframe::epaint::{Pos2, Vec2};
use enum_iterator::Sequence;
use serde::Deserialize;

use crate::{function::*, person::PersonId, world::World};

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
    #[from]
    Uniform(f32),
    X(PersonId),
    Y(PersonId),
    TargetX(PersonId),
    TargetY(PersonId),
    ScalarUn(UnOp<ScalarUnOp>, Box<Self>),
    VectorUn(VectorUnScalarOp, Box<VectorField>),
    Bin(BinOp<HomoBinOp>, Box<Self>, Box<Self>),
    Index(Box<VectorField>, Box<Self>),
    #[from]
    Input(ScalarInputFieldKind),
    #[from]
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
    Input(VectorInputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
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

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum GenericInputFieldKind {
    Scalar(ScalarInputFieldKind),
    Vector(VectorInputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum GenericOutputFieldKind {
    Scalar(ScalarOutputFieldKind),
    Vector(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum GenericScalarFieldKind {
    Input(ScalarInputFieldKind),
    Output(ScalarOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
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
    #[display(fmt = "🌌Magic")]
    Magic,
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

const DROP_OFF_FACTOR: f32 = 10.0;

impl ScalarField {
    pub fn sample_relative(&self, world: &World, caster: PersonId, pos: Pos2) -> f32 {
        let person_pos = world.person(caster).pos;
        self.sample_absolute(world, pos)
            / (1.0
                + ((pos.x - person_pos.x).powf(2.0) + (pos.y - person_pos.y).powf(2.0))
                    / DROP_OFF_FACTOR)
    }
    pub fn sample_absolute(&self, world: &World, pos: Pos2) -> f32 {
        puffin::profile_function!();
        match self {
            ScalarField::Uniform(v) => *v,
            ScalarField::X(person_id) => pos.x - world.person(*person_id).pos.x,
            ScalarField::Y(person_id) => pos.y - world.person(*person_id).pos.y,
            ScalarField::TargetX(person_id) => {
                let person = world.person(*person_id);
                person.pos.x + person.target.unwrap_or_default().x - pos.x
            }
            ScalarField::TargetY(person_id) => {
                let person = world.person(*person_id);
                person.pos.y + person.target.unwrap_or_default().y - pos.y
            }
            ScalarField::ScalarUn(op, field) => op.operate(field.sample_absolute(world, pos)),
            ScalarField::VectorUn(op, field) => op.operate(field.sample_absolute(world, pos)),
            ScalarField::Bin(op, a, b) => {
                op.operate(a.sample_absolute(world, pos), b.sample_absolute(world, pos))
            }
            ScalarField::Index(index, field) => {
                field.sample_absolute(world, index.sample_absolute(world, pos).to_pos2())
            }
            ScalarField::Input(kind) => world.sample_input_scalar_field(*kind, pos),
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
    pub fn sample_relative(&self, world: &World, caster: PersonId, pos: Pos2) -> Vec2 {
        let person_pos = world.person(caster).pos;
        self.sample_absolute(world, pos)
            / (1.0
                + ((pos.x - person_pos.x).powf(2.0) + (pos.y - person_pos.y).powf(2.0))
                    / DROP_OFF_FACTOR)
    }
    pub fn sample_absolute(&self, world: &World, pos: Pos2) -> Vec2 {
        puffin::profile_function!();
        match self {
            VectorField::Uniform(v) => *v,
            VectorField::Un(op, field) => op.operate(field.sample_absolute(world, pos)),
            VectorField::BinSV(op, a, b) => {
                op.operate(a.sample_absolute(world, pos), b.sample_absolute(world, pos))
            }
            VectorField::BinVS(op, a, b) => {
                op.operate(a.sample_absolute(world, pos), b.sample_absolute(world, pos))
            }
            VectorField::BinVV(op, a, b) => {
                op.operate(a.sample_absolute(world, pos), b.sample_absolute(world, pos))
            }
            VectorField::Index(index, field) => {
                field.sample_absolute(world, index.sample_absolute(world, pos).to_pos2())
            }
            VectorField::Input(kind) => world.sample_input_vector_field(*kind, pos),
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
