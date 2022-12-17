use derive_more::From;
use eframe::epaint::Pos2;
use enum_iterator::Sequence;

use crate::{npc::NpcId, word::Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Sequence)]
pub enum PersonId {
    Player,
    Npc(NpcId),
}

pub struct Person {
    pub mana: f32,
    pub max_mana: f32,
    pub mana_exhaustion: f32,
    pub words: Vec<Word>,
    pub target: Option<Pos2>,
}

impl Person {
    pub fn new(max_mana: f32) -> Person {
        Person {
            mana: max_mana,
            max_mana,
            mana_exhaustion: 0.0,
            words: Vec::new(),
            target: None,
        }
    }
    pub fn field_scale(&self) -> f32 {
        if self.mana_exhaustion > 0.0 {
            0.0
        } else {
            1.0
        }
    }
    pub fn reserved_mana(&self) -> f32 {
        self.words.iter().map(|word| word.cost()).sum()
    }
    pub fn capped_mana(&self) -> f32 {
        self.max_mana - self.reserved_mana()
    }
}
