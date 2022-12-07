use derive_more::{Display, From};
use enum_iterator::Sequence;

use crate::{field::*, function::*};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
pub enum SpellCommand {
    Clear,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
pub enum Word {
    Scalar(ScalarWord),
    Vector(VectorWord),
    Axis(AxisWord),
    Operator(OperatorWord),
    Combinator(CombinatorWord),
    Input(InputWord),
    Output(OutputWord),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum ScalarWord {
    Sero,
    Ti,
    Tu,
    Ta,
    Te,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum VectorWord {
    Kovo,
    Kova,
    Kovi,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum AxisWord {
    Seva,
    Sevi,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum InputWord {
    Le,
    Po,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum OutputWord {
    Ke,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum OperatorWord {
    Ma,
    Sa,
    Na,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum CombinatorWord {
    Ne,
    Mo,
    Re,
    Rovo,
}

impl Word {
    pub fn function(&self) -> Function {
        use Word::*;
        match self {
            Scalar(ScalarWord::Sero) => Nullary::Zero.into(),
            Scalar(ScalarWord::Ti) => Nullary::One.into(),
            Scalar(ScalarWord::Tu) => Nullary::Two.into(),
            Scalar(ScalarWord::Ta) => Nullary::Five.into(),
            Scalar(ScalarWord::Te) => Nullary::Ten.into(),
            Vector(VectorWord::Kovo) => Nullary::ZeroVector.into(),
            Vector(VectorWord::Kova) => Nullary::OneX.into(),
            Vector(VectorWord::Kovi) => Nullary::OneY.into(),
            Axis(AxisWord::Seva) => Nullary::X.into(),
            Axis(AxisWord::Sevi) => Nullary::Y.into(),
            Input(InputWord::Le) => ScalarInputFieldKind::Elevation.into(),
            Input(InputWord::Po) => ScalarInputFieldKind::Density.into(),
            Output(OutputWord::Ke) => VectorOutputFieldKind::Force.into(),
            Operator(OperatorWord::Ma) => MathBinOp::Add.into(),
            Operator(OperatorWord::Sa) => MathBinOp::Mul.into(),
            Operator(OperatorWord::Na) => MathBinOp::Sub.into(),
            Combinator(CombinatorWord::Ne) => Combinator1::Drop.into(),
            Combinator(CombinatorWord::Mo) => Combinator1::Duplicate.into(),
            Combinator(CombinatorWord::Re) => Combinator2::Swap.into(),
            Combinator(CombinatorWord::Rovo) => Combinator2::Over.into(),
        }
    }
}
