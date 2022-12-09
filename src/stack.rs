use itertools::Itertools;

use crate::{
    error::EidosError,
    field::*,
    function::*,
    person::PersonId,
    word::Word,
    world::{OutputField, World},
};

pub struct Stack {
    person_id: PersonId,
    stack: Vec<StackItem>,
}

impl Stack {
    pub fn new(person_id: PersonId) -> Self {
        Stack {
            person_id,
            stack: Vec::new(),
        }
    }
}

pub struct StackItem {
    pub field: GenericField,
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
    #[allow(clippy::len_without_is_empty)]
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
    fn push(&mut self, words: impl IntoWords, field: impl Into<GenericField>) {
        self.stack.push(StackItem {
            field: field.into(),
            words: words.into_words(),
        })
    }
    pub fn call(&mut self, world: &mut World, word: Word) -> Result<(), EidosError> {
        puffin::profile_function!();
        let function = word.function();
        self.validate_function_use(function)?;
        match function {
            Function::ReadField(field_kind) => match field_kind {
                GenericInputFieldKind::Scalar(kind) => {
                    self.push(word, ScalarField::World(kind.into()))
                }
                GenericInputFieldKind::Vector(kind) => {
                    self.push(word, VectorField::World(kind.into()))
                }
            },
            Function::WriteField(field_kind) => {
                let words = self
                    .stack
                    .iter_mut()
                    .flat_map(|item| item.words.drain(..))
                    .chain([word])
                    .collect_vec();
                let item = self.pop();
                match (field_kind, item.field) {
                    (GenericOutputFieldKind::Vector(kind), GenericField::Vector(field)) => {
                        world
                            .outputs
                            .vectors
                            .entry(self.person_id)
                            .or_default()
                            .insert(kind, OutputField { field, words });
                    }
                    _ => unreachable!(),
                }
                world.person_mut(self.person_id).words.clear();
            }
            Function::Control(kind) => self.push(word, ScalarField::Control(kind)),
            Function::Nullary(nullary) => self.push(word, nullary.field()),
            Function::Combinator1(com1) => {
                let a = self.pop();
                match com1 {
                    Combinator1::Duplicate => {
                        self.push(a.words, a.field.clone());
                        self.push(word, a.field);
                    }
                    Combinator1::Drop => {}
                }
            }
            Function::Combinator2(com2) => {
                let b = self.pop();
                let a = self.pop();
                match com2 {
                    Combinator2::Swap => {
                        self.stack.push(b);
                        self.stack.push(a);
                    }
                    Combinator2::Over => {
                        self.push(a.words, a.field.clone());
                        self.stack.push(b);
                        self.push(word, a.field);
                    }
                }
            }
            Function::Un(op) => {
                let a = self.pop();
                let words = (a.words, word);
                match op {
                    GenericUnOp::Math(op) => match a.field {
                        GenericField::Scalar(f) => self.push(
                            words,
                            ScalarField::ScalarUn(UnOp::Math(op), f.into()).reduce(),
                        ),
                        GenericField::Vector(f) => {
                            self.push(words, VectorField::Un(UnOp::Math(op), f.into()).reduce())
                        }
                    },
                    GenericUnOp::Scalar(op) => match a.field {
                        GenericField::Scalar(f) => self.push(
                            words,
                            ScalarField::ScalarUn(UnOp::Typed(op), f.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    GenericUnOp::VectorScalar(op) => match a.field {
                        GenericField::Vector(f) => {
                            self.push(words, ScalarField::VectorUn(op, f.into()).reduce())
                        }
                        _ => unreachable!(),
                    },
                    GenericUnOp::VectorVector(op) => match a.field {
                        GenericField::Vector(f) => {
                            self.push(words, VectorField::Un(UnOp::Typed(op), f.into()).reduce())
                        }
                        _ => unreachable!(),
                    },
                }
            }
            Function::Bin(op) => {
                let b = self.pop();
                let a = self.pop();
                let words = (a.words, b.words, word);
                match op {
                    GenericBinOp::Math(op) => match (a.field, b.field) {
                        (GenericField::Scalar(a), GenericField::Scalar(b)) => {
                            self.push(
                                words,
                                ScalarField::Bin(BinOp::Math(op), a.into(), b.into()).reduce(),
                            );
                        }
                        (GenericField::Scalar(a), GenericField::Vector(b)) => {
                            self.push(
                                words,
                                VectorField::BinSV(BinOp::Math(op), a, b.into()).reduce(),
                            );
                        }
                        (GenericField::Vector(a), GenericField::Scalar(b)) => {
                            self.push(
                                words,
                                VectorField::BinVS(BinOp::Math(op), a.into(), b).reduce(),
                            );
                        }
                        (GenericField::Vector(a), GenericField::Vector(b)) => {
                            self.push(
                                words,
                                VectorField::BinVV(BinOp::Math(op), a.into(), b.into()).reduce(),
                            );
                        }
                    },
                    GenericBinOp::Homo(op) => match (a.field, b.field) {
                        (GenericField::Scalar(a), GenericField::Scalar(b)) => self.push(
                            words,
                            ScalarField::Bin(BinOp::Typed(op), a.into(), b.into()).reduce(),
                        ),
                        (GenericField::Vector(a), GenericField::Vector(b)) => self.push(
                            words,
                            VectorField::BinVV(BinOp::Typed(op), a.into(), b.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    GenericBinOp::Index => match (a.field, b.field) {
                        (GenericField::Vector(a), GenericField::Scalar(b)) => {
                            self.push(words, ScalarField::Index(a.into(), b.into()))
                        }
                        (GenericField::Vector(a), GenericField::Vector(b)) => {
                            self.push(words, VectorField::Index(a.into(), b.into()))
                        }
                        _ => unreachable!(),
                    },
                }
            }
        }
        Ok(())
    }
}
