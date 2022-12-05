use std::fmt;

use derive_more::{Display, From};
use eframe::epaint::Vec2;

use crate::{field::*, function::*};

#[derive(Debug, Clone, From)]
pub enum Value<'a> {
    #[from(types(
        f32,
        Vec2,
        "ScalarField<'a>",
        "VectorField<'a>",
        "CommonField<f32>",
        "CommonField<Vec2>"
    ))]
    Field(GenericField<'a>),
    #[from]
    Function(Function),
}

impl<'a> Default for Value<'a> {
    fn default() -> Self {
        0.0.into()
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueType {
    Scalar,
    Vector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Value(ValueType),
    Field(ValueType),
    Function(Function),
}

impl Type {
    pub fn is_field(&self) -> bool {
        matches!(self, Type::Field(_))
    }
    pub fn is_function(&self) -> bool {
        matches!(self, Type::Function(_))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Value(ValueType::Scalar) => "Scalar".fmt(f),
            Type::Value(ValueType::Vector) => "Vector".fmt(f),
            Type::Field(ValueType::Scalar) => "Scalar Field".fmt(f),
            Type::Field(ValueType::Vector) => "Vector Field".fmt(f),
            Type::Function(function) => function.fmt(f),
        }
    }
}

impl<'a> Value<'a> {
    pub fn ty(&self) -> Type {
        match self {
            Value::Field(GenericField::Scalar(_)) => Type::Field(ValueType::Scalar),
            Value::Field(GenericField::Vector(_)) => Type::Field(ValueType::Vector),
            Value::Function(f) => Type::Function(*f),
        }
    }
    #[track_caller]
    pub fn unwrap_field(self) -> GenericField<'a> {
        if let Value::Field(field) = self {
            field
        } else {
            panic!("Value expected to be a field was a {} instead", self.ty())
        }
    }
}
