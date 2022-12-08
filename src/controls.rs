use std::hash::Hash;

use eframe::{egui::*, epaint::util::hash};

pub fn apply_color_fading(visuals: &mut Visuals, visibility: f32) {
    let panel_color = visuals.window_fill();
    fade_color32(&mut visuals.extreme_bg_color, panel_color, visibility);
    fade_color32(&mut visuals.faint_bg_color, panel_color, visibility);
    fade_color32(&mut visuals.selection.bg_fill, panel_color, visibility);
    fade_color32(&mut visuals.selection.stroke.color, panel_color, visibility);
    let widgets = &mut visuals.widgets;
    for widgets in [
        &mut widgets.active,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.noninteractive,
        &mut widgets.open,
    ] {
        for color in [
            &mut widgets.bg_fill,
            &mut widgets.bg_stroke.color,
            &mut widgets.fg_stroke.color,
        ] {
            fade_color32(color, panel_color, visibility)
        }
    }
}

fn fade_color32(color: &mut Color32, faded: Color32, visibility: f32) {
    for c in 0..3 {
        color[c] = (lerp(
            faded[c] as f32 * 255.0..=color[c] as f32 * 255.0,
            visibility,
        ) / 255.0) as u8;
    }
    color[3] = (visibility * 255.0) as u8;
}

/// A button that fades into visibility
pub struct FadeButton {
    id: u64,
    text: WidgetText,
    show: bool,
    hilight: bool,
}

impl FadeButton {
    pub fn new(id_source: impl Hash, show: bool, text: impl Into<WidgetText>) -> Self {
        FadeButton {
            id: hash(id_source),
            text: text.into(),
            show,
            hilight: false,
        }
    }
    pub fn hilight(self, hilight: bool) -> Self {
        Self { hilight, ..self }
    }
}

impl Widget for FadeButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let resp = ui.scope(|ui| {
            let id = ui.make_persistent_id(self.id);
            let visibility = ui.ctx().animate_bool(id, self.show);
            if !self.show {
                return ui.label("");
            }
            apply_color_fading(ui.visuals_mut(), visibility);
            SelectableLabel::new(self.hilight, self.text.clone()).ui(ui)
        });
        resp.inner
    }
}

/// A clickable separator
pub struct SeparatorButton {
    spacing: f32,
    is_horizontal_line: Option<bool>,
    hilight: bool,
}

impl Default for SeparatorButton {
    fn default() -> Self {
        Self {
            spacing: 6.0,
            is_horizontal_line: None,
            hilight: false,
        }
    }
}

#[allow(dead_code)]
impl SeparatorButton {
    /// Hilight a different color when hovered
    pub fn hilight(mut self, hilight: bool) -> Self {
        self.hilight = hilight;
        self
    }
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
            hilight,
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
            let stroke = if hilight
                && ui
                    .input()
                    .pointer
                    .interact_pos()
                    .map_or(false, |pos| rect.contains(pos))
            {
                ui.visuals().selection.stroke
            } else if response.hovered() || response.has_focus() {
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
