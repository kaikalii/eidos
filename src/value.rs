use std::fmt;

use derive_more::From;
use eframe::epaint::Vec2;

use crate::{CommonField, Function, GenericField, ScalarField, VectorField};

#[derive(Debug, Clone, From)]
pub enum Value {
    #[from]
    Scalar(f32),
    #[from]
    Vector(Vec2),
    #[from(types(ScalarField, VectorField, "CommonField<f32>", "CommonField<Vec2>"))]
    Field(GenericField),
    #[from]
    Function(Function),
}

impl Default for Value {
    fn default() -> Self {
        0.0.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldType {
    Scalar,
    Vector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Value(FieldType),
    Field(FieldType),
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
            Type::Value(FieldType::Scalar) => "Scalar".fmt(f),
            Type::Value(FieldType::Vector) => "Vector".fmt(f),
            Type::Field(FieldType::Scalar) => "Scalar Field".fmt(f),
            Type::Field(FieldType::Vector) => "Vector Field".fmt(f),
            Type::Function(function) => function.fmt(f),
        }
    }
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Scalar(_) => Type::Value(FieldType::Scalar),
            Value::Vector(_) => Type::Value(FieldType::Vector),
            Value::Field(GenericField::Scalar(_)) => Type::Field(FieldType::Scalar),
            Value::Field(GenericField::Vector(_)) => Type::Field(FieldType::Vector),
            Value::Function(f) => Type::Function(*f),
        }
    }
    #[track_caller]
    pub fn unwrap_field(self) -> GenericField {
        if let Value::Field(field) = self {
            field
        } else {
            panic!("Value expected to be a field was a {} instead", self.ty())
        }
    }
}
