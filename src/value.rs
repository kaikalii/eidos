use std::fmt;

use crate::{Field1, Field2, Function};

#[derive(Debug, Clone)]
pub enum Value {
    Atom(f32),
    F1(Field1),
    F2(Field2),
    Function(Function),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Field(u8),
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
            Value::Atom(_) => Type::Field(0),
            Value::F1(_) => Type::Field(1),
            Value::F2(_) => Type::Field(2),
            Value::Function(f) => Type::Function(f.clone()),
        }
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

value_from!(Atom, f32);
value_from!(F1, Field1);
value_from!(F2, Field2);
value_from!(Function, Function);
