use std::{
    collections::{BTreeSet, HashSet},
    time::Instant,
};

use eframe::{
    egui::{style::Margin, *},
    epaint::{ahash::HashMap, Hsva},
};
use enum_iterator::{all, Sequence};
use itertools::Itertools;

use crate::{
    controls::{apply_color_fading, FadeButton},
    dialog::{DialogCommand, DialogFragment, Line, DIALOG_SCENES},
    field::*,
    player::MAX_MANA_EXHAUSTION,
    plot::{default_scalar_color, default_vector_color, FieldPlot, MapPlot},
    stack::Stack,
    word::SpellCommand,
    word::*,
    world::World,
    GameState,
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
        let mut game = Game {
            world: World::default(),
            ui_state: UiState::default(),
            last_time: Instant::now(),
            ticker: 0.0,
        };
        game.set_dialog("intro");
        game
    }
}

struct UiState {
    fields_visible: HashMap<GenericFieldKind, bool>,
    dialog: Option<DialogState>,
    last_stack_len: usize,
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
            dialog: None,
            last_stack_len: 0,
        }
    }
}

const BIG_PLOT_SIZE: f32 = 200.0;
const SMALL_PLOT_SIZE: f32 = 100.0;

impl Game {
    pub fn show(&mut self, ctx: &Context) -> Result<(), GameState> {
        puffin::profile_function!();
        // Calculate fields
        let mut stack = Stack::default();
        let mut error = None;
        // Calculate stack fields
        for word in self.world.player.words.clone() {
            if let Err(e) = stack.call(&mut self.world, word) {
                error = Some(e);
                break;
            }
        }

        let mut style = (*ctx.style()).clone();
        style.animation_time = 2.0;
        ctx.set_style(style);

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
            .show_separator_line(false)
            .min_height(100.0)
            .frame(Frame {
                inner_margin: Margin::symmetric(50.0, 20.0),
                fill: panel_color,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.words_ui(ui, &stack);
                    self.controls_ui(ui, &stack);
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        ui.with_layout(Layout::top_down(Align::Min), |ui| self.dialog_ui(ui))
                    });
                });
            });
        TopBottomPanel::bottom("stack")
            .show_separator_line(false)
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

        Ok(())
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
            let id = ui.make_persistent_id("mana bar");
            let length_mul = ui
                .ctx()
                .animate_bool(id, self.world.player.progression.mana_bar);
            if length_mul > 0.0 {
                ui.horizontal(|ui| {
                    ProgressBar::new(curr / max)
                        .text(format!("{} / {}", curr.round(), max.round()))
                        .desired_width(player.capped_mana() * 10.0 * length_mul)
                        .ui(ui);
                    if player.reserved_mana() > 0.0 {
                        ui.visuals_mut().selection.bg_fill = Rgba::from_rgb(0.2, 0.2, 0.9).into();
                        ProgressBar::new(1.0)
                            .text(player.reserved_mana().to_string())
                            .desired_width(player.reserved_mana() * 10.0 * length_mul)
                            .ui(ui);
                    }
                });
            }
        });
    }
    fn fields_ui(&mut self, ui: &mut Ui) {
        Grid::new("fields").show(ui, |ui| {
            let known_fields = &self.world.player.progression.known_fields;
            // Draw toggler buttons
            for kind in all::<GenericInputFieldKind>() {
                if !known_fields.contains(&kind) {
                    continue;
                }
                let kind = GenericFieldKind::from(kind);
                let enabled = &mut self.ui_state.fields_visible.entry(kind).or_insert(false);
                ui.toggle_value(enabled, kind.to_string());
            }
            for output_kind in all::<GenericOutputFieldKind>() {
                if self.world.outputs.contains(output_kind) {
                    let kind = GenericFieldKind::from(output_kind);
                    let enabled = self.ui_state.fields_visible.entry(kind).or_insert(false);
                    ui.toggle_value(enabled, kind.to_string());
                    if *enabled {
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
                let known = known_fields.contains(&kind);
                let kind = GenericFieldKind::from(kind);
                let id = ui.make_persistent_id(kind);
                let alpha = ui.ctx().animate_bool(id, known);
                if !known {
                    continue;
                }
                if self.ui_state.fields_visible[&kind] {
                    self.plot_field_kind(ui, BIG_PLOT_SIZE, 100, alpha, kind);
                } else {
                    ui.label("");
                }
            }
            for output_kind in all::<GenericOutputFieldKind>() {
                if self.world.outputs.contains(output_kind) {
                    let kind = GenericFieldKind::from(output_kind);
                    if self.ui_state.fields_visible[&kind] {
                        if let Some(words) = self.world.outputs.spell(output_kind) {
                            self.plot_field_kind(ui, BIG_PLOT_SIZE, 100, 1.0, kind);
                            Self::spell_words_ui(ui, words, BIG_PLOT_SIZE);
                        } else {
                            ui.label("");
                        }
                    } else {
                        ui.label("");
                    }
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
        ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.allocate_exact_size(vec2(0.0, SMALL_PLOT_SIZE), Sense::hover());
                for item in stack.iter() {
                    self.plot_generic_field(ui, SMALL_PLOT_SIZE, 50, 1.0, &item.field);
                    Self::spell_words_ui(ui, &item.words, SMALL_PLOT_SIZE);
                }
                if self.ui_state.last_stack_len != stack.len() {
                    ui.scroll_to_cursor(None);
                    self.ui_state.last_stack_len = stack.len();
                }
            });
        });
    }
    fn words_ui(&mut self, ui: &mut Ui, stack: &Stack) {
        Grid::new("words").show(ui, |ui| {
            // Commands
            ui.horizontal_wrapped(|ui| {
                let show_commands = !self.world.player.progression.known_words.is_empty();
                let id = ui.make_persistent_id("commands");
                let visibility = ui.ctx().animate_bool(id, show_commands);
                if show_commands {
                    apply_color_fading(ui.visuals_mut(), visibility);
                    for command in all::<SpellCommand>() {
                        if ui.button(command.to_string()).clicked() {
                            match command {
                                SpellCommand::Clear => self.world.player.words.clear(),
                            }
                        }
                    }
                }
            });
            ui.end_row();
            // Words
            fn button<W: Copy + Into<Word> + ToString + Sequence>(
                ui: &mut Ui,
                stack: &Stack,
                know_words: &HashSet<Word>,
                hilight: bool,
            ) -> Option<Word> {
                let mut res = None;
                for w in all::<W>() {
                    let name = w.to_string();
                    let word = w.into();
                    let f = word.function();
                    let known = know_words.contains(&word);
                    let enabled = known && stack.validate_function_use(f).is_ok();
                    if ui
                        .add_enabled(enabled, FadeButton::new(word, known, name).hilight(hilight))
                        .on_hover_text(f.to_string())
                        .clicked()
                    {
                        res = Some(word);
                    }
                }
                res
            }
            let words = &mut self.world.player.words;
            let known_words = &self.world.player.progression.known_words;
            words.extend(button::<ScalarWord>(ui, stack, known_words, false));
            ui.end_row();
            words.extend(button::<VectorWord>(ui, stack, known_words, false));
            words.extend(button::<InputWord>(ui, stack, known_words, false));
            words.extend(button::<ControlWord>(ui, stack, known_words, false));
            ui.end_row();
            words.extend(button::<OperatorWord>(ui, stack, known_words, false));
            words.extend(button::<AxisWord>(ui, stack, known_words, false));
            ui.end_row();
            words.extend(button::<OutputWord>(ui, stack, known_words, true));
            words.extend(button::<CombinatorWord>(ui, stack, known_words, false));
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
            let something_focused = ui.memory().focus().is_some();
            let input = ui.input();
            if input.key_down(Key::D) || input.key_down(Key::A) {
                if !something_focused {
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
    fn init_plot(&self, size: f32, resolution: usize, global_alpha: f32) -> MapPlot {
        MapPlot::new(
            &self.world,
            self.world.player_pos + Vec2::Y,
            5.0,
            global_alpha,
        )
        .size(size)
        .resolution(resolution)
    }
    pub fn plot_generic_field(
        &self,
        ui: &mut Ui,
        size: f32,
        resolution: usize,
        global_alpha: f32,
        field: &GenericField,
    ) {
        let plot = self.init_plot(size, resolution, global_alpha);
        match field {
            GenericField::Scalar(ScalarField::Uniform(n)) => {
                MapPlot::number_ui(&self.world, ui, size, resolution, global_alpha, *n)
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
        global_alpha: f32,
        kind: GenericFieldKind,
    ) {
        let plot = self.init_plot(size, resolution, global_alpha);
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

struct DialogState {
    scene: String,
    node: String,
    line: usize,
    character: usize,
    speaker: Option<String>,
}

impl Game {
    fn set_dialog(&mut self, scene_name: &str) {
        let scene = &DIALOG_SCENES[scene_name];
        let dialog = DialogState {
            scene: scene_name.into(),
            node: scene.nodes.first().unwrap().0.clone(),
            line: 0,
            character: 0,
            speaker: None,
        };
        self.ui_state.dialog = Some(dialog);
    }
    fn dialog_ui(&mut self, ui: &mut Ui) {
        if self.ui_state.dialog.is_none() {
            return;
        }
        ui.group(|ui| self.dialog_ui_impl(ui));
    }
    fn progress_dialog(&mut self) {
        let Some(dialog) = &mut self.ui_state.dialog else {
            return;
        };
        let scene = &DIALOG_SCENES[&dialog.scene];
        let node = &scene.nodes[&dialog.node];

        if dialog.line < node.lines.len().saturating_sub(1) {
            dialog.line += 1;
            dialog.character = 0;
        } else {
            let node_index = scene.nodes.get_index_of(&dialog.node).unwrap();
            if let Some((node_name, _)) = scene.nodes.get_index(node_index + 1) {
                dialog.node = node_name.clone();
                dialog.line = 0;
                dialog.character = 0;
            } else if node.children.is_empty() {
                self.ui_state.dialog = None;
            }
        }
    }
    fn dialog_ui_impl(&mut self, ui: &mut Ui) {
        // Get dialog scene data
        let Some(dialog) = &mut self.ui_state.dialog else {
            return;
        };
        let scene = &DIALOG_SCENES[&dialog.scene];
        let node = &scene.nodes[&dialog.node];
        if node.lines.is_empty() {
            self.progress_dialog();
            self.dialog_ui_impl(ui);
            return;
        }
        let line = &node.lines[dialog.line];
        match line {
            Line::Text(fragments) => {
                // Space the group
                ui.allocate_at_least(vec2(ui.max_rect().width(), 0.0), Sense::hover());
                let line_text = self.world.format_dialog_fragments(fragments);
                let char_indices = line_text.char_indices().collect_vec();
                const DIALOG_SPEED: usize = 2;
                let char_index = dialog.character / DIALOG_SPEED;
                ui.horizontal(|ui| {
                    // Show speaker
                    if let Some(speaker) = &dialog.speaker {
                        ui.label(format!("{speaker}:"));
                    }
                    // Show line text
                    if !line_text.is_empty() {
                        let line_text = &line_text[..=char_indices[char_index].0];
                        ui.horizontal_wrapped(|ui| ui.label(line_text));
                    }
                });
                // Show continue or choices
                let max_dialog_char = (char_indices.len().saturating_sub(1)) * DIALOG_SPEED;
                dialog.character = (dialog.character + 1).min(max_dialog_char);
                let mut next = || {
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| ui.button(">").clicked())
                        .inner
                };
                if dialog.character < max_dialog_char {
                    // Revealing the text
                    if next() {
                        dialog.character = max_dialog_char;
                    }
                } else if node.children.is_empty() {
                    // No choices
                    if line_text.is_empty() || next() {
                        self.progress_dialog();
                    }
                } else {
                    // Choices
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        for (name, fragments) in &node.children {
                            if ui
                                .button(self.world.format_dialog_fragments(fragments))
                                .clicked()
                            {
                                dialog.node = name.clone();
                                dialog.line = 0;
                                dialog.character = 0;
                            }
                        }
                    });
                }
            }
            Line::Command(command) => {
                let progression = &mut self.world.player.progression;
                match command {
                    DialogCommand::Speaker(speaker) => dialog.speaker = speaker.clone(),
                    DialogCommand::RevealWord(word) => {
                        progression.known_words.insert(*word);
                    }
                    DialogCommand::RevealAllWords => progression.known_words.extend(all::<Word>()),
                    DialogCommand::RevealManaBar => progression.mana_bar = true,
                    DialogCommand::RevealField(kind) => {
                        progression.known_fields.insert(*kind);
                    }
                }
                self.progress_dialog();
                self.dialog_ui_impl(ui);
            }
        }
    }
}

impl World {
    fn format_dialog_fragments(&self, fragments: &[DialogFragment]) -> String {
        let mut formatted = String::new();
        for frag in fragments {
            match frag {
                DialogFragment::String(s) => formatted.push_str(s),
                DialogFragment::Variable(_var) => todo!(),
            }
        }
        formatted
    }
}
