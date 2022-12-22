use std::{
    cell::RefCell,
    cmp::Ordering,
    f32::consts::PI,
    f64,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::*,
    epaint::{util::hash, Hsva},
};
use rand::prelude::*;
use rayon::prelude::*;

use crate::{
    color::Color,
    math::{approach_one, round_to},
    texture::textures,
    world::World,
};

pub struct FieldPlot<'w> {
    world: &'w World,
    world_center: Pos2,
    world_range: f32,
    size: f32,
    global_alpha: f32,
}

pub struct PlotData<V> {
    points: Vec<(f32, f32, V)>,
    center: Pos2,
    range: f32,
    point_radius: f32,
    global_alpha: f32,
}

pub trait FieldPlottable: Sync {
    type Value: Plottable;
    fn precision(&self) -> f32;
    fn color_midpoint(&self) -> f32;
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value;
    fn get_color(&self, t: Self::Value) -> Color;
    fn wiggle_delta(&self, point_radius: f32) -> f32 {
        wiggle_delta(point_radius, self.precision())
    }
}

pub trait Plottable: Sized + Send {
    fn cmp(&self, other: &Self) -> Ordering;
    fn plot(
        ui: &mut Ui,
        rect: Rect,
        field_plot: &impl FieldPlottable<Value = Self>,
        data: PlotData<Self>,
    );
    fn format(&self, round: fn(f32) -> f32) -> String;
}

fn wiggle_delta(point_radius: f32, precision: f32) -> f32 {
    point_radius * 0.1 * precision
}

pub fn time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

pub fn default_scalar_color(t: f32) -> Color {
    let h = 0.9 * (1.0 - t);
    let v = (2.0 * t - 1.0).abs().sqrt();
    let s = v.powi(2);
    Hsva::new(h, s, v, 1.0).into()
}

pub fn simple_vector_color(t: Vec2, offset: f32) -> Color {
    let t = (t - Vec2::splat(0.5)) * 2.0;
    let s = t.length();
    let v = 0.9 * t.length() + 0.1;
    let h = (t.angle() + PI) / (2.0 * PI);
    let h = (h + offset) % 1.0;
    Hsva::new(h, s, v, 1.0).into()
}

pub fn default_vector_color(t: Vec2) -> Color {
    simple_vector_color(t, 0.75)
}

pub struct PlotResponse {
    pub response: Response,
    pub hovered_pos: Option<Pos2>,
}

impl<'w> FieldPlot<'w> {
    pub fn new(world: &'w World, center: Pos2, range: f32, size: f32, global_alpha: f32) -> Self {
        FieldPlot {
            world,
            world_center: center,
            world_range: range,
            size,
            global_alpha,
        }
    }
    pub fn show<F>(&self, ui: &mut Ui, field_plot: &F) -> PlotResponse
    where
        F: FieldPlottable,
    {
        puffin::profile_function!();
        // Allocate rect and get response
        let rect = Rect::from_min_size(ui.cursor().left_top(), Vec2::splat(self.size));
        let response = ui.allocate_rect(rect, Sense::drag());
        // Draw background shadow
        let mut panel_color = ui.visuals().panel_fill;
        panel_color =
            Color32::from_rgba_unmultiplied(panel_color.r(), panel_color.g(), panel_color.b(), 210);
        let texture_id = textures(|t| t.circle_gradient.id());
        ui.painter().image(
            texture_id,
            rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            panel_color,
        );
        // Plot data
        let data = self.get_data(field_plot);
        F::Value::plot(ui, rect, field_plot, data);
        // Handle hovering
        let mut hovered_pos = None;
        if let Some(hpos) = response.hover_pos() {
            let normalized_rect_pos = (hpos - rect.left_top()) / (rect.width() / 2.0);
            let world_tl = self.world_center + vec2(-self.world_range, self.world_range);
            let pos =
                world_tl + vec2(normalized_rect_pos.x, -normalized_rect_pos.y) * self.world_range;
            let relative_pos = pos - self.world_center;
            if relative_pos.length() < self.world_range {
                let z = field_plot.get_z(self.world, pos);
                let anchor = if relative_pos.y > self.world_range * 0.9 {
                    Align2::RIGHT_TOP
                } else if relative_pos.x < -self.world_range * 0.5 {
                    Align2::LEFT_BOTTOM
                } else if relative_pos.x > self.world_range * 0.5 {
                    Align2::RIGHT_BOTTOM
                } else {
                    Align2::CENTER_BOTTOM
                };
                let text = format!(
                    " ({}, {}): {} ",
                    (pos.x * 10.0).round() / 10.0,
                    (pos.y * 10.0).round() / 10.0,
                    z.format(|z| (z * 10.0).round() / 10.0),
                );
                let text = text.as_str();
                let painter = ui.painter();
                let font_id = &ui.style().text_styles[&TextStyle::Body];
                for i in 0..2 {
                    let x = hpos.x + i as f32 - 0.5;
                    for j in 0..2 {
                        let y = hpos.y + j as f32 - 0.5;
                        painter.text(pos2(x, y), anchor, text, font_id.clone(), Color32::BLACK);
                    }
                }
                painter.text(hpos, anchor, text, font_id.clone(), Color32::WHITE);
                hovered_pos = Some(pos);
            }
        }
        PlotResponse {
            response,
            hovered_pos,
        }
    }
    pub fn show_number(ui: &mut Ui, size: f32, global_alpha: f32, n: f32) -> PlotResponse {
        let rect = Rect::from_min_size(ui.cursor().left_top(), Vec2::splat(size));
        let response = ui.allocate_rect(rect, Sense::drag());
        let time = time();
        const RANGE: f32 = 2.1;
        let rng = RefCell::new(SmallRng::seed_from_u64(0));
        let point_radius = size * 0.005;
        let ratio = size / RANGE / 2.0;
        let delta = move || {
            (time + rng.borrow_mut().gen::<f64>() * 2.0 * f64::consts::PI).sin() as f32
                * wiggle_delta(point_radius, 1.0)
        };
        let samples = (size as usize * 2).max(80);
        const FLOWER_MAX: f32 = 10.0;
        let frac = n % 1.0;
        let ones_part = (n % FLOWER_MAX).abs().floor() * n.signum();
        let tens_part = (n / FLOWER_MAX % FLOWER_MAX).abs().floor() * n.signum();
        let hundreds_part = (n / (FLOWER_MAX * FLOWER_MAX)).abs().floor() * n.signum();
        let painter = ui.painter();
        // Fractional circle
        let frac_samples = (samples as f32 * frac) as usize;
        if frac_samples > 0 {
            let fill_color = Color::rgba(0.0, 1.0, 0.0, global_alpha);
            for i in 0..frac_samples {
                let t = i as f32 / frac_samples as f32;
                let theta = t * 2.0 * PI * frac;
                let x = theta.cos() * 0.8 + delta();
                let y = theta.sin() * 0.8 + delta();
                let pos = rect.center() + vec2(x, -y) * ratio;
                painter.circle_filled(pos, point_radius, fill_color);
            }
        }
        // Hundreds flower
        if hundreds_part.abs() >= 1.0 {
            let fill_color = Color::rgba(1.0, 1.0, 0.0, global_alpha);
            for i in 0..samples {
                let t = i as f32 / samples as f32;
                let theta = t * 2.0 * PI;
                let r = 1.0 + (theta * hundreds_part / 2.0).cos().powf(16.0);
                let x = r * theta.cos() + delta();
                let y = r * theta.sin() + delta();
                let pos = rect.center() + vec2(x, -y) * ratio;
                painter.circle_filled(pos, point_radius, fill_color);
            }
        }
        // Tens flower
        if tens_part.abs() >= 1.0 {
            let fill_color = Color::rgba(0.0, 0.4, 1.0, global_alpha);
            for i in 0..samples {
                let t = i as f32 / samples as f32;
                let theta = t * 2.0 * PI;
                let r = (1.0 + (theta * tens_part * 0.5).cos().powf(2.0)) * 0.9;
                let x = r * theta.cos() + delta();
                let y = r * theta.sin() + delta();
                let pos = rect.center() + vec2(x, -y) * ratio;
                painter.circle_filled(pos, point_radius, fill_color);
            }
        }
        // Ones flower
        if n == 0.0 || ones_part.abs() >= 1.0 {
            let fill_color = Color::rgba(1.0, 0.0, 0.0, global_alpha);
            for i in 0..samples {
                let t = i as f32 / samples as f32;
                let theta = t * 2.0 * PI;
                let r = (1.0 + (theta * ones_part).cos()) * 0.8;
                let x = r * theta.cos() + delta();
                let y = r * theta.sin() + delta();
                let pos = rect.center() + vec2(x, -y) * ratio;
                painter.circle_filled(pos, point_radius, fill_color);
            }
        }
        PlotResponse {
            response,
            hovered_pos: None,
        }
    }
    fn get_data<F>(&self, field_plot: &F) -> PlotData<F::Value>
    where
        F: FieldPlottable,
    {
        puffin::profile_function!();
        let time = time();
        const SIZE_THRESHOLD: f32 = 180.0;
        let adjusted_size = if self.size > SIZE_THRESHOLD {
            self.size.sqrt() * SIZE_THRESHOLD.sqrt()
        } else {
            self.size
        };
        let resolution = (adjusted_size * field_plot.precision()) as usize;
        let step = 2.0 * self.world_range / resolution as f32;
        let point_radius = self.size / resolution as f32 * 0.5;
        let wiggle_delta = field_plot.wiggle_delta(point_radius);
        let world_center = pos2(
            round_to(self.world_center.x, step),
            round_to(self.world_center.y, step),
        );
        puffin::profile_scope!("point collection outer");
        let mut points: Vec<_> = (0..resolution)
            .par_bridge()
            .flat_map(|i| {
                puffin::profile_scope!("point collection inner");
                let x = world_center.x - self.world_range + (i as f32) * step;
                let rounded_x = round_to(x, step * 0.5);
                let mut points = Vec::with_capacity(resolution);
                for j in 0..resolution {
                    let y = world_center.y - self.world_range + (j as f32) * step;
                    if pos2(x, y).distance(self.world_center) > self.world_range {
                        continue;
                    }
                    let rounded_y = round_to(y, step * 0.5);
                    let mut rng = SmallRng::seed_from_u64(hash((
                        (rounded_x * 1e6) as i64,
                        (rounded_y * 1e6) as i64,
                    )));
                    let dxt = rng.gen::<f32>() + rounded_x - x;
                    let dyt = rng.gen::<f32>() + rounded_x - x;
                    let z = field_plot.get_z(self.world, pos2(rounded_x, rounded_y));
                    let dx = (time + dxt as f64 * f64::consts::TAU).sin() as f32 * wiggle_delta;
                    let dy = (time + dyt as f64 * f64::consts::TAU).sin() as f32 * wiggle_delta;
                    points.push((x + dx, y + dy, z));
                }
                points
            })
            .collect();
        points.par_sort_by(|(_, _, a), (_, _, b)| a.cmp(b));
        PlotData {
            points,
            center: world_center,
            point_radius,
            range: self.world_range,
            global_alpha: self.global_alpha,
        }
    }
}

impl Plottable for f32 {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_nan(), other.is_nan()) {
            (false, false) => self.abs().partial_cmp(&other.abs()).unwrap(),
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
        }
    }
    fn format(&self, round: fn(f32) -> f32) -> String {
        round(*self).to_string()
    }
    fn plot(
        ui: &mut Ui,
        rect: Rect,
        field_plot: &impl FieldPlottable<Value = Self>,
        data: PlotData<Self>,
    ) {
        puffin::profile_function!("f32");
        let midpoint = field_plot.color_midpoint();
        let painter = ui.painter();
        let world_tl = data.center + vec2(-data.range, data.range);
        let ratio = rect.width() / (data.range * 2.0);
        for (x, y, z) in data.points {
            let t = approach_one(z, midpoint) * 0.5 + 0.5;
            let pos = pos2(x, y);
            let alpha = data.global_alpha
                * (1.0
                    - (pos.distance(data.center) / data.range)
                        .powf(2.0)
                        .clamp(0.0, 1.0));
            let color = field_plot.get_color(t).mul_a(alpha);
            if color.a < 1.0 / 255.0 {
                continue;
            }
            let rel_pos = pos - world_tl;
            let point = rect.left_top() + vec2(rel_pos.x, -rel_pos.y) * ratio;
            painter.circle_filled(point, data.point_radius, color);
        }
    }
}

impl Plottable for Vec2 {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.length();
        let b = other.length();
        match (a.is_nan(), b.is_nan()) {
            (false, false) => a.partial_cmp(&b).unwrap(),
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
        }
    }
    fn format(&self, round: fn(f32) -> f32) -> String {
        format!("({}, {})", round(self.x), round(self.y))
    }
    fn plot(
        ui: &mut Ui,
        rect: Rect,
        field_plot: &impl FieldPlottable<Value = Self>,
        data: PlotData<Self>,
    ) {
        puffin::profile_function!("Vec2");
        let midpoint = field_plot.color_midpoint();
        let painter = ui.painter();
        let world_tl = data.center + vec2(-data.range, data.range);
        let ratio = rect.width() / (data.range * 2.0);
        for (x, y, z) in data.points {
            let t = vec2(approach_one(z.x, midpoint), approach_one(z.y, midpoint));
            let pos = pos2(x, y);
            let alpha = data.global_alpha
                * (1.0
                    - (pos.distance(data.center) / data.range)
                        .powf(2.0)
                        .clamp(0.0, 1.0));
            let color = field_plot
                .get_color(t * 0.5 + Vec2::splat(0.5))
                .mul_a(alpha);
            if color.a < 1.0 / 255.0 {
                continue;
            }
            let rel_pos = pos - world_tl;
            let point = rect.left_top() + vec2(rel_pos.x, -rel_pos.y) * ratio;
            let arrow_length = data.point_radius * 2.0;
            painter.arrow(
                point,
                vec2(t.x, -t.y) * arrow_length,
                Stroke::new(data.point_radius * 0.4, color),
            );
        }
    }
}
