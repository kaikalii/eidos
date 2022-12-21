use std::{
    collections::{BTreeSet, HashMap},
    time::Instant,
};

use eframe::egui::{style::Margin, *};
use enum_iterator::all;
use itertools::Itertools;

use crate::{
    controls::{apply_color_fading, FadeButton},
    dialog::DialogState,
    field::*,
    function::Function,
    image::{image_plot, ImagePlotKind},
    person::PersonId,
    player::Player,
    plot::*,
    word::*,
    world::{Controls, World, BODY_TEMP},
    GameState,
};

pub const TICK_RATE: f32 = 1.0 / 60.0;

pub struct Game {
    pub world: World,
    pub ui_state: UiState,
    last_time: Instant,
    ticker: f32,
}

impl Game {
    pub fn new(player: Player) -> Self {
        let mut game = Game {
            world: World::new(player),
            ui_state: UiState::default(),
            last_time: Instant::now(),
            ticker: 0.0,
        };
        game.set_dialog("intro");
        game
    }
}

pub struct UiState {
    pub fields_display: HashMap<FieldKind, FieldDisplay>,
    pub dialog: Option<DialogState>,
    last_stack_len: usize,
    paused: bool,
    next_player_target: Option<Pos2>,
    pub background: Option<String>,
}

pub struct FieldDisplay {
    pub visible: bool,
    pub pos: Vec2,
    pub size: f32,
}

impl FieldDisplay {
    pub fn default_for(kind: FieldKind) -> Self {
        let index = kind.index();
        let x = (index % 5) as f32 * 0.2 + 0.1;
        let y = (index / 5) as f32 * 0.35 + 0.2;
        FieldDisplay {
            visible: true,
            pos: vec2(x, y),
            size: BIG_PLOT_SIZE,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for UiState {
    fn default() -> Self {
        UiState {
            fields_display: HashMap::new(),
            dialog: None,
            last_stack_len: 0,
            paused: false,
            next_player_target: None,
            background: None,
        }
    }
}

const BIG_PLOT_SIZE: f32 = 180.0;
const SMALL_PLOT_SIZE: f32 = 100.0;

impl Game {
    pub fn show(&mut self, ctx: &Context) -> Option<GameState> {
        puffin::profile_function!();

        let mut res = None;

        // Set player target
        self.world.player.person.target = self.ui_state.next_player_target.take();

        // Set animation time
        let mut style = (*ctx.style()).clone();
        style.animation_time = 2.0;
        ctx.set_style(style.clone());

        // Show central UI
        let rect = ctx.available_rect();
        CentralPanel::default().show(ctx, |ui| {
            // Show background
            ui.allocate_ui_at_rect(rect, |ui| {
                if let Some(background) = &self.ui_state.background {
                    let max_size = ui.available_size_before_wrap();
                    image_plot(ui, background, max_size, ImagePlotKind::Background);
                }
            });
            // Show top bar and fields
            ui.allocate_ui_at_rect(rect.shrink(10.0), |ui| {
                self.top_ui(ui);
                self.fields_ui(ui);
            });
        });

        // Show pause menu
        if ctx.input().key_pressed(Key::Escape) {
            self.ui_state.paused = !self.ui_state.paused;
        }

        // Set animation time
        style.animation_time = 0.5;
        ctx.set_style(style.clone());

        SidePanel::right("pause")
            .resizable(false)
            .min_width(200.0)
            .frame(Frame {
                inner_margin: Margin::same(20.0),
                fill: style.visuals.faint_bg_color,
                ..Frame::side_top_panel(&style)
            })
            .show_animated(ctx, self.ui_state.paused, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                if ui
                    .selectable_label(false, RichText::new("Resume").heading())
                    .clicked()
                {
                    self.ui_state.paused = false;
                }
                if ui
                    .selectable_label(false, RichText::new("Main Menu").heading())
                    .clicked()
                {
                    res = Some(GameState::MainMenu);
                }
            });

        // Set animation time
        style.animation_time = 2.0;
        ctx.set_style(style);

        // Show bottom UIs
        let mut panel_color = ctx.style().visuals.panel_fill;
        panel_color =
            Color32::from_rgba_unmultiplied(panel_color.r(), panel_color.g(), panel_color.b(), 210);
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
                    self.words_ui(ui);
                    self.controls_ui(ui);
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        ui.with_layout(Layout::top_down(Align::Min), |ui| self.dialog_ui(ui))
                    });
                });
            });
        TopBottomPanel::bottom("stack")
            .show_separator_line(false)
            .frame(Frame {
                inner_margin: Margin::symmetric(20.0, 0.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                let showed_speakers_ui = self
                    .ui_state
                    .dialog
                    .as_ref()
                    .map_or(false, |dialog| dialog.speakers_ui(ui));
                if !showed_speakers_ui {
                    self.stack_ui(ui);
                }
            });

        // Update world
        while self.ticker >= TICK_RATE {
            self.world.update();
            self.ticker -= TICK_RATE;
        }

        res
    }
    fn top_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Mana bar
            ui.scope(|ui| {
                let reserved = self.world.player.person.reserved_mana();
                let capped = self.world.person(PersonId::Player).max_mana - reserved;
                let color = Rgba::from_rgb(0.1, 0.1, 0.9).into();
                ui.visuals_mut().selection.bg_fill = color;
                let id = ui.make_persistent_id("mana bar");
                let length_mul = ui
                    .ctx()
                    .animate_bool(id, self.world.player.progression.mana_bar);
                if length_mul > 0.0 {
                    ProgressBar::new(1.0)
                        .text(format!("{capped:.0}"))
                        .desired_width(capped * 10.0 * length_mul)
                        .ui(ui);
                    if reserved > 0.0 {
                        ui.visuals_mut().selection.bg_fill = Rgba::from_rgb(0.2, 0.2, 0.9).into();
                        ProgressBar::new(1.0)
                            .text(format!("{reserved:.0}"))
                            .desired_width(reserved * 10.0 * length_mul)
                            .ui(ui);
                    }
                }
            });
            // Fps
            let now = Instant::now();
            let dt = (now - self.last_time).as_secs_f32();
            if !self.ui_state.paused {
                self.ticker += dt;
            }
            self.last_time = now;
            ui.small(format!("{} fps", (1.0 / dt).round()));
        });
    }
    fn fields_ui(&mut self, ui: &mut Ui) {
        // Draw the fields themselves
        let full_rect = ui.available_rect_before_wrap();
        let mut dragged = Vec::new();
        let mut drag_released = None;
        let mut hovered = Vec::new();
        let mut double_clicked = Vec::new();
        // Input fields
        for kind in all::<InputFieldKind>() {
            let known = self.world.player.progression.known_fields.contains(&kind);
            let kind = FieldKind::from(kind);
            let id = ui.make_persistent_id(kind);
            let alpha = ui.ctx().animate_bool(id, known);
            if !known {
                continue;
            }
            let display = self
                .ui_state
                .fields_display
                .entry(kind)
                .or_insert_with(|| FieldDisplay::default_for(kind));
            if display.visible {
                let size = display.size;
                let plot_rect = Rect::from_center_size(
                    full_rect.min + display.pos * full_rect.size(),
                    Vec2::splat(size),
                );
                ui.allocate_ui_at_rect(plot_rect, |ui| {
                    let plot_resp = self.plot_io_field(ui, size, 100, alpha, kind);
                    if plot_resp
                        .response
                        .double_clicked_by(PointerButton::Secondary)
                    {
                        double_clicked.push(kind);
                    } else if plot_resp.response.dragged_by(PointerButton::Secondary) {
                        dragged.push((kind, plot_resp.response.drag_delta()));
                    } else if plot_resp.response.drag_released() {
                        drag_released = Some(kind);
                    } else if plot_resp.response.hovered() {
                        hovered.push(kind);
                    }
                    self.handle_plot_response(ui, plot_resp);
                });
            }
        }
        // Output fields
        for output_kind in all::<OutputFieldKind>() {
            let player_person = &self.world.player.person;
            if player_person.active_spells.contains(output_kind) {
                let kind = FieldKind::from(output_kind);
                let display = self
                    .ui_state
                    .fields_display
                    .entry(kind)
                    .or_insert_with(|| FieldDisplay::default_for(kind));
                if display.visible && player_person.active_spells.spell_words(output_kind).len() > 0
                {
                    let size = display.size;
                    let center = full_rect.min + display.pos * full_rect.size();
                    let plot_rect = Rect::from_min_max(
                        center - vec2(size, size) / 2.0,
                        pos2(full_rect.right(), full_rect.bottom()),
                    );
                    ui.allocate_ui_at_rect(plot_rect, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let plot_resp = self.plot_io_field(ui, size, 100, 1.0, kind);
                            let player_person = &mut self.world.player.person;
                            let words = player_person.active_spells.spell_words(output_kind);
                            let mut to_dispel = None;
                            for (i, words) in words.enumerate() {
                                if Self::spell_words_ui(ui, words, size, true) {
                                    to_dispel = Some(i);
                                }
                            }
                            if let Some(i) = to_dispel {
                                player_person.active_spells.remove(output_kind, i);
                            }
                            if plot_resp
                                .response
                                .double_clicked_by(PointerButton::Secondary)
                            {
                                double_clicked.push(kind);
                            } else if plot_resp.response.dragged_by(PointerButton::Secondary) {
                                dragged.push((kind, plot_resp.response.drag_delta()));
                            } else if plot_resp.response.drag_released() {
                                drag_released = Some(kind);
                            } else if plot_resp.response.hovered() {
                                hovered.push(kind);
                            }
                            self.handle_plot_response(ui, plot_resp);
                        });
                    });
                }
            }
        }
        // Draw toggler buttons
        let known_fields = &self.world.player.progression.known_fields;
        ui.allocate_ui_at_rect(full_rect, |ui| {
            ui.horizontal(|ui| {
                for kind in all::<InputFieldKind>() {
                    if !known_fields.contains(&kind) {
                        continue;
                    }
                    let kind = FieldKind::from(kind);
                    let enabled = &mut self
                        .ui_state
                        .fields_display
                        .entry(kind)
                        .or_insert_with(|| FieldDisplay::default_for(kind))
                        .visible;
                    ui.toggle_value(enabled, kind.to_string());
                }
                for output_kind in all::<OutputFieldKind>() {
                    if self.world.player.person.active_spells.contains(output_kind) {
                        let kind = FieldKind::from(output_kind);
                        let enabled = &mut self
                            .ui_state
                            .fields_display
                            .entry(kind)
                            .or_insert_with(|| FieldDisplay::default_for(kind))
                            .visible;
                        ui.toggle_value(enabled, kind.to_string());
                    }
                }
            });
        });
        // Handle field display dragging
        if let Some(kind) = double_clicked.pop() {
            *self.ui_state.fields_display.get_mut(&kind).unwrap() = FieldDisplay::default_for(kind);
        }
        if let Some((kind, delta)) = dragged.pop() {
            self.ui_state.fields_display.get_mut(&kind).unwrap().pos += delta / full_rect.size();
        }
        if let Some(kind) = hovered.pop() {
            let size = &mut self.ui_state.fields_display.get_mut(&kind).unwrap().size;
            *size = (*size + ui.input().scroll_delta.y / 5.0).clamp(70.0, 400.0);
        }
        if let Some(kind) = drag_released {
            let pos = &mut self.ui_state.fields_display.get_mut(&kind).unwrap().pos;
            pos.x = (pos.x * 40.0).round() / 40.0;
            pos.y = (pos.y * 20.0).round() / 20.0;
        }
    }
    fn spell_words_ui(ui: &mut Ui, words: &[Word], mut max_height: f32, can_dispel: bool) -> bool {
        let font_id = &ui.style().text_styles[&TextStyle::Body];
        let row_height = ui.fonts().row_height(font_id);
        let vert_spacing = ui.spacing().item_spacing.y;
        max_height -= row_height * 2.5;
        let mut words_per_column = ((max_height / (row_height + vert_spacing)) as usize).max(1);
        if words.len() % words_per_column == 1 {
            words_per_column -= 1;
        }
        ui.vertical(|ui| {
            ui.add_space(row_height);
            let dispelled = can_dispel && ui.button("Dispel").clicked();
            ui.horizontal(|ui| {
                for chunk in words.chunks(words_per_column) {
                    ui.vertical(|ui| {
                        ui.add_space(
                            (max_height
                                - chunk.len() as f32 * row_height
                                - words_per_column.saturating_sub(1) as f32 * vert_spacing)
                                / 2.0,
                        );
                        for word in chunk {
                            ui.label(word.to_string());
                        }
                    });
                }
            });
            dispelled
        })
        .inner
    }
    fn stack_ui(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.allocate_exact_size(vec2(0.0, SMALL_PLOT_SIZE), Sense::hover());
                for item in self.world.player.person.stack.iter().collect_vec() {
                    let plot_resp =
                        self.plot_stack_field(ui, SMALL_PLOT_SIZE, 50, 1.0, &item.field);
                    Self::handle_plot_response_impl(
                        ui,
                        &mut self.ui_state,
                        &mut self.world.controls,
                        plot_resp,
                    );
                    Self::spell_words_ui(ui, &item.words, SMALL_PLOT_SIZE, false);
                }
                let stack = &self.world.player.person.stack;
                if self.ui_state.last_stack_len != stack.len() {
                    ui.scroll_to_cursor(None);
                    self.ui_state.last_stack_len = stack.len();
                }
            });
        });
    }
    fn words_ui(&mut self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            self.words_grid(ui);
            self.conduit_ui(ui);
        });
    }
    fn conduit_ui(&mut self, ui: &mut Ui) {
        if !self.world.player.progression.conduit {
            return;
        }
        Grid::new("conduits").show(ui, |ui| {
            for stone in &mut self.world.player.person.rack.conduits {
                let mut stack = self.world.player.person.stack.clone();
                let button = Button::new(stone.format(16));
                let mut res = Ok(());
                for word in &stone.words {
                    res = stack.say(PersonId::Player, *word, None);
                    if res.is_err() {
                        break;
                    }
                }
                let on_hover = |ui: &mut Ui| {
                    ui.label(stone.format(usize::MAX));
                };
                if res.is_ok() {
                    if button.ui(ui).on_hover_ui(on_hover).clicked() {
                        self.world.player.person.stack = stack;
                    }
                } else {
                    ui.add_enabled(false, button).on_disabled_hover_ui(on_hover);
                }
                let can_add = !self.world.player.person.stack.is_empty();
                if ui.add_enabled(can_add, Button::new("+")).clicked() {
                    stone.etch(self.world.player.person.stack.words());
                    self.world.player.person.stack.clear();
                }
                ui.end_row();
            }
        });
    }
    fn words_grid(&mut self, ui: &mut Ui) {
        Grid::new("words").min_col_width(10.0).show(ui, |ui| {
            // Words
            let dialog_allows_casting = self
                .ui_state
                .dialog
                .as_ref()
                .map_or(true, |dialog| dialog.allows_casting());
            let available_mana = self.world.player.person.capped_mana();
            // Rows
            for (i, row) in WORD_GRID.iter().enumerate() {
                // Words in the row
                for word in row {
                    let player_person = &self.world.player.person;
                    let f = word.function();
                    let known = self.world.player.progression.known_words.contains(word);
                    let enabled = dialog_allows_casting
                        && known
                        && player_person.stack.validate_function_use(f).is_ok()
                        && available_mana >= word.cost();
                    let hilight = matches!(f, Function::WriteField(_));
                    let button = FadeButton::new(word, known, word.to_string()).hilight(hilight);
                    if ui.add_enabled(enabled, button).clicked() {
                        let player_person = &mut self.world.player.person;
                        let mut say = || {
                            player_person
                                .stack
                                .say(
                                    PersonId::Player,
                                    *word,
                                    Some(&mut player_person.active_spells),
                                )
                                .err()
                        };
                        let _err = if let Function::ReadField(kind) = f {
                            if self.world.player.progression.known_fields.insert(kind) {
                                // Reveal the relevant field if this is the first time its word is said
                                self.ui_state
                                    .fields_display
                                    .insert(kind.into(), FieldDisplay::default_for(kind.into()));
                                None
                            } else {
                                say()
                            }
                        } else {
                            say()
                        };
                    }
                }
                if i == 0 {
                    // Free
                    let show_free = self.world.player.progression.free;
                    let id = ui.make_persistent_id("free");
                    let visibility = ui.ctx().animate_bool(id, show_free);
                    if show_free {
                        apply_color_fading(ui.visuals_mut(), visibility);
                        if ui.button("Free").clicked() {
                            self.world.player.person.stack.clear();
                        }
                    } else {
                        ui.label("");
                    }
                }
                ui.end_row();
            }
        });
    }
    fn controls_ui(&mut self, ui: &mut Ui) {
        // Controls
        let player_person = &mut self.world.player.person;
        let stack_controls = player_person
            .stack
            .iter()
            .flat_map(|item| item.field.controls());
        let scalar_output_controls = player_person
            .active_spells
            .scalars
            .values()
            .flatten()
            .flat_map(|spell| spell.field.controls());
        let vector_output_controls = player_person
            .active_spells
            .vectors
            .values()
            .flatten()
            .flat_map(|spell| spell.field.controls());
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
        ui.vertical(|ui| {
            // Horizontal slider
            if used_controls.contains(&ControlKind::XSlider) {
                let value = self.world.controls.x_slider.get_or_insert(0.0);
                let something_focused = ui.memory().focus().is_some();
                let input = ui.input();
                if input.key_down(Key::D) || input.key_down(Key::A) {
                    if !something_focused {
                        *value = input.key_down(Key::D) as u8 as f32
                            - input.key_down(Key::A) as u8 as f32;
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
            // Activator
            if used_controls.contains(&ControlKind::Activation) {
                let value = &mut self.world.controls.activation;
                let something_focused = ui.memory().focus().is_some();
                ui.toggle_value(value, Word::Veni.to_string());
                let input = ui.input();
                if input.key_pressed(Key::Space) {
                    if !something_focused {
                        *value = true;
                    }
                } else if input.key_released(Key::Space) {
                    *value = false;
                }
                drop(input);
            } else {
                self.world.controls.activation = false;
            }
        });
    }
    fn handle_plot_response(&mut self, ui: &Ui, plot_resp: PlotResponse) {
        Self::handle_plot_response_impl(ui, &mut self.ui_state, &mut self.world.controls, plot_resp)
    }
    fn handle_plot_response_impl(
        ui: &Ui,
        ui_state: &mut UiState,
        controls: &mut Controls,
        plot_resp: PlotResponse,
    ) {
        if ui_state.next_player_target.is_none() {
            ui_state.next_player_target = plot_resp.hovered_pos;
        }
        if plot_resp.response.hovered() {
            controls.activation = ui.input().pointer.primary_down();
        }
    }
    fn init_plot(&self, size: f32, resolution: usize, global_alpha: f32) -> FieldPlot {
        let rect = self.world.max_rect();
        let range = rect.size().max_elem() * 0.5;
        FieldPlot::new(&self.world, rect.center(), range, global_alpha)
            .size(size)
            .resolution(resolution)
    }
    #[must_use]
    pub fn plot_stack_field(
        &self,
        ui: &mut Ui,
        size: f32,
        resolution: usize,
        global_alpha: f32,
        field: &Field,
    ) -> PlotResponse {
        let plot = self.init_plot(size, resolution, global_alpha);
        match field {
            Field::Scalar(ScalarField::Uniform(n)) => {
                FieldPlot::number_ui(&self.world, ui, size, resolution, global_alpha, *n)
            }
            Field::Scalar(field) => plot.ui(ui, field),
            Field::Vector(field) => plot.ui(ui, field),
        }
    }
    #[must_use]
    pub fn plot_io_field(
        &self,
        ui: &mut Ui,
        size: f32,
        resolution: usize,
        global_alpha: f32,
        kind: FieldKind,
    ) -> PlotResponse {
        let plot = self.init_plot(size, resolution, global_alpha);
        match kind {
            FieldKind::Scalar(kind) => plot.ui(ui, &kind),
            FieldKind::Vector(kind) => plot.ui(ui, &kind),
        }
    }
}

/// For rendering scalar stack fields
impl FieldPlottable for ScalarField {
    type Value = f32;
    fn precision(&self) -> f32 {
        1.0
    }
    fn color_midpoint(&self) -> f32 {
        if let ScalarField::Input(kind) = self {
            ScalarFieldKind::Input(*kind).color_midpoint()
        } else {
            1.0
        }
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        self.sample(world, pos, true)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        match self {
            ScalarField::Input(kind) => ScalarFieldKind::Input(*kind).get_color(t),
            _ => default_scalar_color(t),
        }
    }
}

/// For rendering vector stack fields
impl FieldPlottable for VectorField {
    type Value = Vec2;
    fn precision(&self) -> f32 {
        0.35
    }
    fn color_midpoint(&self) -> f32 {
        1.0
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        self.sample(world, pos, true)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        default_vector_color(t)
    }
}

/// For rendering scalar I/O fields
impl FieldPlottable for ScalarFieldKind {
    type Value = f32;
    fn precision(&self) -> f32 {
        match self {
            ScalarFieldKind::Input(ScalarInputFieldKind::Elevation) => 0.7,
            _ => 1.0,
        }
    }
    fn color_midpoint(&self) -> f32 {
        match self {
            ScalarFieldKind::Input(ScalarInputFieldKind::Density) => 1.0,
            ScalarFieldKind::Input(ScalarInputFieldKind::Elevation) => 3.0,
            ScalarFieldKind::Input(ScalarInputFieldKind::Magic) => 10.0,
            ScalarFieldKind::Input(ScalarInputFieldKind::Light) => 5.0,
            ScalarFieldKind::Input(ScalarInputFieldKind::Heat) => BODY_TEMP,
            ScalarFieldKind::Output(_kind) => unreachable!(),
        }
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        world.sample_scalar_field(*self, pos, true)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        match self {
            ScalarFieldKind::Input(ScalarInputFieldKind::Magic) => {
                let t = (t - 0.5) / 0.5;
                Rgba::from_rgb(0.0, t * 0.5, t)
            }
            ScalarFieldKind::Input(ScalarInputFieldKind::Light) => {
                let t = (t - 0.5) / 0.5;
                Rgba::from_rgb(t.powf(0.5), t.powf(0.6), t)
            }
            ScalarFieldKind::Input(ScalarInputFieldKind::Heat) => {
                let t = (t - 0.5) / 0.5;
                if t > 0.0 {
                    Rgba::from_rgb(t, 0.125 - 0.5 * (t - 0.25).abs(), 0.0)
                } else {
                    Rgba::from_rgb(t.abs() * 0.5, t.abs() * 0.5, t.abs())
                }
            }
            _ => default_scalar_color(t),
        }
    }
}

/// For rendering vector I/O fields
impl FieldPlottable for VectorFieldKind {
    type Value = Vec2;
    fn precision(&self) -> f32 {
        0.35
    }
    fn color_midpoint(&self) -> f32 {
        1.0
    }
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value {
        world.sample_vector_field(*self, pos, true)
    }
    fn get_color(&self, t: Self::Value) -> Rgba {
        match self {
            VectorFieldKind::Input(_) => default_vector_color(t),
            VectorFieldKind::Output(kind) => match kind {
                VectorOutputFieldKind::Gravity => simple_vector_color(t, 0.5),
                VectorOutputFieldKind::Force => simple_vector_color(t, 0.5),
            },
        }
    }
}
