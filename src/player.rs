use std::collections::HashSet;

use rapier2d::prelude::RigidBodyHandle;

use crate::{field::GenericInputFieldKind, game::TICK_RATE, word::Word};

pub const MANA_REGEN_RATE: f32 = 1.0;
pub const MAX_MANA_EXHAUSTION: f32 = 5.0;

pub struct Player {
    pub body_handle: RigidBodyHandle,
    pub mana: f32,
    pub max_mana: f32,
    pub mana_exhaustion: f32,
    pub words: Vec<Word>,
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
    pub known_fields: HashSet<GenericInputFieldKind>,
    pub mana_bar: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Progression {
    fn default() -> Self {
        Progression {
            known_words: HashSet::new(),
            known_fields: HashSet::new(),
            mana_bar: false,
        }
    }
}

impl Player {
    pub fn new(name: String, gender: Gender) -> Self {
        Player {
            body_handle: RigidBodyHandle::default(),
            mana: 40.0,
            max_mana: 40.0,
            mana_exhaustion: 0.0,
            words: Vec::new(),
            progression: Progression::default(),
            name,
            gender,
        }
    }
    pub fn field_scale(&self) -> f32 {
        if self.mana_exhaustion > 0.0 {
            0.0
        } else {
            1.0
        }
    }
    pub fn do_work(&mut self, work: f32) {
        self.mana -= work;
        if self.mana < 0.0 {
            self.mana = 0.0;
            self.mana_exhaustion = MAX_MANA_EXHAUSTION;
        }
    }
    pub fn capped_mana(&self) -> f32 {
        self.max_mana - self.reserved_mana()
    }
    pub fn reserved_mana(&self) -> f32 {
        self.words.len() as f32
    }
    pub fn regen_mana(&mut self) {
        if self.mana_exhaustion > 0.0 {
            self.mana_exhaustion = (self.mana_exhaustion - TICK_RATE * MANA_REGEN_RATE).max(0.0);
        } else {
            self.mana = (self.mana + TICK_RATE * MANA_REGEN_RATE).min(self.capped_mana());
        }
    }
    pub fn can_cast(&self) -> bool {
        self.mana_exhaustion <= 0.0
    }
}
