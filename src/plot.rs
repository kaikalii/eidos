use std::{
    f64,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::{plot::*, *},
    epaint::color::Hsva,
};
use itertools::Itertools;
use rand::prelude::*;

use crate::field::{FieldKind, GenericFieldKind};

pub type FieldPlotKey = FieldKind<GenericFieldKind>;

pub trait FieldPlot {
    type Value: PartitionAndPlottable;
    fn key(&self) -> FieldPlotKey;
    fn get_z(&self, x: f32, y: f32) -> Self::Value;
    fn get_color(&self, t: Self::Value) -> Color32;
}

pub trait PartitionAndPlottable: Sized {
    fn partition_and_plot(
        plot_ui: &mut PlotUi,
        field_plot: &impl FieldPlot<Value = Self>,
        point_radius: f32,
        points: Vec<(f32, f32, Self)>,
    );
    fn format(&self, round: fn(f32) -> f32) -> String;
}

pub fn default_scalar_color(t: f32) -> Color32 {
    let h = 0.9 * (1.0 - t);
    let v = (2.0 * t - 1.0).abs();
    let s = v.powf(0.5);
    Hsva::new(h, s, v, 1.0).into()
}

pub fn default_vector_color(t: Vec2) -> Color32 {
    Rgba::from_rgba_unmultiplied(1.0 - t.y, t.x, 1.0, 1.0).into()
}

pub struct MapPlot {
    center: Vec2,
    range: f32,
    width: f32,
    resolution: usize,
}

const Z_BUCKETS: usize = 99;

impl MapPlot {
    pub fn new(center: Vec2, range: f32) -> Self {
        Self {
            center,
            range,
            width: 200.0,
            resolution: 100,
        }
    }
    pub fn _width(self, width: f32) -> Self {
        Self { width, ..self }
    }
    pub fn _resolution(self, resolution: usize) -> Self {
        Self { resolution, ..self }
    }

    fn init_plot<F>(&self, field_plot: &F) -> Plot
    where
        F: FieldPlot,
    {
        Plot::new(field_plot.key())
            .width(self.width)
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
        self.init_plot(&field_plot).show(ui, |plot_ui| {
            let t = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            let mut rng = SmallRng::seed_from_u64(0);
            let point_radius = self.width / self.resolution as f32;
            let step = 2.0 * self.range / self.resolution as f32;
            let mut points = Vec::with_capacity(self.resolution * self.resolution);
            for i in 0..self.resolution {
                let x = (i as f32) * step + self.center.x - self.range;
                for j in 0..self.resolution {
                    let y = (j as f32) * step + self.center.y - self.range;
                    if vec2(x, y).length() > self.range {
                        continue;
                    }
                    let z = field_plot.get_z(x, y);
                    let dx = (t + rng.gen::<f64>() * 2.0 * f64::consts::PI).sin() as f32
                        * point_radius
                        * 0.05;
                    let dy = (t + rng.gen::<f64>() * 2.0 * f64::consts::PI).sin() as f32
                        * point_radius
                        * 0.05;
                    points.push((x + dx, y + dy, z));
                }
            }
            F::Value::partition_and_plot(plot_ui, &field_plot, point_radius, points);
            if let Some(p) = plot_ui.pointer_coordinate() {
                if p.to_vec2().length() < self.range {
                    let x = p.x as f32;
                    let y = p.y as f32;
                    let z = field_plot.get_z(x, y);
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
}

impl PartitionAndPlottable for f32 {
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
                let t = vec2(i as f32 / Z_BUCKETS as f32, j as f32 / Z_BUCKETS as f32);
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
}

pub fn plot_number(ui: &mut Ui, n: f32, key: FieldPlotKey) {
    Plot::new(key)
        .width(50.0)
        .height(50.0)
        .show_axes([false; 2])
        .show_x(false)
        .show_y(false)
        .show_background(false)
        .allow_zoom(false)
        .allow_drag(false)
        .include_x(-2.0)
        .include_x(2.0)
        .include_y(-2.0)
        .include_y(2.0)
        .allow_scroll(false)
        .show(ui, |plot_ui| {
            const FLOWER_MAX: f32 = 10.0;
            let frac = (n as f64) % 1.0;
            let ones_part = ((n % FLOWER_MAX).abs().floor() * n.signum()) as f64;
            let tens_part = ((n / FLOWER_MAX % FLOWER_MAX).abs().floor() * n.signum()) as f64;
            let hundreds_part = ((n / (FLOWER_MAX * FLOWER_MAX)).abs().floor() * n.signum()) as f64;
            // Fractional circle
            plot_ui.line(
                Line::new(PlotPoints::from_parametric_callback(
                    |t| {
                        let theta = t * 2.0 * f64::consts::PI;
                        (theta.cos(), theta.sin())
                    },
                    0.0..=frac,
                    100,
                ))
                .color(Color32::GREEN),
            );
            // Hundreds flower
            if hundreds_part.abs() >= 1.0 {
                plot_ui.line(
                    Line::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = 1.0 + (theta * hundreds_part / 2.0).cos().powf(16.0);
                            let x = r * theta.cos();
                            let y = r * theta.sin();
                            (x, y)
                        },
                        0.0..=1.0,
                        100,
                    ))
                    .color(Color32::YELLOW),
                );
            }
            // Tens flower
            if tens_part.abs() >= 1.0 {
                plot_ui.line(
                    Line::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = 1.0 + (theta * tens_part * 0.5).cos().powf(2.0);
                            let x = r * theta.cos();
                            let y = r * theta.sin();
                            (x, y)
                        },
                        0.0..=1.0,
                        100,
                    ))
                    .color(Color32::from_rgb(0, 100, 255)),
                );
            }
            // Ones flower
            if n == 0.0 || ones_part.abs() >= 1.0 {
                plot_ui.line(
                    Line::new(PlotPoints::from_parametric_callback(
                        |t| {
                            let theta = t * 2.0 * f64::consts::PI;
                            let r = 1.0 + (theta * ones_part).cos();
                            let x = r * theta.cos();
                            let y = r * theta.sin();
                            (x, y)
                        },
                        0.0..=1.0,
                        100,
                    ))
                    .color(Color32::RED),
                );
            }
        });
}
