use std::{collections::HashMap, iter::empty};

use derive_more::From;
use eframe::epaint::Pos2;
use enum_iterator::Sequence;

use crate::{conduit::ConduitRack, field::*, npc::NpcId, stack::Stack, word::Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Sequence)]
pub enum PersonId {
    Player,
    Npc(NpcId),
}

pub struct Person {
    pub max_mana: f32,
    pub target: Option<Pos2>,
    pub stack: Stack,
    pub rack: ConduitRack,
    pub active_spells: ActiveSpells,
}

impl Person {
    pub fn new(max_mana: f32) -> Person {
        Person {
            max_mana,
            target: None,
            stack: Stack::default(),
            rack: ConduitRack::default(),
            active_spells: ActiveSpells::default(),
        }
    }
    pub fn reserved_mana(&self) -> f32 {
        let from_scalars: f32 = self
            .active_spells
            .scalars
            .values()
            .flatten()
            .flat_map(|spell| &spell.words)
            .map(|word| word.cost())
            .sum();
        let from_vectors: f32 = self
            .active_spells
            .vectors
            .values()
            .flatten()
            .flat_map(|spell| &spell.words)
            .map(|word| word.cost())
            .sum();
        let from_stack: f32 = self
            .stack
            .iter()
            .flat_map(|item| &item.words)
            .map(|word| word.cost())
            .sum();
        from_scalars + from_vectors + from_stack
    }
    pub fn capped_mana(&self) -> f32 {
        self.max_mana - self.reserved_mana()
    }
}

type TypedActiveSpells<K, V> = HashMap<K, Vec<ActiveSpell<V>>>;

#[derive(Default)]
pub struct ActiveSpells {
    pub scalars: TypedActiveSpells<ScalarOutputFieldKind, ScalarField>,
    pub vectors: TypedActiveSpells<VectorOutputFieldKind, VectorField>,
}

pub struct ActiveSpell<T> {
    pub field: T,
    pub words: Vec<Word>,
}

impl ActiveSpells {
    pub fn contains(&self, kind: OutputFieldKind) -> bool {
        match kind {
            OutputFieldKind::Scalar(kind) => self.scalars.contains_key(&kind),
            OutputFieldKind::Vector(kind) => self.vectors.contains_key(&kind),
        }
    }
    pub fn remove(&mut self, kind: OutputFieldKind, i: usize) {
        match kind {
            OutputFieldKind::Scalar(kind) => {
                self.scalars.entry(kind).or_default().remove(i);
            }
            OutputFieldKind::Vector(kind) => {
                self.vectors.entry(kind).or_default().remove(i);
            }
        }
    }
    /// Get an iterator over all the words of all the active spells of a given kind.
    pub fn spell_words(
        &self,
        kind: OutputFieldKind,
    ) -> Box<dyn ExactSizeIterator<Item = &[Word]> + '_> {
        match kind {
            OutputFieldKind::Scalar(kind) => {
                let Some(spells) = self.scalars.get(&kind) else {
                    return Box::new(empty());
                };
                Box::new(spells.iter().map(|spell| spell.words.as_slice()))
            }
            OutputFieldKind::Vector(kind) => {
                let Some(spells) = self.vectors.get(&kind) else {
                    return Box::new(empty());
                };
                Box::new(spells.iter().map(|spell| spell.words.as_slice()))
            }
        }
    }
}
