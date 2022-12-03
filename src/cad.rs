use eframe::{egui::*, epaint::color::Hsva};
use eidos::{EidosError, Field, Function, FunctionCategory, Instr, Runtime, Value};
use enum_iterator::all;

use itertools::Itertools;

use crate::controls::SeparatorButton;

/// The Casting Assistant Device
pub struct Cad {
    lines: Vec<Vec<CadInstr>>,
    dragging: Option<(usize, usize)>,
    keep_evaluating: bool,
}

impl Default for Cad {
    fn default() -> Self {
        Cad {
            lines: vec![vec![]],
            dragging: None,
            keep_evaluating: true,
        }
    }
}

struct CadInstr {
    instr: Instr,
    editing: bool,
    buffer: Option<String>,
    header_open: Option<bool>,
}

impl Default for CadInstr {
    fn default() -> Self {
        CadInstr {
            instr: Instr::Number(0.0),
            editing: true,
            buffer: None,
            header_open: None,
        }
    }
}

impl CadInstr {
    fn set_instr(&mut self, instr: impl Into<Instr>) {
        self.instr = instr.into();
        self.header_open = Some(false);
    }
}

impl Cad {
    pub fn ui(&mut self, ui: &mut Ui) {
        // Initialize runtime
        let mut rt = Runtime::default();
        self.keep_evaluating = true;
        // Main ui and execution loop
        for i in 0..self.lines.len() {
            ui.group(|ui| {
                self.row_ui(ui, &mut rt, i);
                // Show stack
                if !rt.stack.is_empty() {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        for (j, value) in rt.stack.iter().enumerate() {
                            ui.separator();
                            match value {
                                Value::Field(f) => plot(ui, f, i, j),
                                Value::Function(f) => {
                                    ui.label(f.to_string());
                                }
                            };
                        }
                    });
                }
            });
        }
    }
    fn row_ui(&mut self, ui: &mut Ui, rt: &mut Runtime, i: usize) {
        ui.horizontal_wrapped(|ui| {
            // Insertion at start of the line
            self.insertion_at(ui, i, 0);
            // Show the instructions
            for j in 0..self.lines[i].len() {
                let Some(ci) = self.lines[i].get_mut(j) else {
                    continue;
                };
                // Editing
                if ci.editing {
                    let mut number_choice = true;
                    let mut list_choice = true;
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
                            Instr::List(nums) => {
                                let buffer = ci.buffer.get_or_insert_with(|| {
                                    nums.iter()
                                        .map(f32::to_string)
                                        .intersperse(" ".into())
                                        .collect()
                                });
                                if TextEdit::singleline(buffer)
                                    .desired_width(100.0)
                                    .ui(ui)
                                    .changed()
                                {
                                    nums.clear();
                                    for word in buffer
                                        .split_whitespace()
                                        .flat_map(|s| s.split(','))
                                        .filter(|s| !s.is_empty())
                                    {
                                        if let Ok(num) = word.parse::<f64>() {
                                            nums.push(num as f32);
                                        }
                                    }
                                }
                                list_choice = false;
                            }
                            Instr::Function(f) => {
                                ui.horizontal(|ui| {
                                    ui.small("Function:");
                                    ui.label(f.to_string());
                                });
                                selected_function = Some(f.clone());
                            }
                        }
                        // Allow simple selections
                        if number_choice && ui.selectable_label(false, "Number").clicked() {
                            ci.set_instr(Instr::Number(0.0));
                        }
                        if list_choice && ui.selectable_label(false, "List").clicked() {
                            ci.set_instr(Instr::List(Vec::new()));
                        }
                        // Sort functions
                        type CategoryFunctions = Vec<(Function, Option<EidosError>)>;
                        let mut functions: Vec<(String, CategoryFunctions)> =
                            all::<FunctionCategory>()
                                .map(|category| {
                                    let mut functions: Vec<_> = category
                                        .functions()
                                        .map(|function| {
                                            let error = rt.validate_function_use(&function).err();
                                            (function, error)
                                        })
                                        .collect();
                                    functions.sort_by_key(|(_, error)| error.is_some());
                                    (format!("{category:?}"), functions)
                                })
                                .collect();
                        functions.sort_by_key(|(_, functions)| {
                            functions.iter().filter(|(_, e)| e.is_some()).count()
                        });
                        // Show all functions
                        CollapsingHeader::new("Functions")
                            .id_source((i, j))
                            .open(ci.header_open.take())
                            .show(ui, |ui| {
                                #[allow(clippy::single_element_loop)]
                                for function in [Function::Identity] {
                                    let selected = selected_function.as_ref() == Some(&function);
                                    if ui
                                        .selectable_label(selected, function.to_string())
                                        .clicked()
                                    {
                                        ci.set_instr(Instr::Function(function));
                                    }
                                }
                                for (k, (name, functions)) in functions.into_iter().enumerate() {
                                    let enabled = functions.iter().any(|(_, e)| e.is_none());
                                    ui.add_enabled_ui(enabled, |ui| {
                                        ComboBox::new((i, j, k), "")
                                            .width(89.0)
                                            .selected_text(&name)
                                            .show_ui(ui, |ui| {
                                                for (function, error) in functions {
                                                    let selected = selected_function.as_ref()
                                                        == Some(&function);
                                                    let resp = ui.add_enabled(
                                                        error.is_none(),
                                                        SelectableLabel::new(
                                                            selected,
                                                            function.to_string(),
                                                        ),
                                                    );
                                                    if resp.clicked() {
                                                        ci.set_instr(Instr::Function(function));
                                                    }
                                                    if let Some(e) = error {
                                                        resp.on_disabled_hover_text(
                                                            e.to_string()
                                                                .as_str()
                                                                .replace(". ", "\n"),
                                                        );
                                                    }
                                                }
                                            });
                                    })
                                    .response
                                    .on_hover_text(format!("No {name:?} functions are available"));
                                }
                            });
                    });
                    // Submit and cancel
                    let (do_next, finished, cancelled) = ui
                        .vertical(|ui| {
                            (
                                ui.small_button("➡").clicked()
                                    || ui.input().key_pressed(Key::Enter),
                                ui.small_button("✔").clicked(),
                                ui.small_button("❌").clicked(),
                            )
                        })
                        .inner;
                    if do_next {
                        ci.editing = false;
                        self.lines[i].insert(j + 1, CadInstr::default());
                        break;
                    }
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
                if self.keep_evaluating {
                    if let Err(e) = rt.do_instr(&ci.instr) {
                        label_text = label_text.color(Color32::RED);
                        self.keep_evaluating = false;
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
    }
    fn insertion_at(&mut self, ui: &mut Ui, i: usize, mut j: usize) {
        let sep_resp = SeparatorButton::default()
            .hilight(self.dragging.is_some())
            .ui(ui);
        if sep_resp.clicked() {
            self.lines[i].insert(j, CadInstr::default());
            self.clear_editing_other_than(i, j);
        } else if sep_resp.hovered() && ui.input().pointer.any_released() {
            if let Some((i2, j2)) = self.dragging.take() {
                let ci = self.lines[i2].remove(j2);
                if j2 < j {
                    j -= 1;
                }
                self.lines[i].insert(j, ci);
            }
        } else {
            sep_resp.context_menu(|ui| {
                if ui.selectable_label(false, "split line").clicked() {
                    ui.close_menu();
                    let new_line = self.lines[i].split_off(j);
                    self.lines.insert(i + 1, new_line);
                }
            });
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

fn plot(ui: &mut Ui, field: &Field, i: usize, j: usize) {
    use plot::*;
    match field.rank() {
        1 => {
            let mut plot = Plot::new((i, j)).width(200.0).height(100.0);
            if let Some((min, max)) = field.min_max() {
                plot = plot.include_y(min).include_y(max);
            }
            plot.show(ui, |plot_ui| {
                let field = field.clone();
                let range = field.default_range();
                let get_point = move |x| field.sample(x as f32).as_scalar().unwrap() as f64;
                let plot_points = if let Some(range) = range {
                    let range = *range.start() as f64..=*range.end() as f64;
                    PlotPoints::from_explicit_callback(get_point, range, 131)
                } else {
                    PlotPoints::from_explicit_callback(get_point, .., 131)
                };
                plot_ui.line(Line::new(plot_points))
            });
        }
        2 => {
            let mut plot = Plot::new((i, j)).width(200.0).height(100.0);
            if let Some((min, max)) = field.min_max() {
                plot = plot.include_y(min).include_y(max);
            }
            plot.show(ui, |plot_ui| {
                let field = field.clone();
                let range = field.default_range().unwrap_or(0.0..=10.0);
                const LINES: usize = 10;
                for (k, subfield) in field.sample_range_count(range, LINES).enumerate() {
                    let range = subfield.default_range();
                    let get_point = move |x| subfield.sample(x as f32).as_scalar().unwrap() as f64;
                    let plot_points = if let Some(range) = range {
                        let range = *range.start() as f64..=*range.end() as f64;
                        PlotPoints::from_explicit_callback(get_point, range, 131)
                    } else {
                        PlotPoints::from_explicit_callback(get_point, .., 131)
                    };
                    plot_ui.line(Line::new(plot_points).color(Hsva::new(
                        k as f32 / LINES as f32,
                        1.0,
                        1.0,
                        1.0,
                    )))
                }
            });
        }
        _ => {
            ui.label(field.to_string());
        }
    }
}
