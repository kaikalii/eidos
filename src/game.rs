use std::{collections::BTreeSet, time::Instant};

use eframe::{
    egui::{style::Margin, *},
    epaint::{ahash::HashMap, color::Hsva},
};
use enum_iterator::{all, Sequence};

use crate::{
    field::*,
    plot::{default_scalar_color, default_vector_color, FieldPlot, MapPlot},
    stack::Stack,
    word::SpellCommand,
    word::*,
    world::{World, MAX_MANA_EXHAUSTION},
};

pub const TICK_RATE: f32 = 1.0 / 60.0;

pub struct Game {
    pub world: World,
    ui_state: UiState,
    last_time: Instant,
    ticker: f32,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            world: World::default(),
            ui_state: UiState::default(),
            last_time: Instant::now(),
            ticker: 0.0,
        }
    }
}

struct UiState {
    fields_visible: HashMap<GenericFieldKind, bool>,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            fields_visible: [
                ScalarInputFieldKind::Density.into(),
                VectorOutputFieldKind::Force.into(),
            ]
            .map(|kind| (kind, true))
            .into_iter()
            .collect(),
        }
    }
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame();

        #[cfg(all(feature = "profile", not(debug_assertions)))]
        Window::new("Profiler").collapsible(true).show(ctx, |ui| {
            puffin_egui::profiler_ui(ui);
        });

        self.ui(ctx);
    }
}

const BIG_PLOT_SIZE: f32 = 200.0;
const SMALL_PLOT_SIZE: f32 = 100.0;

impl Game {
    fn ui(&mut self, ctx: &Context) {
        puffin::profile_function!();
        // Calculate fields
        let mut stack = Stack::default();
        let mut error = None;
        // Calculate spell field
        for word in self.world.player.words.clone() {
            if let Err(e) = stack.call(&mut self.world, word) {
                error = Some(e);
                break;
            }
        }
        self.world.spell_field = stack.top().map(|item| item.field.clone());

        CentralPanel::default().show(ctx, |ui| {
            self.top_ui(ui);
            self.fields_ui(ui);
            if let Some(e) = error {
                ui.label(RichText::new(e.to_string()).color(Color32::RED));
            }
        });
        let mut panel_color = ctx.style().visuals.window_fill();
        panel_color =
            Color32::from_rgba_unmultiplied(panel_color.r(), panel_color.g(), panel_color.b(), 128);
        TopBottomPanel::bottom("words")
            .frame(Frame {
                inner_margin: Margin::same(20.0),
                fill: panel_color,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.words_ui(ui, &stack);
                    self.controls_ui(ui, &stack);
                });
            });
        TopBottomPanel::bottom("stack")
            .frame(Frame {
                inner_margin: Margin {
                    left: 20.0,
                    right: 20.0,
                    top: 20.0,
                    bottom: 0.0,
                },
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.stack_ui(ui, &stack);
            });

        // Update world
        while self.ticker >= TICK_RATE {
            self.world.update();
            self.ticker -= TICK_RATE;
        }

        ctx.request_repaint();
    }
    fn top_ui(&mut self, ui: &mut Ui) {
        // Fps
        let now = Instant::now();
        let dt = (now - self.last_time).as_secs_f32();
        self.ticker += dt;
        self.last_time = now;
        ui.small(format!("{} fps", (1.0 / dt).round()));
        // Mana bar
        ui.scope(|ui| {
            let player = &self.world.player;
            let (curr, max, color) = if player.can_cast() {
                (
                    player.mana,
                    player.capped_mana(),
                    Rgba::from_rgb(0.1, 0.1, 0.9).into(),
                )
            } else {
                (
                    player.mana_exhaustion,
                    MAX_MANA_EXHAUSTION,
                    Color32::LIGHT_RED,
                )
            };
            ui.visuals_mut().selection.bg_fill = color;
            ui.horizontal(|ui| {
                ProgressBar::new(curr / max)
                    .text(format!("{} / {}", curr.round(), max.round()))
                    .desired_width(player.capped_mana() * 10.0)
                    .ui(ui);
                if player.reserved_mana() > 0.0 {
                    ui.visuals_mut().selection.bg_fill = Rgba::from_rgb(0.2, 0.2, 0.9).into();
                    ProgressBar::new(1.0)
                        .text(player.reserved_mana().to_string())
                        .desired_width(player.reserved_mana() * 10.0)
                        .ui(ui);
                }
            });
        });
    }
    fn fields_ui(&mut self, ui: &mut Ui) {
        Grid::new("fields").show(ui, |ui| {
            // Draw toggler buttons
            for kind in all::<GenericInputFieldKind>() {
                let kind = GenericFieldKind::from(kind);
                let enabled = &mut self.ui_state.fields_visible.entry(kind).or_insert(false);
                ui.toggle_value(enabled, kind.to_string());
            }
            for output_kind in all::<GenericOutputFieldKind>() {
                let kind = GenericFieldKind::from(output_kind);
                let enabled = self.ui_state.fields_visible.entry(kind).or_insert(false);
                ui.toggle_value(enabled, kind.to_string());
                if *enabled {
                    if self.world.outputs.contains(output_kind) {
                        if ui.button("Dispel").clicked() {
                            self.world.outputs.remove(output_kind);
                        }
                    } else {
                        ui.label("");
                    }
                }
            }
            ui.end_row();
            // Draw the fields themselves
            for kind in all::<GenericInputFieldKind>() {
                let kind = GenericFieldKind::from(kind);
                if self.ui_state.fields_visible[&kind] {
                    self.plot_field_kind(ui, BIG_PLOT_SIZE, 100, kind);
                } else {
                    ui.label("");
                }
            }
            for output_kind in all::<GenericOutputFieldKind>() {
                let kind = GenericFieldKind::from(output_kind);
                if self.ui_state.fields_visible[&kind] {
                    if let Some(words) = self.world.outputs.spell(output_kind) {
                        self.plot_field_kind(ui, BIG_PLOT_SIZE, 100, kind);
                        Self::spell_words_ui(ui, words, BIG_PLOT_SIZE);
                    } else {
                        ui.label("");
                    }
                } else {
                    ui.label("");
                }
            }
        });
    }
    fn spell_words_ui(ui: &mut Ui, words: &[Word], max_height: f32) {
        let font_id = &ui.style().text_styles[&TextStyle::Body];
        let row_height = ui.fonts().row_height(font_id);
        let vert_spacing = ui.spacing().item_spacing.y;
        let per_column = ((max_height / (row_height + vert_spacing)) as usize).max(1);
        for chunk in words.chunks(per_column) {
            ui.vertical(|ui| {
                ui.add_space(
                    (max_height
                        - chunk.len() as f32 * row_height
                        - per_column.saturating_sub(1) as f32 * vert_spacing)
                        / 2.0,
                );
                for word in chunk {
                    ui.label(word.to_string());
                }
            });
        }
    }
    fn stack_ui(&mut self, ui: &mut Ui, stack: &Stack) {
        ui.horizontal_wrapped(|ui| {
            ui.allocate_exact_size(vec2(0.0, SMALL_PLOT_SIZE), Sense::hover());
            for item in stack.iter() {
                self.plot_generic_field(ui, SMALL_PLOT_SIZE, 50, &item.field);
                Self::spell_words_ui(ui, &item.words, SMALL_PLOT_SIZE);
            }
        });
    }
    fn words_ui(&mut self, ui: &mut Ui, stack: &Stack) {
        Grid::new("words").show(ui, |ui| {
            // Commands
            ui.horizontal_wrapped(|ui| {
                for command in all::<SpellCommand>() {
                    if ui.button(command.to_string()).clicked() {
                        match command {
                            SpellCommand::Clear => self.world.player.words.clear(),
                        }
                    }
                }
            });
            ui.end_row();
            // Words
            fn button<W: Copy + Into<Word> + ToString + Sequence>(
                ui: &mut Ui,
                stack: &Stack,
                hilight: bool,
            ) -> Option<Word> {
                let mut res = None;
                for w in all::<W>() {
                    let name = w.to_string();
                    let word = w.into();
                    let f = word.function();
                    let enabled = stack.validate_function_use(f).is_ok();
                    if ui
                        .add_enabled(enabled, SelectableLabel::new(hilight, name))
                        .on_hover_text(f.to_string())
                        .clicked()
                    {
                        res = Some(word);
                    }
                }
                res
            }
            let spell = &mut self.world.player.words;
            spell.extend(button::<ScalarWord>(ui, stack, false));
            ui.end_row();
            spell.extend(button::<VectorWord>(ui, stack, false));
            spell.extend(button::<InputWord>(ui, stack, false));
            spell.extend(button::<ControlWord>(ui, stack, false));
            ui.end_row();
            spell.extend(button::<OperatorWord>(ui, stack, false));
            spell.extend(button::<AxisWord>(ui, stack, false));
            ui.end_row();
            spell.extend(button::<OutputWord>(ui, stack, true));
            spell.extend(button::<CombinatorWord>(ui, stack, false));
            ui.end_row();
        });
    }
    fn controls_ui(&mut self, ui: &mut Ui, stack: &Stack) {
        // Controls
        let stack_controls = stack.iter().flat_map(|item| item.field.controls());
        let outputs = &mut self.world.outputs;
        let scalar_output_controls = outputs
            .scalars
            .values()
            .flat_map(|output| output.field.controls());
        let vector_output_controls = outputs
            .vectors
            .values()
            .flat_map(|output| output.field.controls());
        let used_controls: BTreeSet<ControlKind> = stack_controls
            .chain(scalar_output_controls)
            .chain(vector_output_controls)
            .collect();
        // Vertical slider
        if used_controls.contains(&ControlKind::YSlider) {
            let value = self.world.controls.y_slider.get_or_insert(0.0);
            if ui.memory().focus().is_none() {
                if let Some(i) = [
                    Key::Num0,
                    Key::Num1,
                    Key::Num2,
                    Key::Num3,
                    Key::Num4,
                    Key::Num5,
                    Key::Num6,
                    Key::Num7,
                    Key::Num8,
                    Key::Num9,
                ]
                .into_iter()
                .position(|key| ui.input().key_pressed(key))
                {
                    *value = i as f32 / 9.0;
                }
            }
            Slider::new(value, 0.0..=1.0)
                .vertical()
                .fixed_decimals(1)
                .show_value(false)
                .ui(ui);
        } else {
            self.world.controls.y_slider = None;
        }
        // Horizontal slider
        if used_controls.contains(&ControlKind::XSlider) {
            let value = self.world.controls.x_slider.get_or_insert(0.0);
            let input = ui.input();
            if input.key_down(Key::D) || input.key_down(Key::A) {
                if ui.memory().focus().is_none() {
                    *value =
                        input.key_down(Key::D) as u8 as f32 - input.key_down(Key::A) as u8 as f32;
                }
            } else if input.key_released(Key::D) || input.key_released(Key::A) {
                *value = 0.0;
            }
            drop(input);
            Slider::new(value, -1.0..=1.0)
                .fixed_decimals(1)
                .show_value(false)
                .ui(ui);
        } else {
            self.world.controls.x_slider = None;
        }
    }
    fn init_plot(&self, size: f32, resolution: usize) -> MapPlot {
        MapPlot::new(&self.world, self.world.player_pos + Vec2::Y, 5.0)
            .size(size)
            .resolution(resolution)
    }
    pub fn plot_generic_field(
        &self,
        ui: &mut Ui,
        size: f32,
        resolution: usize,
        field: &GenericField,
    ) {
        let plot = self.init_plot(size, resolution);
        match field {
            GenericField::Scalar(ScalarField::Uniform(n)) => {
                MapPlot::number_ui(&self.world, ui, size, resolution, *n)
            }
            GenericField::Scalar(field) => plot.ui(ui, field),
            GenericField::Vector(field) => plot.ui(ui, field),
        }
    }
    pub fn plot_field_kind(
        &self,
        ui: &mut Ui,
        size: f32,
        resolution: usize,
        kind: GenericFieldKind,
    ) {
        let plot = self.init_plot(size, resolution);
        match kind {
            GenericFieldKind::Scalar(kind) => plot.ui(ui, &kind),
            GenericFieldKind::Vector(kind) => plot.ui(ui, &kind),
        }
    }
}

impl FieldPlot for ScalarField {
    type Value = f32;
    fn precision(&self) -> f32 {
        1.0
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        self.sample(world, pos)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        let h = 0.9 * (1.0 - t);
        let v = (2.0 * t - 1.0).abs();
        let s = v.powf(0.5) * 0.8;
        Hsva::new(h, s, v, 1.0).into()
    }
}

impl FieldPlot for VectorField {
    type Value = Vec2;
    fn precision(&self) -> f32 {
        0.35
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        self.sample(world, pos)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        default_vector_color(t)
    }
}

impl FieldPlot for GenericScalarFieldKind {
    type Value = f32;
    fn precision(&self) -> f32 {
        match self {
            GenericScalarFieldKind::Input(ScalarInputFieldKind::Elevation) => 0.7,
            _ => 1.0,
        }
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        world.sample_scalar_field(*self, pos)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        default_scalar_color(t)
    }
}

impl FieldPlot for GenericVectorFieldKind {
    type Value = Vec2;
    fn precision(&self) -> f32 {
        0.35
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        world.sample_vector_field(*self, pos)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        default_vector_color(t)
    }
}
