use std::{collections::HashMap, marker::PhantomData, ops::*};

use derive_more::{Display, From};
use eframe::epaint::{vec2, Vec2};
use enum_iterator::Sequence;

use crate::{error::EidosError, field::*, person::PersonId, stack::Stack};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, From)]
pub enum Function {
    #[from(types(ScalarInputFieldKind, VectorInputFieldKind))]
    ReadField(InputFieldKind),
    #[from(types(ScalarOutputFieldKind, VectorOutputFieldKind))]
    WriteField(OutputFieldKind),
    #[from]
    Control(ControlKind),
    #[from]
    Nullary(Nullary),
    #[from(types(HeteroBinOp, HomoBinOp))]
    Bin(BinOp),
    #[from(types(MathUnOp, ScalarUnOp, ToScalarOp))]
    Un(UnOp),
    #[from]
    Combinator1(Combinator1),
    #[from]
    Combinator2(Combinator2),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Sequence)]
pub enum Nullary {
    Zero,
    One,
    Two,
    Five,
    Ten,
    ZeroVector,
    OneX,
    OneY,
    X,
    Y,
    TargetX,
    TargetY,
    Filter,
}

impl Nullary {
    pub fn field(&self, caster: PersonId) -> Field {
        match self {
            Nullary::Zero => ScalarField::Uniform(0.0).into(),
            Nullary::One => ScalarField::Uniform(1.0).into(),
            Nullary::Two => ScalarField::Uniform(2.0).into(),
            Nullary::Five => ScalarField::Uniform(5.0).into(),
            Nullary::Ten => ScalarField::Uniform(10.0).into(),
            Nullary::ZeroVector => VectorField::Uniform(Vec2::ZERO).into(),
            Nullary::OneX => VectorField::Uniform(Vec2::X).into(),
            Nullary::OneY => VectorField::Uniform(Vec2::Y).into(),
            Nullary::X => ScalarField::X(caster).into(),
            Nullary::Y => ScalarField::Y(caster).into(),
            Nullary::TargetX => ScalarField::TargetX(caster).into(),
            Nullary::TargetY => ScalarField::TargetY(caster).into(),
            Nullary::Filter => ScalarField::Filter(caster).into(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Sequence)]
pub enum Combinator1 {
    Duplicate,
    Drop,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Sequence)]
pub enum Combinator2 {
    Swap,
    Over,
}

pub trait UnOperator<T> {
    type Output;
    fn operate(&self, v: T) -> Self::Output;
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
pub enum UnOp {
    Math(MathUnOp),
    Scalar(ScalarUnOp),
    VectorScalar(VectorUnScalarOp),
    VectorVector(VectorUnVectorOp),
    ToScalar(ToScalarOp),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum TypedUnOp<T> {
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
    Reciprocal,
    Sqrt,
    ToScalar(ToScalarOp),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum VectorUnScalarOp {
    Length,
    ToScalar(ToScalarOp),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum VectorUnVectorOp {
    Unit,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum ToScalarOp {
    Magnitude,
}

impl<A, T> UnOperator<A> for TypedUnOp<T>
where
    MathUnOp: UnOperator<A, Output = T::Output>,
    T: UnOperator<A>,
{
    type Output = T::Output;
    fn operate(&self, v: A) -> Self::Output {
        match self {
            TypedUnOp::Math(op) => op.operate(v),
            TypedUnOp::Typed(op) => op.operate(v),
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
            ScalarUnOp::Reciprocal if v == 0.0 => 0.0,
            ScalarUnOp::Reciprocal => 1.0 / v,
            ScalarUnOp::Sqrt if v < 0.0 => 0.0,
            ScalarUnOp::Sqrt => v.sqrt(),
            ScalarUnOp::ToScalar(op) => match op {
                ToScalarOp::Magnitude => v.abs(),
            },
        }
    }
}

impl UnOperator<Vec2> for VectorUnScalarOp {
    type Output = f32;
    fn operate(&self, v: Vec2) -> Self::Output {
        match self {
            VectorUnScalarOp::Length => v.length(),
            VectorUnScalarOp::ToScalar(op) => match op {
                ToScalarOp::Magnitude => v.length(),
            },
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
pub enum BinOp {
    Math(HeteroBinOp),
    Homo(HomoBinOp),
    #[display(fmt = "🔀Index")]
    Index,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum TypedBinOp<T> {
    Hetero(HeteroBinOp),
    Typed(T),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum HeteroBinOp {
    #[display(fmt = "×")]
    Mul,
    #[display(fmt = "÷")]
    Div,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub enum HomoBinOp {
    #[display(fmt = "+")]
    Add,
    #[display(fmt = "-")]
    Sub,
    #[display(fmt = "⬇Min")]
    Min,
    #[display(fmt = "⬆Max")]
    Max,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence)]
pub struct NoOp<T>(PhantomData<T>);

impl<A, B, T> BinOperator<A, B> for TypedBinOp<T>
where
    HeteroBinOp: BinOperator<A, B, Output = T::Output>,
    T: BinOperator<A, B>,
{
    type Output = T::Output;
    fn operate(&self, a: A, b: B) -> Self::Output {
        match self {
            TypedBinOp::Hetero(op) => op.operate(a, b),
            TypedBinOp::Typed(op) => op.operate(a, b),
        }
    }
}

impl<A, B, T> BinOperator<A, B> for NoOp<T> {
    type Output = T;
    fn operate(&self, _: A, _: B) -> Self::Output {
        unreachable!()
    }
}

impl BinOperator<f32, f32> for HeteroBinOp {
    type Output = f32;
    fn operate(&self, a: f32, b: f32) -> Self::Output {
        self.homo_operate(a, b)
    }
}

impl BinOperator<Vec2, Vec2> for HeteroBinOp {
    type Output = Vec2;
    fn operate(&self, a: Vec2, b: Vec2) -> Self::Output {
        self.homo_operate(a, b)
    }
}

impl BinOperator<f32, Vec2> for HeteroBinOp {
    type Output = Vec2;
    fn operate(&self, a: f32, b: Vec2) -> Self::Output {
        self.operate(Vec2::splat(a), b)
    }
}

impl BinOperator<Vec2, f32> for HeteroBinOp {
    type Output = Vec2;
    fn operate(&self, a: Vec2, b: f32) -> Self::Output {
        self.operate(a, Vec2::splat(b))
    }
}

impl HeteroBinOp {
    fn homo_operate<T>(&self, a: T, b: T) -> T
    where
        T: Add<Output = T>,
        T: Sub<Output = T>,
        T: Mul<Output = T>,
        T: Div<Output = T>,
    {
        match self {
            HeteroBinOp::Mul => a * b,
            HeteroBinOp::Div => a / b,
        }
    }
}

impl BinOperator<f32, f32> for HomoBinOp {
    type Output = f32;
    fn operate(&self, a: f32, b: f32) -> Self::Output {
        match self {
            HomoBinOp::Add => a + b,
            HomoBinOp::Sub => a - b,
            HomoBinOp::Min => a.min(b),
            HomoBinOp::Max => a.max(b),
        }
    }
}

impl BinOperator<Vec2, Vec2> for HomoBinOp {
    type Output = Vec2;
    fn operate(&self, a: Vec2, b: Vec2) -> Vec2 {
        match self {
            HomoBinOp::Add => a + b,
            HomoBinOp::Sub => a - b,
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
            Function::ReadField(_) | Function::Control(_) | Function::Nullary(_) => vec![],
            Function::WriteField(kind) => match kind {
                OutputFieldKind::Scalar(_) => {
                    vec![Constrain(ValueConstraint::Exact(Type::Scalar))]
                }
                OutputFieldKind::Vector(_) => {
                    vec![Constrain(ValueConstraint::Exact(Type::Vector))]
                }
            },
            Function::Combinator1(_) => vec![Any],
            Function::Combinator2(_) => vec![Any; 2],
            Function::Un(op) => match op {
                UnOp::Math(_) => vec![Any],
                UnOp::Scalar(_) => vec![Constrain(ValueConstraint::Exact(Type::Scalar))],
                UnOp::VectorScalar(_) | UnOp::VectorVector(_) => {
                    vec![Constrain(ValueConstraint::Exact(Type::Vector))]
                }
                UnOp::ToScalar(_) => vec![Any],
            },
            Function::Bin(op) => match op {
                BinOp::Math(_) => {
                    vec![Any, Any]
                }
                BinOp::Homo(_) => vec![
                    Constrain(ValueConstraint::Group(0)),
                    Constrain(ValueConstraint::Group(0)),
                ],
                BinOp::Index => vec![Constrain(ValueConstraint::Exact(Type::Vector)), Any],
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
