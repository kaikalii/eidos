use std::f64;

use eframe::{
    egui::{plot::*, *},
    epaint::color::Hsva,
};
use eidos::{EidosError, Field, Function, FunctionCategory, Instr, Runtime, Value};
use enum_iterator::all;

use itertools::Itertools;

use crate::controls::SeparatorButton;

/// The Spell Verification Assistant
pub struct Sva {
    lines: Vec<Vec<SvaInstr>>,
    dragging: Option<(usize, usize)>,
    keep_evaluating: bool,
}

impl Default for Sva {
    fn default() -> Self {
        Sva {
            lines: vec![vec![]],
            dragging: None,
            keep_evaluating: true,
        }
    }
}

struct SvaInstr {
    instr: Instr,
    editing: bool,
    buffer: Option<String>,
    header_open: Option<bool>,
}

impl Default for SvaInstr {
    fn default() -> Self {
        SvaInstr::new(Instr::Number(0.0))
    }
}

impl SvaInstr {
    fn new(instr: Instr) -> Self {
        SvaInstr {
            instr,
            editing: true,
            buffer: None,
            header_open: None,
        }
    }
    fn set_instr(&mut self, instr: impl Into<Instr>) {
        self.instr = instr.into();
        self.header_open = Some(false);
    }
}

impl Sva {
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
                                Value::Field(f) => plot_field(ui, f, i, j),
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
                        self.lines[i].insert(j + 1, SvaInstr::default());
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
            self.lines[i].insert(j, SvaInstr::default());
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

fn plot_number(ui: &mut Ui, n: f32, i: usize, j: usize) {
    Plot::new((i, j))
        .width(50.0)
        .height(50.0)
        .show_axes([false; 2])
        .allow_zoom(false)
        .allow_drag(false)
        .include_x(-2.0)
        .include_x(2.0)
        .include_y(-2.0)
        .include_y(2.0)
        .allow_scroll(false)
        .show(ui, |plot_ui| {
            const FLOWER_MAX: f32 = 10.0;
            let frac = (n as f64) % 1.0;
            let ones_part = ((n % FLOWER_MAX).abs().floor() * n.signum()) as f64;
            let tens_part = ((n / FLOWER_MAX % FLOWER_MAX).abs().floor() * n.signum()) as f64;
            let hundreds_part = ((n / (FLOWER_MAX * FLOWER_MAX)).abs().floor() * n.signum()) as f64;
            // Fractional circle
            plot_ui.line(
                Line::new(PlotPoints::from_parametric_callback(
                    |t| {
                        let theta = t * 2.0 * f64::consts::PI;
                        (theta.cos(), theta.sin())
                    },
                    0.0..=frac,
                    100,
                ))
                .color(Color32::GREEN),
            );
            // Hundreds flower
            if hundreds_part.abs() >= 1.0 {
                plot_ui.line(
                    Line::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = 1.0 + (theta * hundreds_part / 2.0).cos().powf(16.0);
                            let x = r * theta.cos();
                            let y = r * theta.sin();
                            (x, y)
                        },
                        0.0..=1.0,
                        100,
                    ))
                    .color(Color32::YELLOW),
                );
            }
            // Tens flower
            if tens_part.abs() >= 1.0 {
                plot_ui.line(
                    Line::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = 1.0 + (theta * tens_part * 0.5).cos().powf(2.0);
                            let x = r * theta.cos();
                            let y = r * theta.sin();
                            (x, y)
                        },
                        0.0..=1.0,
                        100,
                    ))
                    .color(Color32::from_rgb(0, 100, 255)),
                );
            }
            // Ones flower
            if n == 0.0 || ones_part.abs() >= 1.0 {
                plot_ui.line(
                    Line::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = 1.0 + (theta * ones_part).cos();
                            let x = r * theta.cos();
                            let y = r * theta.sin();
                            (x, y)
                        },
                        0.0..=1.0,
                        100,
                    ))
                    .color(Color32::RED),
                );
            }
        });
}

fn plot_field(ui: &mut Ui, field: &Field, i: usize, j: usize) {
    match field.rank() {
        0 => {
            let n = field.as_scalar().unwrap();
            plot_number(ui, n, i, j);
        }
        1 => {
            let mut plot = Plot::new((i, j))
                .width(200.0)
                .height(100.0)
                .allow_scroll(false);
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
            let field_clone = field.clone();
            let range = field.default_range().unwrap_or(0.0..=10.0);
            let start = range.start().min(0.0);
            let end = range.end().max(0.0);
            let mut plot = Plot::new((i, j))
                .width(200.0)
                .height(100.0)
                .include_x(start)
                .include_x(end)
                .include_y(-1.0)
                .include_y(1.0)
                .data_aspect(1.0)
                .allow_scroll(false)
                .label_formatter(move |_, point| {
                    let z = field_clone
                        .sample(point.x as f32)
                        .sample(point.y as f32)
                        .as_scalar()
                        .unwrap();
                    format!("({:.2} {:.2} {:.2})", point.x, point.y, z)
                });
            if let Some((min, max)) = field.min_max() {
                plot = plot.include_y(min).include_y(max);
            }
            plot.show(ui, |plot_ui| {
                const WIDTH: usize = 80;
                const HEIGHT: usize = 40;
                const Z_BUCKETS: usize = 99;
                let field = field.clone();
                let bounds = plot_ui.plot_bounds();
                let [min_x, min_y] = bounds.min().map(|d| d as f32);
                let [max_x, max_y] = bounds.max().map(|d| d as f32);
                let step_x = (max_x - min_x) / WIDTH as f32;
                let mut points = Vec::with_capacity(WIDTH * HEIGHT);
                for k in 0..WIDTH {
                    let x = k as f32 * step_x + min_x;
                    let step_y = (max_y - min_y) / HEIGHT as f32;
                    for l in 0..HEIGHT {
                        let y = l as f32 * step_y + min_y;
                        let z = field.sample(x).sample(y).as_scalar().unwrap();
                        points.push((x, y, z));
                    }
                }
                let (min_z, max_z) = points
                    .iter()
                    .map(|(_, _, z)| *z)
                    .minmax()
                    .into_option()
                    .unwrap();
                let max_abs_z = min_z.abs().max(max_z.abs());
                let mut grouped_points = vec![Vec::new(); Z_BUCKETS];
                for (x, y, z) in points {
                    let group = ((z / max_abs_z * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                        .max(0.0)
                        .round() as usize)
                        .min(Z_BUCKETS - 1);
                    grouped_points[group].push(PlotPoint::new(x, y));
                }
                for (k, points) in grouped_points.into_iter().enumerate() {
                    let h = 0.9 * (1.0 - k as f32 / Z_BUCKETS as f32);
                    let v = (2.0 * k as f32 / Z_BUCKETS as f32 - 1.0).abs();
                    let s = v.powf(0.5);
                    plot_ui.points(
                        Points::new(PlotPoints::Owned(points))
                            .shape(MarkerShape::Circle)
                            .radius(2.5)
                            .color(Hsva::new(h, s, v, 1.0)),
                    );
                }
                plot_ui.vline(VLine::new(0.0));
                plot_ui.hline(HLine::new(0.0));
            });
        }
        _ => {
            ui.label(field.to_string());
        }
    }
}
