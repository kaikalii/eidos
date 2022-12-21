use std::{
    cell::RefCell,
    f32::consts::PI,
    f64,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::{plot::*, *},
    epaint::{util::hash, Hsva},
};
use itertools::Itertools;
use puffin::profile_scope;
use rand::prelude::*;
use rayon::prelude::*;

use crate::{
    math::{approach_one, round_to},
    texture::textures,
    world::World,
};

pub trait FieldPlottable: Sync {
    type Value: PartitionAndPlottable;
    fn precision(&self) -> f32;
    fn color_midpoint(&self) -> f32;
    fn get_z(&self, world: &World, pos: Pos2) -> Self::Value;
    fn get_color(&self, t: Self::Value) -> Rgba;
    fn wiggle_delta(&self, point_radius: f32) -> f32 {
        wiggle_delta(point_radius, self.precision())
    }
}

fn wiggle_delta(point_radius: f32, precision: f32) -> f32 {
    point_radius * 0.1 * precision
}

pub trait PartitionAndPlottable: Sized + Send {
    fn partition_and_plot(
        plot_ui: &mut PlotUi,
        field_plot: &impl FieldPlottable<Value = Self>,
        data: PlotData<Self>,
    );
    fn format(&self, round: fn(f32) -> f32) -> String;
}

pub fn default_scalar_color(t: f32) -> Rgba {
    let h = 0.9 * (1.0 - t);
    let v = (2.0 * t - 1.0).abs();
    let s = v.powf(0.5);
    Hsva::new(h, s, v, 1.0).into()
}

pub fn simple_vector_color(t: Vec2, offset: f32) -> Rgba {
    let t = (t - Vec2::splat(0.5)) * 2.0;
    let s = t.length();
    let v = 0.9 * t.length() + 0.1;
    let h = (t.angle() + PI) / (2.0 * PI);
    let h = (h + offset) % 1.0;
    Hsva::new(h, s, v, 1.0).into()
}

pub fn default_vector_color(t: Vec2) -> Rgba {
    simple_vector_color(t, 0.75)
}

pub struct FieldPlot<'w> {
    world: &'w World,
    center: Pos2,
    range: f32,
    size: f32,
    global_alpha: f32,
}

const Z_BUCKETS: usize = if cfg!(debug_assertions) { 31 } else { 51 };
const ALPHA_BUCKETS: usize = if cfg!(debug_assertions) { 10 } else { 20 };

pub fn time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

pub struct PlotData<V> {
    points: Vec<(f32, f32, V)>,
    center: Pos2,
    point_radius: f32,
    global_alpha: f32,
}

pub struct PlotResponse {
    pub response: Response,
    pub hovered_pos: Option<Pos2>,
}

impl<'w> FieldPlot<'w> {
    pub fn new(world: &'w World, center: Pos2, range: f32, global_alpha: f32) -> Self {
        Self {
            world,
            center,
            range,
            size: 200.0,
            global_alpha,
        }
    }
    pub fn size(self, size: f32) -> Self {
        Self { size, ..self }
    }
    fn init_plot(&self) -> Plot {
        Plot::new(random::<u64>())
            .width(self.size)
            .view_aspect(1.0)
            .include_x(self.center.x - self.range)
            .include_x(self.center.x + self.range)
            .include_y(self.center.y - self.range)
            .include_y(self.center.y + self.range)
            .allow_scroll(false)
            .allow_drag(false)
            .allow_zoom(false)
            .show_axes([false; 2])
            .show_x(false)
            .show_y(false)
            .show_background(false)
    }
    fn get_data<F>(&self, field_plot: &F) -> PlotData<F::Value>
    where
        F: FieldPlottable,
    {
        let time = time();
        const SIZE_THRESHOLD: f32 = 180.0;
        let adjusted_size = if self.size > SIZE_THRESHOLD {
            self.size.sqrt() * SIZE_THRESHOLD.sqrt()
        } else {
            self.size
        };
        let resolution = (adjusted_size * field_plot.precision()) as usize;
        let step = 2.0 * self.range / resolution as f32;
        let point_radius = self.size / resolution as f32 * 0.5;
        let wiggle_delta = field_plot.wiggle_delta(point_radius);
        let center = pos2(round_to(self.center.x, step), round_to(self.center.y, step));
        let points = (0..resolution)
            .par_bridge()
            .flat_map(|i| {
                puffin::profile_scope!("point collection");
                let x = (i as f32) * step + center.x - self.range;
                let rounded_x = round_to(x, step * 0.5);
                let mut points = Vec::with_capacity(resolution);
                for j in 0..resolution {
                    let y = (j as f32) * step + center.y - self.range;
                    if pos2(x, y).distance(self.center) > self.range {
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
        PlotData {
            points,
            center,
            point_radius,
            global_alpha: self.global_alpha,
        }
    }
    pub fn ui<F>(&self, ui: &mut Ui, field_plot: &F) -> PlotResponse
    where
        F: FieldPlottable,
    {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
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
        ui.allocate_ui_at_rect(rect, |ui| self.ui_impl(ui, field_plot))
            .inner
    }
    fn ui_impl<F>(&self, ui: &mut Ui, field_plot: &F) -> PlotResponse
    where
        F: FieldPlottable,
    {
        puffin::profile_function!();
        let resp = self.init_plot().show(ui, |plot_ui| {
            let data = self.get_data(field_plot);
            let center = data.center;
            F::Value::partition_and_plot(plot_ui, field_plot, data);
            // Show coordinate tooltip
            if let Some(p) = plot_ui.pointer_coordinate() {
                let ppos = pos2(p.x as f32, p.y as f32);
                let relative_pos = ppos - center;
                if relative_pos.length() < self.range {
                    let z = field_plot.get_z(self.world, ppos);
                    let anchor = if relative_pos.y > self.range * 0.9 {
                        Align2::RIGHT_TOP
                    } else if relative_pos.x < -self.range * 0.5 {
                        Align2::LEFT_BOTTOM
                    } else if relative_pos.x > self.range * 0.5 {
                        Align2::RIGHT_BOTTOM
                    } else {
                        Align2::CENTER_BOTTOM
                    };
                    let text = format!(
                        " ({}, {}): {} ",
                        (ppos.x * 10.0).round() / 10.0,
                        (ppos.y * 10.0).round() / 10.0,
                        z.format(|z| (z * 10.0).round() / 10.0),
                    );
                    for i in 0..2 {
                        let x = p.x + (((i as f64) * 1.0 - 0.5) * 0.04);
                        for j in 0..2 {
                            let y = p.y + (((j as f64) * 1.0 - 0.5) * 0.04);
                            plot_ui.text(
                                Text::new(PlotPoint::new(x, y), text.clone())
                                    .anchor(anchor)
                                    .color(Color32::BLACK),
                            );
                        }
                    }
                    plot_ui.text(Text::new(p, text).anchor(anchor).color(Color32::WHITE));
                    Some(ppos)
                } else {
                    None
                }
            } else {
                None
            }
        });
        PlotResponse {
            response: resp.response,
            hovered_pos: resp.inner,
        }
    }
    pub fn number_ui(
        world: &'w World,
        ui: &mut Ui,
        size: f32,
        global_alpha: f32,
        n: f32,
    ) -> PlotResponse {
        let time = time();
        let plot = Self::new(world, Pos2::ZERO, 2.1, global_alpha).size(size);
        let rng = RefCell::new(SmallRng::seed_from_u64(0));
        let point_radius = plot.range * plot.size / plot.size * 0.1;
        let delta = move || {
            (time + rng.borrow_mut().gen::<f64>() * 2.0 * f64::consts::PI).sin()
                * wiggle_delta(point_radius, 1.0) as f64
        };
        let samples = (plot.size as usize * 2).max(80);
        let resp = plot.init_plot().show(ui, |plot_ui| {
            const FLOWER_MAX: f32 = 10.0;
            let frac = (n as f64) % 1.0;
            let ones_part = ((n % FLOWER_MAX).abs().floor() * n.signum()) as f64;
            let tens_part = ((n / FLOWER_MAX % FLOWER_MAX).abs().floor() * n.signum()) as f64;
            let hundreds_part = ((n / (FLOWER_MAX * FLOWER_MAX)).abs().floor() * n.signum()) as f64;
            // Fractional circle
            let frac_samples = (samples as f64 * frac) as usize;
            if frac_samples > 0 {
                plot_ui.points(
                    Points::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let x = theta.cos() * 0.8 + delta();
                            let y = theta.sin() * 0.8 + delta();
                            (x, y)
                        },
                        0.0..=frac,
                        frac_samples,
                    ))
                    .color(Color32::GREEN),
                );
            }
            // Hundreds flower
            if hundreds_part.abs() >= 1.0 {
                plot_ui.points(
                    Points::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = 1.0 + (theta * hundreds_part / 2.0).cos().powf(16.0);
                            let x = r * theta.cos() + delta();
                            let y = r * theta.sin() + delta();
                            (x, y)
                        },
                        0.0..=1.0,
                        samples,
                    ))
                    .color(Color32::YELLOW),
                );
            }
            // Tens flower
            if tens_part.abs() >= 1.0 {
                plot_ui.points(
                    Points::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = (1.0 + (theta * tens_part * 0.5).cos().powf(2.0)) * 0.9;
                            let x = r * theta.cos() + delta();
                            let y = r * theta.sin() + delta();
                            (x, y)
                        },
                        0.0..=1.0,
                        samples,
                    ))
                    .color(Color32::from_rgb(0, 100, 255)),
                );
            }
            // Ones flower
            if n == 0.0 || ones_part.abs() >= 1.0 {
                plot_ui.points(
                    Points::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = (1.0 + (theta * ones_part).cos()) * 0.8;
                            let x = r * theta.cos() + delta();
                            let y = r * theta.sin() + delta();
                            (x, y)
                        },
                        0.0..=1.0,
                        samples,
                    ))
                    .color(Color32::RED),
                );
            }
        });
        PlotResponse {
            response: resp.response,
            hovered_pos: None,
        }
    }
}

impl PartitionAndPlottable for f32 {
    fn format(&self, round: fn(f32) -> f32) -> String {
        round(*self).to_string()
    }
    fn partition_and_plot(
        plot_ui: &mut PlotUi,
        field_plot: &impl FieldPlottable<Value = Self>,
        data: PlotData<Self>,
    ) {
        puffin::profile_function!("f32");
        let center = plot_ui.plot_bounds().center().to_pos2();
        let radius = plot_ui.plot_bounds().width() as f32 * 0.5;
        let mut grouped_points = vec![vec![Vec::new(); ALPHA_BUCKETS]; Z_BUCKETS];
        let midpoint = field_plot.color_midpoint();
        for (x, y, z) in data.points {
            let t = approach_one(z, midpoint);
            let group = ((t * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                .max(0.0)
                .round() as usize)
                .min(Z_BUCKETS - 1);
            let alpha = data.global_alpha
                * (1.0
                    - (pos2(x, y).distance(center) / radius)
                        .powf(2.0)
                        .clamp(0.0, 1.0));
            let alpha = ((alpha * ALPHA_BUCKETS as f32) as usize).min(ALPHA_BUCKETS - 1);
            grouped_points[group][alpha].push(PlotPoint::new(x, y));
        }
        for (i, points) in grouped_points.into_iter().enumerate() {
            if points.is_empty() {
                continue;
            }
            let t = i as f32 / (Z_BUCKETS + 1) as f32;
            let color = field_plot.get_color(t);
            if color.a() < 1.0 / 255.0 {
                continue;
            }
            profile_scope!("point drawing", "f32");
            for (a, points) in points.into_iter().enumerate() {
                let color = Rgba::from_rgba_unmultiplied(
                    color.r(),
                    color.g(),
                    color.b(),
                    color.a() * (a as f32 + 0.1) / ALPHA_BUCKETS as f32,
                );
                plot_ui.points(
                    Points::new(PlotPoints::Owned(points))
                        .shape(MarkerShape::Circle)
                        .radius(data.point_radius)
                        .color(color),
                );
            }
        }
    }
}

impl PartitionAndPlottable for Vec2 {
    fn format(&self, round: fn(f32) -> f32) -> String {
        format!("({}, {})", round(self.x), round(self.y))
    }
    fn partition_and_plot(
        plot_ui: &mut PlotUi,
        field_plot: &impl FieldPlottable<Value = Self>,
        data: PlotData<Self>,
    ) {
        puffin::profile_function!("Vec2");
        let center = plot_ui.plot_bounds().center().to_pos2();
        let radius = plot_ui.plot_bounds().width() as f32 * 0.5;
        let mut grouped_points = vec![vec![vec![Vec::new(); ALPHA_BUCKETS]; Z_BUCKETS]; Z_BUCKETS];
        let midpoint = field_plot.color_midpoint();
        for (x, y, z) in data.points {
            let tx = approach_one(z.x, midpoint);
            let ty = approach_one(z.y, midpoint);
            let x_group = ((tx * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                .max(0.0)
                .round() as usize)
                .min(Z_BUCKETS - 1);
            let y_group = ((ty * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                .max(0.0)
                .round() as usize)
                .min(Z_BUCKETS - 1);
            let alpha = data.global_alpha
                * (1.0
                    - (pos2(x, y).distance(center) / radius)
                        .powf(2.0)
                        .clamp(0.0, 1.0));
            let alpha = ((alpha * ALPHA_BUCKETS as f32) as usize).min(ALPHA_BUCKETS - 1);
            grouped_points[x_group][y_group][alpha].push(PlotPoint::new(x, y));
        }
        for (i, points) in grouped_points.into_iter().enumerate() {
            if points.is_empty() {
                continue;
            }
            for (j, points) in points.into_iter().enumerate() {
                if points.is_empty() {
                    continue;
                }
                let t = vec2(
                    i as f32 / (Z_BUCKETS + 1) as f32,
                    j as f32 / (Z_BUCKETS + 1) as f32,
                );
                let color = field_plot.get_color(t);
                if color.a() < 1.0 / 255.0 {
                    continue;
                }
                profile_scope!("point drawing", "Vec2");
                let t = (t - Vec2::splat(0.5)) * 2.0;
                for (a, points) in points.into_iter().enumerate() {
                    let arrow_length = data.point_radius * 0.1;
                    let tips = points
                        .iter()
                        .map(|p| {
                            PlotPoint::new(
                                p.x as f32 + t.x * arrow_length,
                                p.y as f32 + t.y * arrow_length,
                            )
                        })
                        .collect_vec();
                    let color = Rgba::from_rgba_unmultiplied(
                        color.r(),
                        color.g(),
                        color.b(),
                        color.a() * (a as f32 + 0.1) / ALPHA_BUCKETS as f32,
                    );
                    plot_ui.arrows(
                        Arrows::new(PlotPoints::Owned(points), PlotPoints::Owned(tips))
                            .color(color),
                    );
                }
            }
        }
    }
}
