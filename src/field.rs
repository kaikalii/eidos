use derive_more::{Display, From};
use eframe::epaint::{Pos2, Vec2};
use enum_iterator::Sequence;
use serde::Deserialize;

use crate::{function::*, person::PersonId, world::World};

#[derive(Debug, Clone, From)]
pub enum Field {
    #[from(types(f32))]
    Scalar(ScalarField),
    #[from(types(Vec2))]
    Vector(VectorField),
}

impl Field {
    pub fn ty(&self) -> Type {
        match self {
            Field::Scalar(_) => Type::Scalar,
            Field::Vector(_) => Type::Vector,
        }
    }
    pub fn controls(&self) -> Vec<ControlKind> {
        match self {
            Field::Scalar(field) => field.controls(),
            Field::Vector(field) => field.controls(),
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
    X,
    Y,
    TargetX(PersonId),
    TargetY(PersonId),
    ScalarUn(TypedUnOp<ScalarUnOp>, Box<Self>),
    VectorUn(VectorUnScalarOp, Box<VectorField>),
    Bin(TypedBinOp<HomoBinOp>, Box<Self>, Box<Self>),
    Index(Box<VectorField>, Box<Self>),
    #[from]
    Input(ScalarInputFieldKind),
    #[from]
    Control(ControlKind),
    Variable,
}

#[derive(Debug, Clone, From)]
pub enum VectorField {
    Uniform(Vec2),
    VectorUn(TypedUnOp<VectorUnVectorOp>, Box<Self>),
    ScalarUn(ScalarUnVectorOp, Box<ScalarField>),
    BinSV(TypedBinOp<NoOp<Vec2>>, ScalarField, Box<Self>),
    BinVS(TypedBinOp<NoOp<Vec2>>, Box<Self>, ScalarField),
    BinVV(TypedBinOp<HomoBinOp>, Box<Self>, Box<Self>),
    Index(Box<Self>, Box<Self>),
    Input(VectorInputFieldKind),
    Variable,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum FieldKind {
    #[from(types(ScalarInputFieldKind, ScalarOutputFieldKind))]
    Scalar(ScalarFieldKind),
    #[from(types(VectorInputFieldKind, VectorOutputFieldKind))]
    Vector(VectorFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum IoFieldKind {
    #[from(types(ScalarInputFieldKind, VectorInputFieldKind))]
    Input(InputFieldKind),
    #[from(types(ScalarOutputFieldKind, VectorOutputFieldKind))]
    Output(OutputFieldKind),
}

impl From<InputFieldKind> for FieldKind {
    fn from(kind: InputFieldKind) -> Self {
        match kind {
            InputFieldKind::Scalar(kind) => kind.into(),
            InputFieldKind::Vector(kind) => kind.into(),
        }
    }
}

impl From<OutputFieldKind> for FieldKind {
    fn from(kind: OutputFieldKind) -> Self {
        match kind {
            OutputFieldKind::Scalar(kind) => kind.into(),
            OutputFieldKind::Vector(kind) => kind.into(),
        }
    }
}

impl From<FieldKind> for IoFieldKind {
    fn from(kind: FieldKind) -> Self {
        match kind {
            FieldKind::Scalar(ScalarFieldKind::Input(kind)) => kind.into(),
            FieldKind::Scalar(ScalarFieldKind::Output(kind)) => kind.into(),
            FieldKind::Vector(VectorFieldKind::Input(kind)) => kind.into(),
            FieldKind::Vector(VectorFieldKind::Output(kind)) => kind.into(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum InputFieldKind {
    Scalar(ScalarInputFieldKind),
    Vector(VectorInputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum OutputFieldKind {
    Scalar(ScalarOutputFieldKind),
    Vector(VectorOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum ScalarFieldKind {
    Input(ScalarInputFieldKind),
    Output(ScalarOutputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum VectorFieldKind {
    Output(VectorOutputFieldKind),
    Input(VectorInputFieldKind),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum ScalarInputFieldKind {
    #[display(fmt = "ρ Density")]
    Density,
    #[display(fmt = "🗻Elevation")]
    Elevation,
    #[display(fmt = "🌡Temperature")]
    Temperature,
    #[display(fmt = "🍃Disorder")]
    Disorder,
    #[display(fmt = "🌌Magic")]
    Magic,
    #[display(fmt = "🕯Light")]
    Light,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum VectorInputFieldKind {}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum ScalarOutputFieldKind {
    #[display(fmt = "🔥Heat")]
    Heat,
    #[display(fmt = "🗄Order")]
    Order,
    #[display(fmt = "⚓Anchor")]
    Anchor,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum VectorOutputFieldKind {
    #[display(fmt = "⬇ Gravity")]
    Gravity,
    #[display(fmt = "↗ Force")]
    Force,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControlKind {
    XSlider,
    YSlider,
    Activation,
}

impl ScalarField {
    pub fn sample(&self, world: &World, pos: Pos2, allow_recursion: bool) -> f32 {
        puffin::profile_function!();
        match self {
            ScalarField::Uniform(v) => *v,
            ScalarField::X => pos.x,
            ScalarField::Y => pos.y,
            ScalarField::TargetX(person_id) => {
                let person = world.person(*person_id);
                let Some(target) = person.target else {
                    return 0.0;
                };
                target.x - pos.x
            }
            ScalarField::TargetY(person_id) => {
                let person = world.person(*person_id);
                let Some(target) = person.target else {
                    return 0.0;
                };
                target.y - pos.y
            }
            ScalarField::ScalarUn(op, field) => {
                op.operate(field.sample(world, pos, allow_recursion))
            }
            ScalarField::VectorUn(op, field) => {
                op.operate(field.sample(world, pos, allow_recursion))
            }
            ScalarField::Bin(op, a, b) => op.operate(
                a.sample(world, pos, allow_recursion),
                b.sample(world, pos, allow_recursion),
            ),
            ScalarField::Index(index, field) => field.sample(
                world,
                index.sample(world, pos, allow_recursion).to_pos2(),
                allow_recursion,
            ),
            ScalarField::Input(kind) => {
                world.sample_input_scalar_field(*kind, pos, allow_recursion)
            }
            ScalarField::Control(kind) => world.controls.get(*kind),
            ScalarField::Variable => pos.to_vec2().length(),
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
    pub fn derivative_at(&self, world: &World, pos: Pos2, allow_recursion: bool) -> Vec2 {
        const RANGE: f32 = 0.1;
        let left_x = self.sample(world, pos - Vec2::X * RANGE, allow_recursion);
        let right_x = self.sample(world, pos + Vec2::X * RANGE, allow_recursion);
        let down_y = self.sample(world, pos - Vec2::Y * RANGE, allow_recursion);
        let up_y = self.sample(world, pos + Vec2::Y * RANGE, allow_recursion);
        Vec2::new(right_x - left_x, up_y - down_y) / (2.0 * RANGE)
    }
}

impl VectorField {
    pub fn sample(&self, world: &World, pos: Pos2, allow_recursion: bool) -> Vec2 {
        puffin::profile_function!();
        match self {
            VectorField::Uniform(v) => *v,
            VectorField::VectorUn(op, field) => {
                op.operate(field.sample(world, pos, allow_recursion))
            }
            VectorField::ScalarUn(op, field) => match op {
                ScalarUnVectorOp::Derivative => field.derivative_at(world, pos, allow_recursion),
            },
            VectorField::BinSV(op, a, b) => op.operate(
                a.sample(world, pos, allow_recursion),
                b.sample(world, pos, allow_recursion),
            ),
            VectorField::BinVS(op, a, b) => op.operate(
                a.sample(world, pos, allow_recursion),
                b.sample(world, pos, allow_recursion),
            ),
            VectorField::BinVV(op, a, b) => op.operate(
                a.sample(world, pos, allow_recursion),
                b.sample(world, pos, allow_recursion),
            ),
            VectorField::Index(index, field) => field.sample(
                world,
                index.sample(world, pos, allow_recursion).to_pos2(),
                allow_recursion,
            ),
            VectorField::Input(kind) => world.sample_input_vector_field(*kind, pos),
            VectorField::Variable => pos.to_vec2(),
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
            VectorField::VectorUn(op, field) => {
                if let Some(v) = field.uniform() {
                    VectorField::Uniform(op.operate(v))
                } else {
                    VectorField::VectorUn(op, field)
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
            VectorField::VectorUn(_, field) => field.controls(),
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
