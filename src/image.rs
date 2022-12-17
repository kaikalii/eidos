use std::{collections::HashMap, f64};

use eframe::{
    egui::{plot::*, *},
    epaint::mutex::Mutex,
};
use image::RgbaImage;
use once_cell::sync::Lazy;
use rand::prelude::*;

use crate::{plot::time, utils::resources_path};

static IMAGES: Lazy<Mutex<HashMap<String, RgbaImage>>> = Lazy::new(Default::default);

pub fn use_image<F, T>(name: &str, mut f: F) -> T
where
    F: FnMut(&RgbaImage) -> T,
{
    let mut images = IMAGES.lock();
    let image = images.entry(name.into()).or_insert_with(|| {
        let path = resources_path().join("images").join(name);
        image::open(path)
            .map(|image| image.to_rgba8())
            .unwrap_or_else(|_| {
                eprintln!("Failed to load image: {}", name);
                RgbaImage::new(1, 1)
            })
    });
    f(image)
}

pub enum ImagePlotKind {
    Portrait(bool),
    #[allow(dead_code)]
    Background,
}

pub fn image_plot(ui: &mut Ui, name: &str, max_size: Vec2, kind: ImagePlotKind) {
    use_image(name, |image| {
        let image_aspect = image.width() as f32 / image.height() as f32;
        let max_size_aspect = max_size.x / max_size.y;
        let size = match kind {
            ImagePlotKind::Portrait(_) => {
                if image_aspect > max_size_aspect {
                    Vec2::new(max_size.x, max_size.x / image_aspect)
                } else {
                    Vec2::new(max_size.y * image_aspect, max_size.y)
                }
            }
            ImagePlotKind::Background => {
                if image_aspect > max_size_aspect {
                    Vec2::new(max_size.y * image_aspect, max_size.y)
                } else {
                    Vec2::new(max_size.x, max_size.x / image_aspect)
                }
            }
        };
        let alpha = match kind {
            ImagePlotKind::Portrait(true) => 0.8,
            ImagePlotKind::Portrait(false) => 0.1,
            _ => 1.0,
        };
        let step = match kind {
            ImagePlotKind::Portrait(_) => 5.0,
            ImagePlotKind::Background => 5.0,
        };
        let wiggle_range = match kind {
            ImagePlotKind::Portrait(_) => step,
            ImagePlotKind::Background => step * 0.3,
        };
        let color_mul = match kind {
            ImagePlotKind::Portrait(_) => 1.0,
            ImagePlotKind::Background => 0.6,
        };
        let time = time();
        Plot::new(random::<u64>())
            .width(size.x)
            .height(size.y)
            .show_axes([false; 2])
            .show_background(false)
            .show_x(false)
            .show_y(false)
            .include_x(-wiggle_range)
            .include_x(size.x + wiggle_range)
            .include_y(-wiggle_range)
            .include_y(size.y + wiggle_range)
            .data_aspect(1.0)
            .view_aspect(image_aspect)
            .allow_scroll(false)
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                let mut rng = SmallRng::seed_from_u64(0);
                for i in 0..(size.x / step) as usize {
                    for j in 0..(size.y / step) as usize {
                        let x = i as f32 * step;
                        let y = j as f32 * step;
                        let color = image.get_pixel(
                            (x / size.x * image.width() as f32) as u32,
                            ((0.9999 - y / size.y) * image.height() as f32) as u32,
                        );
                        let dx = wiggle_range
                            * (time + rng.gen_range(0.0..=f64::consts::TAU)).sin() as f32;
                        let dy = wiggle_range
                            * (time + rng.gen_range(0.0..=f64::consts::TAU)).sin() as f32;
                        if color[3] == 0 {
                            continue;
                        }
                        let dropoff = match kind {
                            ImagePlotKind::Portrait(_) => 1.0,
                            ImagePlotKind::Background => {
                                let dist_from_center =
                                    (Vec2::new(x + dx, y + dy) - max_size * 0.5).length();
                                (1.0 - dist_from_center / ((size.x + size.y) * 0.25)).max(0.01)
                            }
                        };
                        let alpha = alpha * dropoff;
                        let color_mul = color_mul * dropoff;
                        let color = Rgba::from_rgba_unmultiplied(
                            color[0] as f32 / 255.0 * color_mul,
                            color[1] as f32 / 255.0 * color_mul,
                            color[2] as f32 / 255.0 * color_mul,
                            color[3] as f32 / 255.0 * alpha,
                        );
                        let point = PlotPoint::new(x + dx, y + dy);
                        plot_ui.points(
                            Points::new(PlotPoints::Owned(vec![point]))
                                .color(color)
                                .radius(step * 0.5),
                        );
                    }
                }
            });
    });
}
