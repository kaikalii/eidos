use std::fmt;

use enum_iterator::Sequence;

use crate::{BinOp, Type, UnOp, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Function {
    Un(UnOp),
    Bin(BinOp),
}

impl Function {
    pub fn ret_type(&self, stack: &[Value]) -> Option<Type> {
        Some(match (self, stack) {
            (Function::Un(_), [.., f]) if f.ty().is_field() => f.ty(),
            (Function::Bin(_), [.., a, b]) if a.ty().is_field() && b.ty().is_field() => {
                a.ty().max(b.ty())
            }
            _ => return None,
        })
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Function::Un(op) => write!(f, "{op:?}"),
            Function::Bin(op) => write!(f, "{op:?}"),
        }
    }
}
