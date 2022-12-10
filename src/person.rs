use derive_more::From;
use eframe::epaint::Pos2;
use enum_iterator::Sequence;
use rapier2d::prelude::RigidBodyHandle;

use crate::{game::TICK_RATE, word::Word};

pub const MANA_REGEN_RATE: f32 = 2.0;
pub const MAX_MANA_EXHAUSTION: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Sequence)]
pub enum PersonId {
    Player,
    Npc(NpcId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Sequence)]
pub enum NpcId {}

pub struct Npc {
    pub person: Person,
}

pub struct Person {
    pub pos: Pos2,
    pub body_handle: RigidBodyHandle,
    pub mana: f32,
    pub max_mana: f32,
    pub mana_exhaustion: f32,
    pub words: Vec<Word>,
}

impl Person {
    pub fn new(max_mana: f32) -> Person {
        Person {
            pos: Pos2::ZERO,
            body_handle: RigidBodyHandle::default(),
            mana: max_mana,
            max_mana,
            mana_exhaustion: 0.0,
            words: Vec::new(),
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
    pub fn reserved_mana(&self) -> f32 {
        self.words.iter().map(|word| word.cost()).sum()
    }
    pub fn capped_mana(&self) -> f32 {
        self.max_mana - self.reserved_mana()
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
