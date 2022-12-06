use std::{
    cell::RefCell,
    f32::consts::PI,
    f64,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::{plot::*, *},
    epaint::color::Hsva,
};
use itertools::Itertools;
use rand::prelude::*;

use crate::{
    field::{FieldKind, GenericInputFieldKind},
    world::World,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldPlotKey {
    Kind(FieldKind<GenericInputFieldKind>),
    Staging(usize),
}

pub trait FieldPlot {
    type Value: PartitionAndPlottable;
    fn key(&self) -> FieldPlotKey;
    fn get_z(&self, world: &World, x: f32, y: f32) -> Self::Value;
    fn get_color(&self, t: Self::Value) -> Color32;
}

pub trait PartitionAndPlottable: Sized {
    const SCALE: f32;
    fn partition_and_plot(
        plot_ui: &mut PlotUi,
        field_plot: &impl FieldPlot<Value = Self>,
        point_radius: f32,
        points: Vec<(f32, f32, Self)>,
    );
    fn format(&self, round: fn(f32) -> f32) -> String;
    fn wiggle_delta(point_radius: f32) -> f32 {
        point_radius * 0.05 / Self::SCALE
    }
}

pub fn default_scalar_color(t: f32) -> Color32 {
    let h = 0.9 * (1.0 - t);
    let v = (2.0 * t - 1.0).abs();
    let s = v.powf(0.5);
    Hsva::new(h, s, v, 1.0).into()
}

pub fn default_vector_color(t: Vec2) -> Color32 {
    let t = (t - Vec2::splat(0.5)) * 2.0;
    let s = t.length();
    let v = 0.9 * t.length() + 0.1;
    let h = (t.angle() + PI) / (2.0 * PI);
    let h = (h + 0.75) % 1.0;
    Hsva::new(h, s, v, 1.0).into()
}

pub struct MapPlot<'w> {
    world: &'w World,
    center: Pos2,
    range: f32,
    size: f32,
    resolution: usize,
}

const Z_BUCKETS: usize = 51;

fn time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

impl<'w> MapPlot<'w> {
    pub fn new(world: &'w World, center: Pos2, range: f32) -> Self {
        Self {
            world,
            center,
            range,
            size: 200.0,
            resolution: 100,
        }
    }
    pub fn size(self, size: f32) -> Self {
        Self { size, ..self }
    }
    pub fn resolution(self, resolution: usize) -> Self {
        Self { resolution, ..self }
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
    pub fn ui<F>(&self, ui: &mut Ui, field_plot: F)
    where
        F: FieldPlot,
    {
        let time = time();
        self.init_plot().show(ui, |plot_ui| {
            let mut rng = SmallRng::seed_from_u64(0);
            let resolution = ((self.resolution as f32) / F::Value::SCALE) as usize;
            let step = 2.0 * self.range / resolution as f32;
            let point_radius = self.range * self.size / resolution as f32 * 0.1;
            let mut points = Vec::with_capacity(self.resolution * resolution);
            for i in 0..self.resolution {
                let x = (i as f32) * step + self.center.x - self.range;
                for j in 0..self.resolution {
                    let y = (j as f32) * step + self.center.y - self.range;
                    let dxt = rng.gen::<f64>();
                    let dyt = rng.gen::<f64>();
                    if pos2(x, y).distance(self.center) > self.range {
                        continue;
                    }
                    let z = field_plot.get_z(self.world, x, y);
                    let dx = (time + dxt * 2.0 * f64::consts::PI).sin() as f32
                        * F::Value::wiggle_delta(point_radius);
                    let dy = (time + dyt * 2.0 * f64::consts::PI).sin() as f32
                        * F::Value::wiggle_delta(point_radius);
                    points.push((x + dx, y + dy, z));
                }
            }
            F::Value::partition_and_plot(plot_ui, &field_plot, point_radius, points);
            if let Some(p) = plot_ui.pointer_coordinate() {
                if p.to_vec2().length() < self.range {
                    let x = p.x as f32;
                    let y = p.y as f32;
                    let z = field_plot.get_z(self.world, x, y);
                    let anchor = if y > self.range * 0.9 {
                        Align2::RIGHT_TOP
                    } else if x < -self.range * 0.5 {
                        Align2::LEFT_BOTTOM
                    } else if x > self.range * 0.5 {
                        Align2::RIGHT_BOTTOM
                    } else {
                        Align2::CENTER_BOTTOM
                    };
                    plot_ui.text(
                        Text::new(
                            p,
                            format!(
                                " ({}, {}): {} ",
                                (x * 10.0).round() / 10.0,
                                (y * 10.0).round() / 10.0,
                                z.format(|z| (z * 10.0).round() / 10.0),
                            ),
                        )
                        .anchor(anchor),
                    );
                }
            }
        });
    }
    pub fn number_ui(world: &'w World, ui: &mut Ui, size: f32, resolution: usize, n: f32) {
        let time = time();
        let plot = Self::new(world, Pos2::ZERO, 2.1)
            .size(size)
            .resolution(resolution);
        let rng = RefCell::new(SmallRng::seed_from_u64(0));
        let point_radius = plot.range * plot.size / plot.resolution as f32 * 0.1;
        let delta = move || {
            (time + rng.borrow_mut().gen::<f64>() * 2.0 * f64::consts::PI).sin()
                * f32::wiggle_delta(point_radius) as f64
        };
        let samples = (plot.resolution * 2).max(80);
        plot.init_plot().show(ui, |plot_ui| {
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
    }
}

impl PartitionAndPlottable for f32 {
    const SCALE: f32 = 1.0;
    fn format(&self, round: fn(f32) -> f32) -> String {
        round(*self).to_string()
    }
    fn partition_and_plot(
        plot_ui: &mut PlotUi,
        field_plot: &impl FieldPlot<Value = Self>,
        point_radius: f32,
        points: Vec<(f32, f32, Self)>,
    ) {
        let (min_z, max_z) = points
            .iter()
            .map(|(_, _, z)| *z)
            .minmax()
            .into_option()
            .unwrap();
        let max_abs_z = min_z.abs().max(max_z.abs()).max(0.1);
        let mut grouped_points = vec![Vec::new(); Z_BUCKETS];
        for (x, y, z) in points {
            let group = ((z / max_abs_z * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                .max(0.0)
                .round() as usize)
                .min(Z_BUCKETS - 1);
            grouped_points[group].push(PlotPoint::new(x, y));
        }
        for (i, points) in grouped_points.into_iter().enumerate() {
            if points.is_empty() {
                continue;
            }
            let t = i as f32 / (Z_BUCKETS + 1) as f32;
            let color = field_plot.get_color(t);
            if color.a() == 0 {
                continue;
            }
            plot_ui.points(
                Points::new(PlotPoints::Owned(points))
                    .shape(MarkerShape::Circle)
                    .radius(point_radius)
                    .color(color),
            );
        }
    }
}

impl PartitionAndPlottable for Vec2 {
    const SCALE: f32 = 3.0;
    fn format(&self, round: fn(f32) -> f32) -> String {
        format!("({}, {})", round(self.x), round(self.y))
    }
    fn partition_and_plot(
        plot_ui: &mut PlotUi,
        field_plot: &impl FieldPlot<Value = Self>,
        point_radius: f32,
        points: Vec<(f32, f32, Self)>,
    ) {
        let (min_zx, max_zx) = points
            .iter()
            .map(|(_, _, z)| z.x)
            .minmax()
            .into_option()
            .unwrap();
        let (min_zy, max_zy) = points
            .iter()
            .map(|(_, _, z)| z.y)
            .minmax()
            .into_option()
            .unwrap();
        let max_abs_zx = min_zx.abs().max(max_zx.abs()).max(0.1);
        let max_abs_zy = min_zy.abs().max(max_zy.abs()).max(0.1);
        let mut grouped_points = vec![vec![Vec::new(); Z_BUCKETS]; Z_BUCKETS];
        for (x, y, z) in points {
            let x_group = ((z.x / max_abs_zx * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                .max(0.0)
                .round() as usize)
                .min(Z_BUCKETS - 1);
            let y_group = ((z.y / max_abs_zy * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                .max(0.0)
                .round() as usize)
                .min(Z_BUCKETS - 1);
            grouped_points[x_group][y_group].push(PlotPoint::new(x, y));
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
                if color.a() == 0 {
                    continue;
                }
                let t = (t - Vec2::splat(0.5)) * 2.0;
                let arrow_length = point_radius * 0.1;
                let tips = points
                    .iter()
                    .map(|p| {
                        PlotPoint::new(
                            p.x as f32 + t.x * arrow_length,
                            p.y as f32 + t.y * arrow_length,
                        )
                    })
                    .collect_vec();
                plot_ui.arrows(
                    Arrows::new(PlotPoints::Owned(points), PlotPoints::Owned(tips)).color(color),
                );
            }
        }
    }
}
