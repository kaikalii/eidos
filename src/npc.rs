use std::{collections::HashMap, fs};

use enum_iterator::Sequence;
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::{
    person::Person,
    utils::{fatal_error, resources_path},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Sequence, Deserialize)]
pub enum NpcId {
    Leavy,
}

#[derive(Debug, Deserialize)]
pub struct NpcDef {
    pub max_mana: f32,
}

pub struct Npc {
    pub person: Person,
}

pub static NPCS: Lazy<HashMap<NpcId, NpcDef>> =
    Lazy::new(|| load_npcs().unwrap_or_else(|e| fatal_error(format!("Error loading npcs: {e}"))));

fn load_npcs() -> anyhow::Result<HashMap<NpcId, NpcDef>> {
    let yaml = fs::read_to_string(resources_path().join("npcs.yaml"))?;
    Ok(serde_yaml::from_str(&yaml)?)
}
