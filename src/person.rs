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
    pub max_mana: f32,
    pub words: Vec<Word>,
    pub target: Option<Pos2>,
}

impl Person {
    pub fn new(max_mana: f32) -> Person {
        Person {
            max_mana,
            words: Vec::new(),
            target: None,
        }
    }
}
