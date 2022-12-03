use eframe::egui::*;
use eidos::{Function, Instr, Runtime, Value};
use enum_iterator::all;

use crate::controls::SeparatorButton;

/// The Casting Assistant Device
pub struct Cad {
    lines: Vec<Vec<CadInstr>>,
    dragging: Option<(usize, usize)>,
}

struct CadInstr {
    instr: Instr,
    editing: bool,
}

impl Default for Cad {
    fn default() -> Self {
        Cad {
            lines: vec![vec![]],
            dragging: None,
        }
    }
}

impl Cad {
    pub fn ui(&mut self, ui: &mut Ui) {
        // Initialize runtime
        let mut rt = Runtime::default();
        let mut keep_evaluating = true;
        // Main ui and execution loop
        for i in 0..self.lines.len() {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    // Insertion at start of the line
                    self.insertion_at(ui, i, 0);
                    // Show the instructions
                    for j in 0..self.lines[i].len() {
                        let ci = &mut self.lines[i][j];
                        // Editing
                        if ci.editing {
                            let mut number_choice = true;
                            let mut selected_function = None;
                            // Show the current value
                            ui.vertical(|ui| {
                                match &mut ci.instr {
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
                                // Allow number selection
                                if number_choice && ui.button("Number").clicked() {
                                    ci.instr = Instr::Number(0.0);
                                }
                                // Partition functions
                                let mut available = Vec::new();
                                let mut unavailable = Vec::new();
                                for function in all::<Function>() {
                                    match rt.function_ret_type(&function) {
                                        Ok(_) => available.push(function),
                                        Err(e) => unavailable.push((function, e)),
                                    }
                                }
                                // Show all functions
                                ui.add_enabled_ui(!available.is_empty(), |ui| {
                                    ComboBox::new((i, j), "")
                                        .selected_text("Functions")
                                        .show_ui(ui, |ui| {
                                            // Show available functions
                                            for function in available {
                                                if ui
                                                    .selectable_label(
                                                        selected_function.as_ref()
                                                            == Some(&function),
                                                        function.to_string(),
                                                    )
                                                    .clicked()
                                                {
                                                    ci.instr = Instr::Function(function)
                                                }
                                            }
                                            // Show unavailable functions
                                            for (function, e) in unavailable {
                                                ui.add_enabled(
                                                    false,
                                                    SelectableLabel::new(
                                                        selected_function.as_ref()
                                                            == Some(&function),
                                                        function.to_string(),
                                                    ),
                                                )
                                                .on_disabled_hover_text(
                                                    e.to_string().as_str().replace(". ", "\n"),
                                                );
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
                                        ui.small_button("✔").clicked()
                                            || ui.input().key_pressed(Key::Enter),
                                        ui.small_button("❌").clicked(),
                                    )
                                })
                                .inner;
                            if finished {
                                ci.editing = false;
                            }
                            if cancelled {
                                self.lines[i].remove(j);
                                break;
                            }
                        }
                        // Execute this instruction
                        let mut label_text = RichText::new(ci.instr.to_string());
                        let mut error = None;
                        if keep_evaluating {
                            if let Err(e) = rt.do_instr(&ci.instr) {
                                label_text = label_text.color(Color32::RED);
                                keep_evaluating = false;
                                error = Some(e);
                            }
                        }
                        // Not editing
                        if !ci.editing {
                            let visuals = ui.visuals().clone();
                            ui.visuals_mut().widgets.inactive = ui.visuals().widgets.noninteractive;
                            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::none();
                            let mut button_resp = Button::new(label_text)
                                .sense(Sense::click_and_drag())
                                .ui(ui);
                            *ui.visuals_mut() = visuals;
                            if button_resp.drag_started() {
                                self.dragging = Some((i, j));
                            }
                            if let Some(error) = error {
                                button_resp = button_resp
                                    .on_hover_text(error.to_string().as_str().replace(". ", "\n"))
                            }
                            if button_resp.clicked() {
                                ci.editing = true;
                                self.clear_editing_other_than(i, j);
                            }
                        }
                        // Insertion after this instruction
                        self.insertion_at(ui, i, j + 1);
                    }
                });
                // Show stack
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
    }
    fn insertion_at(&mut self, ui: &mut Ui, i: usize, mut j: usize) {
        let sep_resp = SeparatorButton::default()
            .hilight(self.dragging.is_some())
            .ui(ui);
        if sep_resp.clicked() {
            self.lines[i].insert(
                j,
                CadInstr {
                    instr: Instr::Number(0.0),
                    editing: true,
                },
            );
            self.clear_editing_other_than(i, j);
        } else if sep_resp.hovered() && ui.input().pointer.any_released() {
            if let Some((i2, j2)) = self.dragging.take() {
                let ci = self.lines[i2].remove(j2);
                if j2 < j {
                    j -= 1;
                }
                self.lines[i].insert(j, ci);
            }
        }
    }
    fn clear_editing_other_than(&mut self, i: usize, j: usize) {
        for (i2, line) in self.lines.iter_mut().enumerate() {
            for (j2, ci) in line.iter_mut().enumerate() {
                if !(i == i2 && j == j2) {
                    ci.editing = false;
                }
            }
        }
    }
}
