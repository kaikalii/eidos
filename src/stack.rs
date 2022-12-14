use crate::{
    error::EidosError,
    field::*,
    function::*,
    person::PersonId,
    word::Word,
    world::{ActiveSpell, World},
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
    fn push(&mut self, words: impl IntoWords, field: impl Into<Field>) {
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
                InputFieldKind::Scalar(kind) => self.push(word, ScalarField::Input(kind)),
                InputFieldKind::Vector(kind) => self.push(word, VectorField::Input(kind)),
            },
            Function::WriteField(field_kind) => {
                let item = self.pop();
                match (field_kind, item.field) {
                    (OutputFieldKind::Vector(kind), Field::Vector(field)) => {
                        world
                            .active_spells
                            .vectors
                            .entry(self.person_id)
                            .or_default()
                            .entry(kind)
                            .or_default()
                            .push(ActiveSpell {
                                field,
                                words: item.words.into_iter().chain([word]).collect(),
                            });
                    }
                    _ => unreachable!(),
                }
                world.person_mut(self.person_id).words.clear();
            }
            Function::Control(kind) => self.push(word, ScalarField::Control(kind)),
            Function::Nullary(nullary) => self.push(word, nullary.field(self.person_id)),
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
                    UnOp::Math(op) => match a.field {
                        Field::Scalar(f) => self.push(
                            words,
                            ScalarField::ScalarUn(TypedUnOp::Math(op), f.into()).reduce(),
                        ),
                        Field::Vector(f) => self.push(
                            words,
                            VectorField::Un(TypedUnOp::Math(op), f.into()).reduce(),
                        ),
                    },
                    UnOp::Scalar(op) => match a.field {
                        Field::Scalar(f) => self.push(
                            words,
                            ScalarField::ScalarUn(TypedUnOp::Typed(op), f.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    UnOp::VectorScalar(op) => match a.field {
                        Field::Vector(f) => {
                            self.push(words, ScalarField::VectorUn(op, f.into()).reduce())
                        }
                        _ => unreachable!(),
                    },
                    UnOp::VectorVector(op) => match a.field {
                        Field::Vector(f) => self.push(
                            words,
                            VectorField::Un(TypedUnOp::Typed(op), f.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    UnOp::ToScalar(op) => match a.field {
                        Field::Scalar(f) => self.push(
                            words,
                            ScalarField::ScalarUn(
                                TypedUnOp::Typed(ScalarUnOp::ToScalar(op)),
                                f.into(),
                            )
                            .reduce(),
                        ),
                        Field::Vector(f) => self.push(
                            words,
                            ScalarField::VectorUn(VectorUnScalarOp::ToScalar(op), f.into())
                                .reduce(),
                        ),
                    },
                }
            }
            Function::Bin(op) => {
                let b = self.pop();
                let a = self.pop();
                let words = (a.words, b.words, word);
                match op {
                    BinOp::Math(op) => match (a.field, b.field) {
                        (Field::Scalar(a), Field::Scalar(b)) => {
                            self.push(
                                words,
                                ScalarField::Bin(TypedBinOp::Hetero(op), a.into(), b.into())
                                    .reduce(),
                            );
                        }
                        (Field::Scalar(a), Field::Vector(b)) => {
                            self.push(
                                words,
                                VectorField::BinSV(TypedBinOp::Hetero(op), a, b.into()).reduce(),
                            );
                        }
                        (Field::Vector(a), Field::Scalar(b)) => {
                            self.push(
                                words,
                                VectorField::BinVS(TypedBinOp::Hetero(op), a.into(), b).reduce(),
                            );
                        }
                        (Field::Vector(a), Field::Vector(b)) => {
                            self.push(
                                words,
                                VectorField::BinVV(TypedBinOp::Hetero(op), a.into(), b.into())
                                    .reduce(),
                            );
                        }
                    },
                    BinOp::Homo(op) => match (a.field, b.field) {
                        (Field::Scalar(a), Field::Scalar(b)) => self.push(
                            words,
                            ScalarField::Bin(TypedBinOp::Typed(op), a.into(), b.into()).reduce(),
                        ),
                        (Field::Vector(a), Field::Vector(b)) => self.push(
                            words,
                            VectorField::BinVV(TypedBinOp::Typed(op), a.into(), b.into()).reduce(),
                        ),
                        _ => unreachable!(),
                    },
                    BinOp::Index => match (a.field, b.field) {
                        (Field::Vector(a), Field::Scalar(b)) => {
                            self.push(words, ScalarField::Index(a.into(), b.into()))
                        }
                        (Field::Vector(a), Field::Vector(b)) => {
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
