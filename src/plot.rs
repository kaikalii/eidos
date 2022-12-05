use std::{
    f64,
    hash::Hash,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::{plot::*, *},
    epaint::color::Hsva,
};
use itertools::Itertools;
use rand::prelude::*;

pub trait FieldPlot {
    type Key: Hash;
    fn key(&self) -> Self::Key;
    fn get_z(&self, x: f32, y: f32) -> f32;
    fn get_color(&self, t: f32) -> Color32 {
        let h = 0.9 * (1.0 - t);
        let v = (2.0 * t - 1.0).abs();
        let s = v.powf(0.5);
        Hsva::new(h, s, v, 1.0).into()
    }
}

pub struct MapPlot {
    center: Vec2,
    range: f32,
    width: f32,
    resolution: usize,
}

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
    pub fn ui<F>(&self, ui: &mut Ui, field_plot: F)
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
            .show(ui, |plot_ui| {
                let t = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64();
                let mut rng = SmallRng::seed_from_u64(0);
                let point_radius = self.width / self.resolution as f32;
                const Z_BUCKETS: usize = 99;
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
                let (min_z, max_z) = points
                    .iter()
                    .map(|(_, _, z)| *z)
                    .minmax()
                    .into_option()
                    .unwrap();
                let max_abs_z = min_z.abs().max(max_z.abs());
                let mut grouped_points = vec![Vec::new(); Z_BUCKETS];
                for (x, y, z) in points {
                    let group = ((z / max_abs_z * Z_BUCKETS as f32 * 0.5 + Z_BUCKETS as f32 * 0.5)
                        .max(0.0)
                        .round() as usize)
                        .min(Z_BUCKETS - 1);
                    grouped_points[group].push(PlotPoint::new(x, y));
                }
                for (k, points) in grouped_points.into_iter().enumerate() {
                    let t = k as f32 / Z_BUCKETS as f32;
                    let color = field_plot.get_color(t);
                    for point in points {
                        plot_ui.points(
                            Points::new(PlotPoints::Owned(vec![point]))
                                .shape(MarkerShape::Circle)
                                .radius(point_radius)
                                .color(color),
                        );
                    }
                }
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
                                    (z * 10.0).round() / 10.0
                                ),
                            )
                            .anchor(anchor),
                        );
                    }
                }
            });
    }
}
