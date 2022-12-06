use derive_more::{Display, From};
use enum_iterator::Sequence;

use crate::function::Function;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
pub enum Word {
    Command(SpellCommand),
    Function(Function),
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Sequence)]
pub enum SpellCommand {
    Clear,
}
