use derive_more::{Display, From};
use enum_iterator::Sequence;
use serde::Deserialize;

use crate::{field::*, function::*};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, From, Sequence)]
pub enum SpellCommand {
    Clear,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Sequence, Deserialize)]
#[serde(untagged)]
pub enum Word {
    Scalar(ScalarWord),
    Vector(VectorWord),
    Axis(AxisWord),
    Operator(OperatorWord),
    Combinator(CombinatorWord),
    Control(ControlWord),
    Input(InputWord),
    Output(OutputWord),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum ScalarWord {
    Sero,
    Ti,
    Tu,
    Ta,
    Te,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum VectorWord {
    Kovo,
    Kova,
    Kovi,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum AxisWord {
    Seva,
    Sevi,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum InputWord {
    Le,
    Po,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum OutputWord {
    Ke,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum OperatorWord {
    Ma,
    Sa,
    Na,
    Neka,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum CombinatorWord {
    No,
    Mo,
    Re,
    Rovo,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum ControlWord {
    Sila,
    Vila,
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
            Operator(OperatorWord::Ma) => HomoBinOp::Add.into(),
            Operator(OperatorWord::Sa) => HeteroBinOp::Mul.into(),
            Operator(OperatorWord::Na) => HomoBinOp::Sub.into(),
            Operator(OperatorWord::Neka) => MathUnOp::Neg.into(),
            Combinator(CombinatorWord::No) => Combinator1::Drop.into(),
            Combinator(CombinatorWord::Mo) => Combinator1::Duplicate.into(),
            Combinator(CombinatorWord::Re) => Combinator2::Swap.into(),
            Combinator(CombinatorWord::Rovo) => Combinator2::Over.into(),
            Control(ControlWord::Sila) => ControlKind::XSlider.into(),
            Control(ControlWord::Vila) => ControlKind::YSlider.into(),
        }
    }
}
