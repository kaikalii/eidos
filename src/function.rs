use std::fmt;

use enum_iterator::Sequence;

use crate::{BinOp, EidosError, Type, UnOp, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Function {
    Un(UnOp),
    Bin(BinOp),
}

impl Function {
    pub fn ret_type(&self, stack: &[Value]) -> Result<Type, EidosError> {
        match (self, stack) {
            (Function::Un(_), [.., f]) => {
                if f.ty().is_field() {
                    Ok(f.ty())
                } else {
                    Err(EidosError::invalid_argument(self, 1, f.ty()))
                }
            }
            (Function::Un(_), _) => Err(EidosError::not_enough_arguments(self, 1, stack.len())),
            (Function::Bin(_), [.., a, b]) => {
                if !a.ty().is_field() {
                    return Err(EidosError::invalid_argument(self, 1, a.ty()));
                }
                if !b.ty().is_field() {
                    return Err(EidosError::invalid_argument(self, 2, b.ty()));
                }
                Ok(a.ty().max(b.ty()))
            }
            (Function::Bin(_), _) => Err(EidosError::not_enough_arguments(self, 2, stack.len())),
        }
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
