use crate::{
    error::EidosError,
    field::*,
    function::*,
    person::{ActiveSpell, Person, PersonId},
    word::Word,
};

#[derive(Default)]
pub struct Stack {
    stack: Vec<StackItem>,
}

pub struct StackItem {
    pub field: Field,
    pub words: Vec<Word>,
}

trait IntoWords {
    fn into_words(self) -> Vec<Word>;
}

impl IntoWords for Word {
    fn into_words(self) -> Vec<Word> {
        vec![self]
    }
}

impl IntoWords for Vec<Word> {
    fn into_words(self) -> Vec<Word> {
        self
    }
}

impl<A, B> IntoWords for (A, B)
where
    A: IntoWords,
    B: IntoWords,
{
    fn into_words(self) -> Vec<Word> {
        let mut words = self.0.into_words();
        words.append(&mut self.1.into_words());
        words
    }
}

impl<A, B, C> IntoWords for (A, B, C)
where
    A: IntoWords,
    B: IntoWords,
    C: IntoWords,
{
    fn into_words(self) -> Vec<Word> {
        let mut words = self.0.into_words();
        words.append(&mut self.1.into_words());
        words.append(&mut self.2.into_words());
        words
    }
}

impl Stack {
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
    pub fn len(&self) -> usize {
        self.stack.len()
    }
    pub fn iter(&self) -> std::slice::Iter<StackItem> {
        self.stack.iter()
    }
    pub fn validate_function_use(&self, function: Function) -> Result<(), EidosError> {
        function.validate_use(self)
    }
    #[track_caller]
    fn pop(&mut self) -> StackItem {
        self.stack.pop().expect("Nothing to pop")
    }
    fn push(&mut self, words: impl IntoWords, field: impl Into<Field>) {
        self.stack.push(StackItem {
            field: field.into(),
            words: words.into_words(),
        })
    }
    pub fn clear(&mut self) {
        self.stack.clear();
    }
    pub fn words(&self) -> impl Iterator<Item = Word> + '_ {
        self.stack.iter().flat_map(|item| &item.words).copied()
    }
}

impl Person {
    pub fn say(&mut self, person_id: PersonId, word: Word) -> Result<(), EidosError> {
        puffin::profile_function!();
        let stack = &mut self.stack;
        let function = word.function();
        stack.validate_function_use(function)?;
        match function {
            Function::ReadField(field_kind) => match field_kind {
                InputFieldKind::Scalar(kind) => stack.push(word, ScalarField::Input(kind)),
                InputFieldKind::Vector(kind) => stack.push(word, VectorField::Input(kind)),
            },
            Function::WriteField(field_kind) => {
                let item = stack.pop();
                match (field_kind, item.field) {
                    (OutputFieldKind::Vector(kind), Field::Vector(field)) => {
                        self.active_spells
                            .vectors
                            .entry(kind)
                            .or_default()
                            .push(ActiveSpell {
                                field,
                                words: item.words.into_iter().chain([word]).collect(),
                            });
                    }
                    _ => unreachable!(),
                }
                stack.clear();
            }
            Function::Control(kind) => stack.push(word, ScalarField::Control(kind)),
            Function::Nullary(nullary) => stack.push(word, nullary.field(person_id)),
            Function::Combinator1(com1) => {
                let a = stack.pop();
                match com1 {
                    Combinator1::Duplicate => {
                        stack.push(a.words, a.field.clone());
                        stack.push(word, a.field);
                    }
                    Combinator1::Drop => {}
                }
            }
            Function::Combinator2(com2) => {
                let b = stack.pop();
                let a = stack.pop();
                match com2 {
                    Combinator2::Swap => {
                        stack.stack.push(b);
                        stack.stack.push(a);
                    }
                    Combinator2::Over => {
                        stack.push(a.words, a.field.clone());
                        stack.stack.push(b);
                        stack.push(word, a.field);
                    }
                }
            }
            Function::Un(op) => {
                let a = stack.pop();
                let words = (a.words, word);
                match op {
                    UnOp::Math(op) => match a.field {
                        Field::Scalar(f) => stack.push(
                            words,
                            ScalarField::ScalarUn(TypedUnOp::Math(op), f.into()).reduce(),
                        ),
                        Field::Vector(f) => stack.push(
                            words,
                            VectorField::Un(TypedUnOp::Math(op), f.into()).reduce(),
                        ),
                    },
                    UnOp::Scalar(op) => match a.field {
                        Field::Scalar(f) => stack.push(
                            words,
                            ScalarField::ScalarUn(TypedUnOp::Typed(op), f.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    UnOp::VectorScalar(op) => match a.field {
                        Field::Vector(f) => {
                            stack.push(words, ScalarField::VectorUn(op, f.into()).reduce())
                        }
                        _ => unreachable!(),
                    },
                    UnOp::VectorVector(op) => match a.field {
                        Field::Vector(f) => stack.push(
                            words,
                            VectorField::Un(TypedUnOp::Typed(op), f.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    UnOp::ToScalar(op) => match a.field {
                        Field::Scalar(f) => stack.push(
                            words,
                            ScalarField::ScalarUn(
                                TypedUnOp::Typed(ScalarUnOp::ToScalar(op)),
                                f.into(),
                            )
                            .reduce(),
                        ),
                        Field::Vector(f) => stack.push(
                            words,
                            ScalarField::VectorUn(VectorUnScalarOp::ToScalar(op), f.into())
                                .reduce(),
                        ),
                    },
                }
            }
            Function::Bin(op) => {
                let b = stack.pop();
                let a = stack.pop();
                let words = (a.words, b.words, word);
                match op {
                    BinOp::Math(op) => match (a.field, b.field) {
                        (Field::Scalar(a), Field::Scalar(b)) => {
                            stack.push(
                                words,
                                ScalarField::Bin(TypedBinOp::Hetero(op), a.into(), b.into())
                                    .reduce(),
                            );
                        }
                        (Field::Scalar(a), Field::Vector(b)) => {
                            stack.push(
                                words,
                                VectorField::BinSV(TypedBinOp::Hetero(op), a, b.into()).reduce(),
                            );
                        }
                        (Field::Vector(a), Field::Scalar(b)) => {
                            stack.push(
                                words,
                                VectorField::BinVS(TypedBinOp::Hetero(op), a.into(), b).reduce(),
                            );
                        }
                        (Field::Vector(a), Field::Vector(b)) => {
                            stack.push(
                                words,
                                VectorField::BinVV(TypedBinOp::Hetero(op), a.into(), b.into())
                                    .reduce(),
                            );
                        }
                    },
                    BinOp::Homo(op) => match (a.field, b.field) {
                        (Field::Scalar(a), Field::Scalar(b)) => stack.push(
                            words,
                            ScalarField::Bin(TypedBinOp::Typed(op), a.into(), b.into()).reduce(),
                        ),
                        (Field::Vector(a), Field::Vector(b)) => stack.push(
                            words,
                            VectorField::BinVV(TypedBinOp::Typed(op), a.into(), b.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    BinOp::Index => match (a.field, b.field) {
                        (Field::Vector(a), Field::Scalar(b)) => {
                            stack.push(words, ScalarField::Index(a.into(), b.into()))
                        }
                        (Field::Vector(a), Field::Vector(b)) => {
                            stack.push(words, VectorField::Index(a.into(), b.into()))
                        }
                        _ => unreachable!(),
                    },
                }
            }
            Function::Variable(var) => match var {
                Variable::Scalar => stack.push(word, ScalarField::Variable),
                Variable::Vector => stack.push(word, VectorField::Variable),
            },
            Function::Record => todo!(),
        }
        Ok(())
    }
}
