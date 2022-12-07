use std::time::Instant;

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
        // Calculate fields
        let mut stack = Stack::default();
        let mut error = None;
        // Calculate spell field
        for word in self.world.player.spell.clone() {
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
                self.words_ui(ui, &stack);
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
}

const BIG_PLOT_SIZE: f32 = 200.0;
const SMALL_PLOT_SIZE: f32 = 100.0;

impl Game {
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
                (player.mana, player.max_mana, Color32::BLUE)
            } else {
                (
                    player.mana_exhaustion,
                    MAX_MANA_EXHAUSTION,
                    Color32::LIGHT_RED,
                )
            };
            ui.visuals_mut().selection.bg_fill = color;
            ProgressBar::new(curr / max)
                .text(format!("{} / {}", curr.round(), max.round()))
                .desired_width(300.0)
                .ui(ui);
        });
    }
    fn fields_ui(&mut self, ui: &mut Ui) {
        Grid::new("fields").show(ui, |ui| {
            // Draw fields
            for field_kind in all::<GenericFieldKind>() {
                ui.toggle_value(
                    self.ui_state
                        .fields_visible
                        .entry(field_kind)
                        .or_insert(false),
                    field_kind.to_string(),
                );
            }
            ui.end_row();
            for field_kind in all::<GenericFieldKind>() {
                if self.ui_state.fields_visible[&field_kind] {
                    self.plot_field_kind(ui, BIG_PLOT_SIZE, 100, field_kind);
                } else {
                    ui.label("");
                }
            }
        });
    }
    fn stack_ui(&mut self, ui: &mut Ui, stack: &Stack) {
        ui.horizontal_wrapped(|ui| {
            ui.allocate_exact_size(vec2(0.0, SMALL_PLOT_SIZE), Sense::hover());
            for item in stack.iter() {
                self.plot_generic_field(ui, SMALL_PLOT_SIZE, 50, &item.field);
                for chunk in item.words.chunks(5) {
                    ui.vertical(|ui| {
                        ui.add_space((SMALL_PLOT_SIZE - chunk.len() as f32 * 15.0) / 2.0);
                        for word in chunk {
                            ui.label(word.to_string());
                        }
                    });
                }
            }
        });
    }
    fn words_ui(&mut self, ui: &mut Ui, stack: &Stack) {
        ui.horizontal_wrapped(|ui| {
            for command in all::<SpellCommand>() {
                if ui.button(command.to_string()).clicked() {
                    match command {
                        SpellCommand::Clear => self.world.player.spell.clear(),
                    }
                }
            }
        });
        Grid::new("words").show(ui, |ui| {
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
            let spell = &mut self.world.player.spell;
            spell.extend(button::<ScalarWord>(ui, stack, false));
            ui.end_row();
            spell.extend(button::<VectorWord>(ui, stack, false));
            spell.extend(button::<InputWord>(ui, stack, false));
            ui.end_row();
            spell.extend(button::<OperatorWord>(ui, stack, false));
            spell.extend(button::<AxisWord>(ui, stack, false));
            ui.end_row();
            spell.extend(button::<OutputWord>(ui, stack, true));
            spell.extend(button::<CombinatorWord>(ui, stack, false));
            ui.end_row();
        });
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
            GenericField::Scalar(ScalarField::Common(CommonField::Uniform(n))) => {
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
