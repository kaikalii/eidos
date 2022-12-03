use std::fmt;

use enum_iterator::{all, Sequence};

use crate::{BinOp, EidosError, Resampler, Type, UnOp, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Identity,
    Combinator(Combinator),
    Zip(BinOp),
    Square(BinOp),
    Un(UnOp),
    Resample(Resampler),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator {
    Duplicate,
    Combinator2(Combinator2),
}

impl fmt::Debug for Combinator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Combinator::Duplicate => write!(f, "Duplicate"),
            Combinator::Combinator2(com) => write!(f, "{com:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator2 {
    Swap,
    Over,
}

#[derive(Debug, Sequence)]
pub enum FunctionCategory {
    Combinator,
    Zip,
    Square,
    Unary,
    Resample,
}

impl FunctionCategory {
    pub fn functions(&self) -> Box<dyn Iterator<Item = Function>> {
        match self {
            FunctionCategory::Combinator => Box::new(all::<Combinator>().map(Function::Combinator)),
            FunctionCategory::Zip => Box::new(all::<BinOp>().map(Function::Zip)),
            FunctionCategory::Square => Box::new(all::<BinOp>().map(Function::Square)),
            FunctionCategory::Unary => Box::new(all::<UnOp>().map(Function::Un)),
            FunctionCategory::Resample => Box::new(all::<Resampler>().map(Function::Resample)),
        }
    }
}

impl Function {
    pub fn validate_use(&self, stack: &[Value]) -> Result<(), EidosError> {
        match (self, stack) {
            (Function::Identity, _) => Ok(()),
            (Function::Combinator(com), stack) => {
                let args = match com {
                    Combinator::Duplicate => 1,
                    Combinator::Combinator2(_) => 2,
                };
                if stack.len() >= args {
                    Ok(())
                } else {
                    Err(EidosError::not_enough_arguments(self, args, stack.len()))
                }
            }
            (Function::Un(_), [.., f]) => {
                if f.ty().is_field() {
                    Ok(())
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
                Ok(())
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
                Ok(())
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
            Function::Combinator(com) => write!(f, "{com:?}"),
            Function::Un(op) => write!(f, "{op:?}"),
            Function::Zip(op) => write!(f, "{op:?}"),
            Function::Square(op) => write!(f, "square {op:?}"),
            Function::Resample(res) => write!(f, "{res:?}"),
        }
    }
}
