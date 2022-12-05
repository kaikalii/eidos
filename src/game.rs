use eframe::{egui::*, epaint::ahash::HashMap};
use enum_iterator::all;

use crate::{
    field::*,
    function::Function,
    math::polygon_contains,
    plot::{FieldPlot, MapPlot},
    runtime::Runtime,
    world::World,
};

#[derive(Default)]
pub struct Game {
    world: World,
    ui_state: UiState,
    spell: SpellState<Vec<Function>>,
}

#[derive(Default)]
struct UiState {
    scalar_fields_visible: HashMap<ScalarFieldKind, bool>,
}

#[derive(Clone, Copy)]
pub struct FieldsSource<'a> {
    pub world: &'a World,
    pub spell: &'a SpellState<GenericField<'a>>,
}

#[derive(Clone, Copy, Default)]
pub struct SpellState<T> {
    pub holographic: T,
    pub staging: T,
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.ui(ui));
        ctx.request_repaint();
    }
}

impl Game {
    fn ui(&mut self, ui: &mut Ui) {
        // Calculate fields
        let mut rt = Runtime::default();
        let mut error = None;
        for function in &self.spell.holographic {
            if let Err(e) = rt.call(*function) {
                error = Some(e);
                break;
            }
        }
        let holographic = rt
            .top_field()
            .cloned()
            .unwrap_or_else(|| ScalarField::Common(CommonField::Uniform(0.0)).into());
        if error.is_none() {
            for function in &self.spell.staging {
                if let Err(e) = rt.call(*function) {
                    error = Some(e);
                }
            }
        }
        let staging = error
            .is_none()
            .then(|| rt.top_field().cloned())
            .flatten()
            .unwrap_or_else(|| ScalarField::Common(CommonField::Uniform(0.0)).into());
        let spell_fields = SpellState {
            holographic,
            staging,
        };
        let source = FieldsSource {
            world: &self.world,
            spell: &spell_fields,
        };
        // Draw ui
        Grid::new("fields").show(ui, |ui| {
            for field_kind in all::<ScalarFieldKind>() {
                ui.toggle_value(
                    self.ui_state
                        .scalar_fields_visible
                        .entry(field_kind)
                        .or_insert(false),
                    field_kind.to_string(),
                );
            }
            ui.end_row();
            for (field_kind, enabled) in &self.ui_state.scalar_fields_visible {
                if *enabled {
                    source.plot_scalar_field(ui, *field_kind);
                } else {
                    ui.label("");
                }
            }
        });
    }
}

impl<'a> FieldsSource<'a> {
    pub fn plot_scalar_field(&self, ui: &mut Ui, kind: ScalarFieldKind) {
        MapPlot::new(Vec2::ZERO, 10.0).ui(
            ui,
            ScalarWorldField {
                kind,
                source: *self,
            },
        );
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
            ScalarFieldKind::Holographic => self.spell.holographic.sample(x, y),
            ScalarFieldKind::Staging => self.spell.staging.sample(x, y),
        }
    }
}

impl<'a> FieldPlot for ScalarField<'a> {
    type Key = ();
    fn key(&self) -> Self::Key {}
    fn get_z(&self, x: f32, y: f32) -> f32 {
        self.sample(x, y)
    }
}

impl<'a> FieldPlot for ScalarWorldField<'a> {
    type Key = ScalarFieldKind;
    fn key(&self) -> Self::Key {
        self.kind
    }
    fn get_z(&self, x: f32, y: f32) -> f32 {
        self.source.sample_scalar_field(self.kind, x, y)
    }
}
