use std::time::Instant;

use eframe::{
    egui::*,
    epaint::{ahash::HashMap, color::Hsva},
};
use enum_iterator::all;

use crate::{
    field::*,
    function::{Function, FunctionCategory},
    math::polygon_contains,
    plot::{default_scalar_color, default_vector_color, FieldPlot, MapPlot},
    runtime::Runtime,
    word::SpellCommand,
    world::World,
};

pub struct Game {
    world: World,
    ui_state: UiState,
    spell: SpellState<Vec<Function>>,
    last_time: Instant,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            world: World::default(),
            ui_state: UiState::default(),
            spell: SpellState::default(),
            last_time: Instant::now(),
        }
    }
}

struct UiState {
    fields_visible: HashMap<FieldKind<GenericFieldKind>, bool>,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            fields_visible: [
                FieldKind::Typed(GenericFieldKind::Scalar(ScalarFieldKind::Density)),
                FieldKind::Any(AnyFieldKind::Spell),
                FieldKind::Any(AnyFieldKind::Staging),
            ]
            .map(|kind| (kind, true))
            .into_iter()
            .collect(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct FieldsSource<'a, S = &'a SpellState<GenericField<'a>>> {
    pub world: &'a World,
    pub spell_state: S,
}

#[derive(Clone, Copy, Default)]
pub struct SpellState<T> {
    pub spell: T,
    pub staging: T,
}

impl SpellState<Vec<Function>> {
    pub fn command(&mut self, command: SpellCommand) {
        match command {
            SpellCommand::Commit => self.spell.append(&mut self.staging),
            SpellCommand::Disapate => self.staging.clear(),
            SpellCommand::Clear => {
                self.spell.clear();
                self.staging.clear();
            }
        }
    }
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.ui(ui));
        ctx.request_repaint();
    }
}

impl Game {
    fn ui(&mut self, ui: &mut Ui) {
        // Fps
        let now = Instant::now();
        let elapsed = (now - self.last_time).as_secs_f32();
        self.last_time = now;
        ui.small(format!("{} fps", (1.0 / elapsed).round()));
        // Calculate fields
        let mut rt = Runtime::default();
        let mut error = None;
        let source = FieldsSource {
            world: &self.world,
            spell_state: (),
        };
        // Calculate spell field
        for function in &self.spell.spell {
            if let Err(e) = rt.call(source, *function) {
                error = Some(e);
                break;
            }
        }
        let spell = rt
            .top_field()
            .cloned()
            .unwrap_or_else(|| ScalarField::Common(CommonField::Uniform(0.0)).into());
        // Calculate staging field
        if error.is_none() {
            for function in &self.spell.staging {
                if let Err(e) = rt.call(source, *function) {
                    error = Some(e);
                }
            }
        }
        let staging = error
            .is_none()
            .then(|| rt.top_field().cloned())
            .flatten()
            .unwrap_or_else(|| ScalarField::Common(CommonField::Uniform(0.0)).into());
        // Build fields source
        let spell_fields = SpellState { spell, staging };
        let source = FieldsSource {
            world: &self.world,
            spell_state: &spell_fields,
        };
        // Draw ui
        Grid::new("fields").show(ui, |ui| {
            // Draw fields
            for field_kind in all::<FieldKind<GenericFieldKind>>() {
                ui.toggle_value(
                    self.ui_state
                        .fields_visible
                        .entry(field_kind)
                        .or_insert(false),
                    field_kind.to_string(),
                );
            }
            ui.end_row();
            for field_kind in all::<FieldKind<GenericFieldKind>>() {
                if self.ui_state.fields_visible[&field_kind] {
                    source.plot_field(ui, field_kind);
                } else {
                    ui.label("");
                }
            }
        });
        // Draw functions
        ui.horizontal_wrapped(|ui| {
            ui.label("");
            for function in &self.spell.staging {
                ui.label(function.to_string());
            }
        });
        // Draw word buttons
        ui.horizontal_wrapped(|ui| {
            for command in all::<SpellCommand>() {
                if ui.button(command.to_string()).clicked() {
                    self.spell.command(command);
                }
            }
        });
        for category in all::<FunctionCategory>() {
            ui.horizontal_wrapped(|ui| {
                for function in category.functions() {
                    let enabled = error.is_none() && rt.validate_function_use(function).is_ok();
                    if ui
                        .add_enabled(enabled, Button::new(function.to_string()))
                        .clicked()
                    {
                        self.spell.staging.push(function);
                    }
                }
            });
        }
    }
}

impl<'a> FieldsSource<'a> {
    pub fn plot_field(&self, ui: &mut Ui, kind: FieldKind<GenericFieldKind>) {
        let plot = MapPlot::new(Vec2::ZERO, 10.0);
        match kind {
            FieldKind::Any(kind) => {
                let field = match kind {
                    AnyFieldKind::Spell => &self.spell_state.spell,
                    AnyFieldKind::Staging => &self.spell_state.staging,
                };
                match field {
                    GenericField::Scalar(ScalarField::Common(CommonField::Uniform(n))) => {
                        MapPlot::number_ui(ui, *n, FieldKind::Any(kind))
                    }
                    GenericField::Scalar(field) => plot.ui(ui, (field, kind)),
                    GenericField::Vector(field) => plot.ui(ui, (field, kind)),
                }
            }
            FieldKind::Typed(GenericFieldKind::Scalar(kind)) => plot.ui(
                ui,
                ScalarWorldField {
                    kind,
                    source: self.no_spells(),
                },
            ),
            FieldKind::Typed(GenericFieldKind::Vector(kind)) => plot.ui(
                ui,
                VectorWorldField {
                    kind,
                    source: self.no_spells(),
                },
            ),
        }
    }
}
impl<'a, S> FieldsSource<'a, S> {
    pub fn no_spells(self) -> FieldsSource<'a, ()> {
        FieldsSource {
            world: self.world,
            spell_state: (),
        }
    }
    pub fn sample_scalar_field(&self, kind: ScalarFieldKind, x: f32, y: f32) -> f32 {
        match kind {
            ScalarFieldKind::Density => self
                .world
                .static_objects
                .iter()
                .find(|obj| polygon_contains(&obj.shape, vec2(x, y) + Vec2::splat(1e-5)))
                .map(|obj| obj.density)
                .unwrap_or(0.0),
        }
    }
    pub fn sample_vector_field(&self, kind: VectorFieldKind, _x: f32, _y: f32) -> Vec2 {
        match kind {}
    }
}

impl<'a> FieldPlot for (&'a ScalarField<'a>, AnyFieldKind) {
    type Value = f32;
    fn key(&self) -> FieldKind<GenericFieldKind> {
        FieldKind::Any(self.1)
    }
    fn get_z(&self, x: f32, y: f32) -> Self::Value {
        self.0.sample(x, y)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        let h = if t > 0.5 { 0.5 } else { 0.0 };
        let v = 0.7 * (2.0 * t - 1.0).abs() + 0.3;
        let s = (2.0 * t - 1.0).abs()
            * match self.1 {
                AnyFieldKind::Spell => 1.0,
                AnyFieldKind::Staging => 0.7,
            };
        Hsva::new(h, s, v, 1.0).into()
    }
}

impl<'a> FieldPlot for (&'a VectorField<'a>, AnyFieldKind) {
    type Value = Vec2;
    fn key(&self) -> FieldKind<GenericFieldKind> {
        FieldKind::Any(self.1)
    }
    fn get_z(&self, x: f32, y: f32) -> Self::Value {
        self.0.sample(x, y)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        default_vector_color(t)
    }
}

impl<'a> FieldPlot for ScalarWorldField<'a> {
    type Value = f32;
    fn key(&self) -> FieldKind<GenericFieldKind> {
        FieldKind::Typed(GenericFieldKind::Scalar(self.kind))
    }
    fn get_z(&self, x: f32, y: f32) -> Self::Value {
        self.source.sample_scalar_field(self.kind, x, y)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        default_scalar_color(t)
    }
}

impl<'a> FieldPlot for VectorWorldField<'a> {
    type Value = Vec2;
    fn key(&self) -> FieldKind<GenericFieldKind> {
        FieldKind::Typed(GenericFieldKind::Vector(self.kind))
    }
    fn get_z(&self, x: f32, y: f32) -> Self::Value {
        self.source.sample_vector_field(self.kind, x, y)
    }
    fn get_color(&self, t: Self::Value) -> Color32 {
        default_vector_color(t)
    }
}
