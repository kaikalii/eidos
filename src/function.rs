use std::{collections::HashMap, marker::PhantomData, ops::*};

use derive_more::Display;
use eframe::epaint::{vec2, Vec2};
use enum_iterator::{all, Sequence};

use crate::{error::EidosError, field::*, value::*};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Function {
    ReadField(GenericFieldKind),
    Nullary(Nullary),
    Combinator1(Combinator1),
    Combinator2(Combinator2),
    Un(GenericUnOp),
    Bin(GenericBinOp),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum FunctionCategory {
    ReadField,
    Nullary,
    Combinator,
    Unary,
    Binary,
}

impl FunctionCategory {
    pub fn functions(&self) -> Box<dyn Iterator<Item = Function>> {
        match self {
            FunctionCategory::ReadField => {
                Box::new(all::<GenericFieldKind>().map(Function::ReadField))
            }
            FunctionCategory::Nullary => Box::new(all::<Nullary>().map(Function::Nullary)),
            FunctionCategory::Combinator => Box::new(
                all::<Combinator1>()
                    .map(Function::Combinator1)
                    .chain(all::<Combinator2>().map(Function::Combinator2)),
            ),
            FunctionCategory::Unary => Box::new(all::<GenericUnOp>().map(Function::Un)),
            FunctionCategory::Binary => Box::new(all::<GenericBinOp>().map(Function::Bin)),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Nullary {
    Zero,
    One,
    OneX,
    OneY,
    X,
    Y,
    VX,
    VY,
}

impl Nullary {
    pub fn value<'a>(&self) -> Value<'a> {
        match self {
            Nullary::Zero => CommonField::Uniform(0.0).into(),
            Nullary::One => CommonField::Uniform(1.0).into(),
            Nullary::OneX => CommonField::Uniform(Vec2::X).into(),
            Nullary::OneY => CommonField::Uniform(Vec2::X).into(),
            Nullary::X => ScalarField::Common(CommonField::X).into(),
            Nullary::Y => ScalarField::Common(CommonField::Y).into(),
            Nullary::VX => VectorField::Common(CommonField::X).into(),
            Nullary::VY => VectorField::Common(CommonField::Y).into(),
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
pub enum GenericUnOp {
    Math(MathUnOp),
    Scalar(ScalarUnOp),
    VectorScalar(VectorUnScalarOp),
    VectorVector(VectorUnVectorOp),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum UnOp<T> {
    Math(MathUnOp),
    Typed(T),
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
pub enum GenericBinOp {
    Math(MathBinOp),
    Homo(HomoBinOp),
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
    Field(ValueConstraint),
    Any,
}

#[derive(Debug, Display, Clone, Copy)]
pub enum ValueConstraint {
    Exact(ValueType),
    Group(u8),
    Any,
}

#[derive(Default)]
struct ConstraintContext {
    values: HashMap<u8, ValueType>,
}

impl TypeConstraint {
    fn matches(&self, ty: Type, ctx: &mut ConstraintContext) -> bool {
        match (self, ty) {
            (TypeConstraint::Field(constraint), Type::Field(vt)) => constraint.matches(vt, ctx),
            (TypeConstraint::Any, _) => true,
            _ => false,
        }
    }
}

impl ValueConstraint {
    fn matches(&self, ty: ValueType, ctx: &mut ConstraintContext) -> bool {
        match self {
            ValueConstraint::Exact(vt) => vt == &ty,
            ValueConstraint::Group(i) => {
                if let Some(ty2) = ctx.values.get(i) {
                    &ty == ty2
                } else {
                    ctx.values.insert(*i, ty);
                    true
                }
            }
            ValueConstraint::Any => true,
        }
    }
}

impl Function {
    pub fn validate_use(&self, stack: &[Value]) -> Result<(), EidosError> {
        // Collect constraints
        use TypeConstraint::*;
        let constraints = match self {
            Function::ReadField(_) | Function::Nullary(_) => vec![],
            Function::Combinator1(_) => vec![Any],
            Function::Combinator2(_) => vec![Any; 2],
            Function::Un(op) => match op {
                GenericUnOp::Math(_) => vec![Any],
                GenericUnOp::Scalar(_) => vec![Field(ValueConstraint::Exact(ValueType::Scalar))],
                GenericUnOp::VectorScalar(_) | GenericUnOp::VectorVector(_) => {
                    vec![Field(ValueConstraint::Exact(ValueType::Vector))]
                }
            },
            Function::Bin(op) => match op {
                GenericBinOp::Math(_) => {
                    vec![Field(ValueConstraint::Any), Field(ValueConstraint::Any)]
                }
                GenericBinOp::Homo(_) => vec![
                    Field(ValueConstraint::Group(0)),
                    Field(ValueConstraint::Group(0)),
                ],
            },
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
        let mut ctx = ConstraintContext::default();
        for (i, (constraint, value)) in constraints
            .into_iter()
            .rev()
            .zip(stack.iter().rev())
            .rev()
            .enumerate()
        {
            if !constraint.matches(value.ty(), &mut ctx) {
                return Err(EidosError::InvalidArgument {
                    function: *self,
                    position: i + 1,
                    expected: constraint,
                    found: value.ty(),
                });
            }
        }
        Ok(())
    }
}
