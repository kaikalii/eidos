use eframe::egui::*;

/// The Casting Assistant Device
pub struct Cad {
    pub(crate) lines: Vec<Vec<Instr>>,
    pub(crate) insertion: Option<[usize; 2]>,
}

pub enum Instr {
    Number(f32),
}

impl Default for Cad {
    fn default() -> Self {
        Cad {
            lines: vec![vec![]],
            insertion: None,
        }
    }
}

impl Cad {
    pub fn ui(&mut self, ui: &mut Ui) {
        let lines_len = self.lines.len();
        for (i, line) in self.lines.iter_mut().enumerate() {
            for instr in line {}
            // Allow adding
            if i == lines_len - 1 {}
        }
    }
}
