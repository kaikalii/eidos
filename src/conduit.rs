use crate::word::Word;

pub struct ConduitRack {
    pub conduits: Vec<ConduitStone>,
}

impl ConduitRack {
    pub fn new(size: usize) -> Self {
        ConduitRack {
            conduits: vec![ConduitStone::default(); size],
        }
    }
}

#[derive(Default, Clone)]
pub struct ConduitStone {
    pub words: Vec<Word>,
}

impl ConduitStone {
    pub fn etch(&mut self, words: impl IntoIterator<Item = Word>) {
        self.words = words.into_iter().filter(Word::etchable).collect();
    }
    pub fn format(&self, max_length: usize) -> String {
        if self.words.is_empty() {
            return "...".into();
        }
        let mut s = String::new();
        for word in &self.words {
            let word = word.to_string();
            if s.len() + word.len() + 1 > max_length {
                s.push_str("...");
                break;
            }
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&word);
        }
        s
    }
}
