use std::fmt;

use eframe::egui::*;

/// The Casting Assistant Device
pub struct Cad {
    lines: Vec<Vec<Instr>>,
    insertion: Option<Insertion>,
}

struct Insertion {
    line: usize,
    position: usize,
    instr: Instr,
    new: bool,
    finish: bool,
}

impl Insertion {
    fn new(line: usize, position: usize) -> Self {
        Insertion {
            line,
            position,
            instr: Instr::Number(0.0),
            new: true,
            finish: false,
        }
    }
}

#[derive(Debug)]
pub enum Instr {
    Number(f32),
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::Number(n) => n.fmt(f),
        }
    }
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
        let mut edit_position = None;
        let insertion_at = |ui: &mut Ui, insertion: &mut Option<Insertion>, i: usize, j: usize| {
            // Insertion prompt
            if let Some(ins) = insertion
                .as_mut()
                .filter(|ins| ins.line == i && ins.position == j)
            {
                if j > 0 {
                    ui.separator();
                }
                // Type and value
                CollapsingHeader::new(ins.instr.to_string())
                    .open(ins.new.then_some(true))
                    .show(ui, |ui| match &mut ins.instr {
                        Instr::Number(f) => {
                            DragValue::new(f).ui(ui);
                        }
                    });
                // Submit and cancel
                if ui.small_button("✔").clicked() {
                    ins.finish = true;
                }
                if ui.small_button("❌").clicked() {
                    *insertion = None;
                }
                true
            } else {
                false
            }
        };
        for (i, line) in self.lines.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if !insertion_at(ui, &mut self.insertion, i, 0)
                        && SeparatorButton::default().ui(ui).clicked()
                    {
                        self.insertion = Some(Insertion::new(i, 0))
                    }
                    for (j, instr) in line.iter_mut().enumerate() {
                        // This instruction
                        match instr {
                            Instr::Number(f) => {
                                if ui.selectable_label(false, f.to_string()).clicked() {
                                    edit_position = Some((i, j));
                                }
                            }
                        }
                        // Insertion button
                        if !insertion_at(ui, &mut self.insertion, i, j + 1)
                            && SeparatorButton::default().ui(ui).clicked()
                        {
                            self.insertion = Some(Insertion::new(i, j + 1))
                        }
                    }
                });
            });
        }
        // Add insertion
        if let Some(insertion) = &self.insertion {
            if insertion.finish {
                let insertion = self.insertion.take().unwrap();
                self.lines[insertion.line].insert(insertion.position, insertion.instr);
            }
        }
        // Edit instruction
        if let Some((i, j)) = edit_position {
            let instr = self.lines[i].remove(j);
            let mut insertion = Insertion::new(i, j);
            insertion.instr = instr;
            self.insertion = Some(insertion);
        }
    }
}

/// A clickable separator
pub struct SeparatorButton {
    spacing: f32,
    is_horizontal_line: Option<bool>,
}

impl Default for SeparatorButton {
    fn default() -> Self {
        Self {
            spacing: 6.0,
            is_horizontal_line: None,
        }
    }
}

#[allow(dead_code)]
impl SeparatorButton {
    /// How much space we take up. The line is painted in the middle of this.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
    /// Explicitly ask for a horizontal line.
    /// By default you will get a horizontal line in vertical layouts,
    /// and a vertical line in horizontal layouts.
    pub fn horizontal(mut self) -> Self {
        self.is_horizontal_line = Some(true);
        self
    }
    /// Explicitly ask for a vertical line.
    /// By default you will get a horizontal line in vertical layouts,
    /// and a vertical line in horizontal layouts.
    pub fn vertical(mut self) -> Self {
        self.is_horizontal_line = Some(false);
        self
    }
}

impl Widget for SeparatorButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let SeparatorButton {
            spacing,
            is_horizontal_line,
        } = self;

        let is_horizontal_line =
            is_horizontal_line.unwrap_or_else(|| !ui.layout().main_dir().is_horizontal());

        let available_space = ui.available_size_before_wrap();

        let size = if is_horizontal_line {
            vec2(available_space.x, spacing)
        } else {
            vec2(spacing, available_space.y)
        };

        let (rect, response) = ui.allocate_at_least(size, Sense::click());

        if ui.is_rect_visible(response.rect) {
            let stroke = if response.hovered() || response.has_focus() {
                ui.visuals().widgets.hovered.bg_stroke
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke
            };
            if is_horizontal_line {
                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
            } else {
                ui.painter().vline(rect.center().x, rect.y_range(), stroke);
            }
        }

        response
    }
}
