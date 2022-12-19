use crate::word::Word;

#[derive(Default)]
pub struct ConduitRack {
    pub conduits: Vec<ConduitStone>,
}

#[derive(Default)]
pub struct ConduitStone {
    pub words: Vec<Word>,
}

impl ConduitStone {
    pub fn etch(&mut self, words: impl IntoIterator<Item = Word>) {
        self.words = words.into_iter().filter(Word::etchable).collect();
    }
}
