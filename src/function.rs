use std::{collections::HashMap, marker::PhantomData, ops::*};

use derive_more::{Display, From};
use eframe::epaint::{vec2, Vec2};
use enum_iterator::{all, Sequence};

use crate::{error::EidosError, field::*, runtime::Stack};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
pub enum Function {
    #[from(types(ScalarInputFieldKind, VectorInputFieldKind))]
    ReadField(GenericInputFieldKind),
    #[from(types(ScalarOutputFieldKind, VectorOutputFieldKind))]
    WriteField(GenericOutputFieldKind),
    #[from]
    Nullary(Nullary),
    #[from(types(MathBinOp, HomoBinOp))]
    Bin(GenericBinOp),
    #[from(types(MathUnOp))]
    Un(GenericUnOp),
    #[from]
    Combinator1(Combinator1),
    #[from]
    Combinator2(Combinator2),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum FunctionCategory {
    ReadField,
    WriteField,
    Nullary,
    Binary,
    Unary,
    Combinator,
}

impl FunctionCategory {
    pub fn functions(&self) -> Box<dyn Iterator<Item = Function>> {
        match self {
            FunctionCategory::ReadField => {
                Box::new(all::<GenericInputFieldKind>().map(Function::ReadField))
            }
            FunctionCategory::WriteField => {
                Box::new(all::<GenericOutputFieldKind>().map(Function::WriteField))
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
    #[display(fmt = "0")]
    Zero,
    #[display(fmt = "1")]
    One,
    #[display(fmt = "2")]
    Two,
    #[display(fmt = "5")]
    Five,
    #[display(fmt = "10")]
    Ten,
    #[display(fmt = "↕0↔")]
    ZeroVector,
    #[display(fmt = "1➡")]
    OneX,
    #[display(fmt = "1⬆")]
    OneY,
    X,
    Y,
}

impl Nullary {
    pub fn field(&self) -> GenericField {
        match self {
            Nullary::Zero => CommonField::Uniform(0.0).into(),
            Nullary::One => CommonField::Uniform(1.0).into(),
            Nullary::Two => CommonField::Uniform(2.0).into(),
            Nullary::Five => CommonField::Uniform(5.0).into(),
            Nullary::Ten => CommonField::Uniform(10.0).into(),
            Nullary::ZeroVector => CommonField::Uniform(Vec2::ZERO).into(),
            Nullary::OneX => CommonField::Uniform(Vec2::X).into(),
            Nullary::OneY => CommonField::Uniform(Vec2::Y).into(),
            Nullary::X => ScalarField::Common(CommonField::X).into(),
            Nullary::Y => ScalarField::Common(CommonField::Y).into(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator1 {
    #[display(fmt = "⏺⏺")]
    Duplicate,
    #[display(fmt = "⤵")]
    Drop,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum Combinator2 {
    #[display(fmt = "🔄")]
    Swap,
    #[display(fmt = "↗↘")]
    Over,
}

pub trait UnOperator<T> {
    type Output;
    fn operate(&self, v: T) -> Self::Output;
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
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
    #[display(fmt = "0-")]
    Neg,
    #[display(fmt = "|x|")]
    Abs,
    #[display(fmt = "+-?")]
    Sign,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum ScalarUnOp {
    #[display(fmt = "~Sin")]
    Sin,
    #[display(fmt = "~Cos")]
    Cos,
    #[display(fmt = "~Tan")]
    Tan,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum VectorUnScalarOp {
    #[display(fmt = "📏Length")]
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

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
pub enum GenericBinOp {
    Math(MathBinOp),
    Homo(HomoBinOp),
    #[display(fmt = "🔀Index")]
    Index,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum BinOp<T> {
    Math(MathBinOp),
    Typed(T),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum MathBinOp {
    #[display(fmt = "+")]
    Add,
    #[display(fmt = "-")]
    Sub,
    #[display(fmt = "×")]
    Mul,
    #[display(fmt = "÷")]
    Div,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum HomoBinOp {
    #[display(fmt = "⬇Min")]
    Min,
    #[display(fmt = "⬆Max")]
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
    Constrain(ValueConstraint),
    Any,
}

#[derive(Debug, Display, Clone, Copy)]
pub enum ValueConstraint {
    Exact(Type),
    Group(u8),
}

#[derive(Default)]
struct ConstraintContext {
    values: HashMap<u8, Type>,
}

impl TypeConstraint {
    fn matches(&self, ty: Type, ctx: &mut ConstraintContext) -> bool {
        match (self, ty) {
            (TypeConstraint::Constrain(constraint), vt) => constraint.matches(vt, ctx),
            (TypeConstraint::Any, _) => true,
        }
    }
}

impl ValueConstraint {
    fn matches(&self, ty: Type, ctx: &mut ConstraintContext) -> bool {
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
        }
    }
}

impl Function {
    pub fn validate_use(&self, stack: &Stack) -> Result<(), EidosError> {
        // Collect constraints
        use TypeConstraint::*;
        let constraints = match self {
            Function::ReadField(_) | Function::Nullary(_) => vec![],
            Function::WriteField(kind) => match kind {
                GenericOutputFieldKind::Scalar(_) => {
                    vec![Constrain(ValueConstraint::Exact(Type::Scalar))]
                }
                GenericOutputFieldKind::Vector(_) => {
                    vec![Constrain(ValueConstraint::Exact(Type::Vector))]
                }
            },
            Function::Combinator1(_) => vec![Any],
            Function::Combinator2(_) => vec![Any; 2],
            Function::Un(op) => match op {
                GenericUnOp::Math(_) => vec![Any],
                GenericUnOp::Scalar(_) => vec![Constrain(ValueConstraint::Exact(Type::Scalar))],
                GenericUnOp::VectorScalar(_) | GenericUnOp::VectorVector(_) => {
                    vec![Constrain(ValueConstraint::Exact(Type::Vector))]
                }
            },
            Function::Bin(op) => match op {
                GenericBinOp::Math(_) => {
                    vec![Any, Any]
                }
                GenericBinOp::Homo(_) => vec![
                    Constrain(ValueConstraint::Group(0)),
                    Constrain(ValueConstraint::Group(0)),
                ],
                GenericBinOp::Index => vec![Constrain(ValueConstraint::Exact(Type::Vector)), Any],
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
        for (i, (constraint, item)) in constraints
            .into_iter()
            .rev()
            .zip(stack.iter().rev())
            .rev()
            .enumerate()
        {
            if !constraint.matches(item.field.ty(), &mut ctx) {
                return Err(EidosError::InvalidArgument {
                    function: *self,
                    position: i + 1,
                    expected: constraint,
                    found: item.field.ty(),
                });
            }
        }
        Ok(())
    }
}
