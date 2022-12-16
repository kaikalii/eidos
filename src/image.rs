#![allow(dead_code)]

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

pub fn image_plot(ui: &mut Ui, name: &str, max_size: Vec2) {
    use_image(name, |image| {
        let image_aspect = image.width() as f32 / image.height() as f32;
        let max_size_aspect = max_size.x / max_size.y;
        let size = if image_aspect > max_size_aspect {
            Vec2::new(max_size.x, max_size.x / image_aspect)
        } else {
            Vec2::new(max_size.y * image_aspect, max_size.y)
        };
        let step = 5.0;
        let wiggle_range = step * 0.5;
        let time = time();
        Plot::new(name)
            .width(size.x)
            .height(size.y)
            .show_axes([false; 2])
            .show_background(false)
            .show_x(false)
            .show_y(false)
            .data_aspect(1.0)
            .view_aspect(image_aspect)
            .show(ui, |plot_ui| {
                let mut rng = SmallRng::seed_from_u64(0);
                for i in 0..(size.x / step) as usize {
                    for j in 0..(size.y / step) as usize {
                        let x = i as f32 * step;
                        let y = j as f32 * step;
                        let color = image.get_pixel(
                            (x / size.x * image.width() as f32) as u32,
                            (y / size.y * image.height() as f32) as u32,
                        );
                        let dx = wiggle_range
                            * (time + rng.gen_range(0.0..=f64::consts::TAU)).sin() as f32;
                        let dy = wiggle_range
                            * (time + rng.gen_range(0.0..=f64::consts::TAU)).sin() as f32;
                        if color[3] == 0 {
                            continue;
                        }
                        let color =
                            Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
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
