use eframe::egui::*;
use eidos::{Function, Instr, Runtime, Value};
use enum_iterator::all;

/// The Casting Assistant Device
pub struct Cad {
    lines: Vec<Vec<Instr>>,
    insertion: Option<Insertion>,
}

struct Insertion {
    line: usize,
    position: usize,
    instr: Instr,
    finish: bool,
}

impl Insertion {
    fn new(line: usize, position: usize) -> Self {
        Insertion {
            line,
            position,
            instr: Instr::Number(0.0),
            finish: false,
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
        // Initialize runtime
        let mut rt = Runtime::default();
        // Main ui and execution loop
        for i in 0..self.lines.len() {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    // Insertion at start
                    self.insertion_at(ui, &mut rt, i, 0);
                    for j in 0..self.lines[i].len() {
                        let instr = &self.lines[i][j];
                        // Execute this instruction
                        rt.do_instr(instr);
                        // Show this instruction
                        if ui.selectable_label(false, instr.to_string()).clicked() {
                            edit_position = Some((i, j));
                        }
                        // Insertion after this instruction
                        self.insertion_at(ui, &mut rt, i, j + 1);
                    }
                });
                if !rt.stack.is_empty() {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        for value in &rt.stack {
                            ui.separator();
                            match value {
                                Value::Atom(f) => ui.label(f.to_string()),
                                Value::F1(_) => todo!(),
                                Value::F2(_) => todo!(),
                                Value::Function(f) => ui.label(f.to_string()),
                            };
                        }
                    });
                }
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
    fn insertion_at(&mut self, ui: &mut Ui, rt: &mut Runtime, i: usize, j: usize) {
        if let Some(ins) = &mut self
            .insertion
            .as_mut()
            .filter(|ins| ins.line == i && ins.position == j)
        {
            if j > 0 {
                ui.separator();
            }
            // Type and value
            let mut number_choice = true;
            let mut selected_function = None;
            ui.vertical(|ui| {
                match &mut ins.instr {
                    Instr::Number(f) => {
                        ui.horizontal(|ui| {
                            ui.small("Number:");
                            DragValue::new(f).ui(ui);
                        });
                        number_choice = false;
                    }
                    Instr::Function(f) => {
                        ui.horizontal(|ui| {
                            ui.small("Function:");
                            ui.label(f.to_string());
                        });
                        selected_function = Some(f.clone());
                    }
                }
                if number_choice && ui.button("Number").clicked() {
                    ins.instr = Instr::Number(0.0);
                }
                let mut available = Vec::new();
                let mut unavailable = Vec::new();
                for function in all::<Function>() {
                    match rt.function_ret_type(&function) {
                        Ok(_) => available.push(function),
                        Err(e) => unavailable.push((function, e)),
                    }
                }
                ui.add_enabled_ui(!available.is_empty(), |ui| {
                    ComboBox::new("functions", "")
                        .selected_text("Functions")
                        .show_ui(ui, |ui| {
                            for function in available {
                                if ui
                                    .selectable_label(
                                        selected_function.as_ref() == Some(&function),
                                        function.to_string(),
                                    )
                                    .clicked()
                                {
                                    ins.instr = Instr::Function(function)
                                }
                            }
                            for (function, e) in unavailable {
                                ui.add_enabled(
                                    false,
                                    SelectableLabel::new(
                                        selected_function.as_ref() == Some(&function),
                                        function.to_string(),
                                    ),
                                )
                                .on_disabled_hover_text(e.to_string().as_str().replace(". ", "\n"));
                            }
                        });
                })
                .response
                .on_hover_text("No functions are available");
            });
            // Submit and cancel
            let (finished, cancelled) = ui
                .vertical(|ui| {
                    (
                        ui.small_button("✔").clicked() || ui.input().key_pressed(Key::Enter),
                        ui.small_button("❌").clicked(),
                    )
                })
                .inner;
            if finished {
                ins.finish = true;
            }
            if cancelled {
                self.insertion = None;
            }
        } else if SeparatorButton::default().ui(ui).clicked() {
            self.insertion = Some(Insertion::new(i, j))
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
