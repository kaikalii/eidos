use std::collections::HashSet;

use crate::{field::InputFieldKind, person::Person, word::Word};

pub struct Player {
    pub person: Person,
    pub progression: Progression,
    pub name: String,
    pub gender: Gender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    Enby,
}

pub struct Progression {
    pub known_words: HashSet<Word>,
    pub known_fields: HashSet<InputFieldKind>,
    pub mana_bar: bool,
    pub free: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Progression {
    fn default() -> Self {
        Progression {
            known_words: HashSet::new(),
            known_fields: HashSet::new(),
            mana_bar: false,
            free: false,
        }
    }
}

impl Player {
    pub fn new(name: String, gender: Gender) -> Self {
        Player {
            person: Person::new(50.0),
            progression: Progression::default(),
            name,
            gender,
        }
    }
}
