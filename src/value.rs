use std::fmt;

use crate::{Field, Function};

#[derive(Debug, Clone)]
pub enum Value {
    Field(Field),
    Function(Function),
}

impl Default for Value {
    fn default() -> Self {
        0.0.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Field(usize),
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
            Type::Field(rank) => match rank {
                0 => "Scalar".fmt(f),
                n => write!(f, "{n}D Field"),
            },
            Type::Function(function) => function.fmt(f),
        }
    }
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Field(f) => Type::Field(f.rank()),
            Value::Function(f) => Type::Function(*f),
        }
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Field::uniform(f).into()
    }
}

macro_rules! value_from {
    ($variant:ident, $ty:ty) => {
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Value::$variant(value)
            }
        }
    };
}

value_from!(Field, Field);
value_from!(Function, Function);
