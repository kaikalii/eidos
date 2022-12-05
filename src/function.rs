use std::{marker::PhantomData, ops::*};

use derive_more::Display;
use eframe::epaint::{vec2, Vec2};
use enum_iterator::Sequence;

use crate::{CommonField, EidosError, ScalarField, Type, Value};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Nullary(Nullary),
    Combinator1(Combinator1),
    Combinator2(Combinator2),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Nullary {
    X,
    Y,
    Zero,
    One,
}

impl Nullary {
    pub fn value(&self) -> Value {
        match self {
            Nullary::X => ScalarField::Common(CommonField::X).into(),
            Nullary::Y => ScalarField::Common(CommonField::Y).into(),
            Nullary::Zero => CommonField::Uniform(0.0).into(),
            Nullary::One => CommonField::Uniform(1.0).into(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator1 {
    Duplicate,
    Drop,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator2 {
    Apply,
    Swap,
    Over,
}

pub trait UnOperator<T> {
    type Output;
    fn operate(&self, v: T) -> Self::Output;
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum UnOp<T> {
    Math(MathUnOp),
    Typed(T),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum GenericUnOp {
    Scalar(ScalarUnOp),
    VectorScalar(VectorUnScalarOp),
    VectorVector(VectorUnVectorOp),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum MathUnOp {
    Neg,
    Abs,
    Sign,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum ScalarUnOp {
    Sin,
    Cos,
    Tan,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum VectorUnScalarOp {
    Length,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum VectorUnVectorOp {
    Unit,
}

impl<A, T> UnOperator<A> for UnOp<T>
where
    MathUnOp: UnOperator<A, Output = T::Output>,
    T: UnOperator<A>,
{
    type Output = T::Output;
    fn operate(&self, v: A) -> Self::Output {
        match self {
            UnOp::Math(op) => op.operate(v),
            UnOp::Typed(op) => op.operate(v),
        }
    }
}

impl UnOperator<f32> for MathUnOp {
    type Output = f32;
    fn operate(&self, v: f32) -> Self::Output {
        match self {
            MathUnOp::Neg => -v,
            MathUnOp::Abs => v.abs(),
            MathUnOp::Sign => v.signum(),
        }
    }
}

impl UnOperator<Vec2> for MathUnOp {
    type Output = Vec2;
    fn operate(&self, v: Vec2) -> Self::Output {
        vec2(self.operate(v.x), self.operate(v.y))
    }
}

impl UnOperator<f32> for ScalarUnOp {
    type Output = f32;
    fn operate(&self, v: f32) -> Self::Output {
        match self {
            ScalarUnOp::Sin => v.sin(),
            ScalarUnOp::Cos => v.cos(),
            ScalarUnOp::Tan => v.tan(),
        }
    }
}

impl UnOperator<Vec2> for VectorUnScalarOp {
    type Output = f32;
    fn operate(&self, v: Vec2) -> Self::Output {
        match self {
            VectorUnScalarOp::Length => v.length(),
        }
    }
}

impl UnOperator<Vec2> for VectorUnVectorOp {
    type Output = Vec2;
    fn operate(&self, v: Vec2) -> Self::Output {
        match self {
            VectorUnVectorOp::Unit => v.normalized(),
        }
    }
}

pub trait BinOperator<A, B> {
    type Output;
    fn operate(&self, a: A, b: B) -> Self::Output;
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum BinOp<T> {
    Math(MathBinOp),
    Typed(T),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum MathBinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum HomoBinOp {
    Min,
    Max,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub struct NoOp<T>(PhantomData<T>);

impl<A, B, T> BinOperator<A, B> for BinOp<T>
where
    MathBinOp: BinOperator<A, B, Output = T::Output>,
    T: BinOperator<A, B>,
{
    type Output = T::Output;
    fn operate(&self, a: A, b: B) -> Self::Output {
        match self {
            BinOp::Math(op) => op.operate(a, b),
            BinOp::Typed(op) => op.operate(a, b),
        }
    }
}

impl<A, B, T> BinOperator<A, B> for NoOp<T> {
    type Output = T;
    fn operate(&self, _: A, _: B) -> Self::Output {
        unreachable!()
    }
}

impl BinOperator<f32, f32> for MathBinOp {
    type Output = f32;
    fn operate(&self, a: f32, b: f32) -> Self::Output {
        self.homo_operate(a, b)
    }
}

impl BinOperator<Vec2, Vec2> for MathBinOp {
    type Output = Vec2;
    fn operate(&self, a: Vec2, b: Vec2) -> Self::Output {
        self.homo_operate(a, b)
    }
}

impl BinOperator<f32, Vec2> for MathBinOp {
    type Output = Vec2;
    fn operate(&self, a: f32, b: Vec2) -> Self::Output {
        self.operate(Vec2::splat(a), b)
    }
}

impl BinOperator<Vec2, f32> for MathBinOp {
    type Output = Vec2;
    fn operate(&self, a: Vec2, b: f32) -> Self::Output {
        self.operate(a, Vec2::splat(b))
    }
}

impl MathBinOp {
    fn homo_operate<T>(&self, a: T, b: T) -> T
    where
        T: Add<Output = T>,
        T: Sub<Output = T>,
        T: Mul<Output = T>,
        T: Div<Output = T>,
    {
        match self {
            MathBinOp::Add => a + b,
            MathBinOp::Sub => a - b,
            MathBinOp::Mul => a * b,
            MathBinOp::Div => a / b,
        }
    }
}

impl BinOperator<f32, f32> for HomoBinOp {
    type Output = f32;
    fn operate(&self, a: f32, b: f32) -> Self::Output {
        match self {
            HomoBinOp::Min => a.min(b),
            HomoBinOp::Max => a.max(b),
        }
    }
}

impl BinOperator<Vec2, Vec2> for HomoBinOp {
    type Output = Vec2;
    fn operate(&self, a: Vec2, b: Vec2) -> Vec2 {
        match self {
            HomoBinOp::Min => a.min(b),
            HomoBinOp::Max => a.max(b),
        }
    }
}

#[derive(Debug, Display, Clone, Copy)]
pub enum TypeConstraint {
    Exact(Type),
    Field,
    Any,
}

impl Function {
    pub fn validate_use(&self, stack: &[Value]) -> Result<(), EidosError> {
        // Collect constraints
        use TypeConstraint::*;
        let constraints = match self {
            Function::Nullary(_) => vec![],
            Function::Combinator1(_) => vec![Any],
            Function::Combinator2(_) => vec![Any; 2],
        };
        // Validate stack size
        if stack.len() < constraints.len() {
            return Err(EidosError::NotEnoughArguments {
                function: *self,
                expected: constraints.len(),
                stack_size: stack.len(),
            });
        }
        // Validate constraints
        for (i, (constraint, value)) in constraints
            .into_iter()
            .rev()
            .zip(stack.iter().rev())
            .rev()
            .enumerate()
        {
            match (constraint, value) {
                (Any, _) => {}
                (Field, Value::Field(_)) => {}
                (Exact(ty), value) if ty == value.ty() => {}
                _ => {
                    return Err(EidosError::InvalidArgument {
                        function: *self,
                        position: i + 1,
                        expected: constraint,
                        found: value.ty(),
                    })
                }
            }
        }
        Ok(())
    }
}
