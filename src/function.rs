use crate::{BinOp, Type, UnOp, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
