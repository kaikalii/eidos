use std::fmt;

use enum_iterator::{all, Sequence};

use crate::{BinOp, EidosError, Resampler, Type, UnOp, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Identity,
    Zip(BinOp),
    Square(BinOp),
    Un(UnOp),
    Resample(Resampler),
}

#[derive(Debug, Sequence)]
pub enum FunctionCategory {
    Zip,
    Square,
    Unary,
    Resample,
}

impl FunctionCategory {
    pub fn functions(&self) -> Box<dyn Iterator<Item = Function>> {
        match self {
            FunctionCategory::Zip => Box::new(all::<BinOp>().map(Function::Zip)),
            FunctionCategory::Square => Box::new(all::<BinOp>().map(Function::Square)),
            FunctionCategory::Unary => Box::new(all::<UnOp>().map(Function::Un)),
            FunctionCategory::Resample => Box::new(all::<Resampler>().map(Function::Resample)),
        }
    }
}

impl Function {
    pub fn ret_type(&self, stack: &[Value]) -> Result<Type, EidosError> {
        match (self, stack) {
            (Function::Identity, _) => Ok(Type::Field(1)),
            (Function::Un(_), [.., f]) => {
                if f.ty().is_field() {
                    Ok(f.ty())
                } else {
                    Err(EidosError::invalid_argument(self, 1, f.ty()))
                }
            }
            (Function::Un(_), _) => Err(EidosError::not_enough_arguments(self, 1, stack.len())),
            (Function::Zip(_) | Function::Square(_), [.., a, b]) => {
                if !a.ty().is_field() {
                    return Err(EidosError::invalid_argument(self, 1, a.ty()));
                }
                if !b.ty().is_field() {
                    return Err(EidosError::invalid_argument(self, 2, b.ty()));
                }
                Ok(a.ty().max(b.ty()))
            }
            (Function::Zip(_) | Function::Square(_), _) => {
                Err(EidosError::not_enough_arguments(self, 2, stack.len()))
            }
            (Function::Resample(_), [.., a, b]) => {
                if !a.ty().is_field() {
                    return Err(EidosError::invalid_argument(self, 1, a.ty()));
                }
                if b.ty() != Type::Field(0) {
                    return Err(EidosError::invalid_argument(self, 2, b.ty()));
                }
                Ok(a.ty())
            }
            (Function::Resample(_), _) => {
                Err(EidosError::not_enough_arguments(self, 2, stack.len()))
            }
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Function::Identity => write!(f, "Identity"),
            Function::Un(op) => write!(f, "{op:?}"),
            Function::Zip(op) => write!(f, "{op:?}"),
            Function::Square(op) => write!(f, "square {op:?}"),
            Function::Resample(res) => write!(f, "{res:?}"),
        }
    }
}
