use std::{collections::HashMap, f64};

use eframe::{
    egui::*,
    epaint::{mutex::Mutex, util::hash},
};
use image::RgbaImage;
use once_cell::sync::Lazy;
use rand::prelude::*;
use rayon::prelude::*;

use crate::{
    color::Color,
    plot::{time, Plottable},
    utils::resources_path,
};

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

#[derive(Clone, Copy)]
pub enum ImagePlotKind {
    Portrait(bool),
    Background,
}

pub fn image_plot(ui: &mut Ui, name: &str, max_size: Vec2, kind: ImagePlotKind) {
    puffin::profile_function!();
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
            ImagePlotKind::Portrait(_) => 3.0,
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
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let max_i = (size.x / step) as usize;
        let max_j = (size.y / step) as usize;
        let mut points = Vec::with_capacity(max_i * max_j);
        points.par_extend((0..max_i).par_bridge().flat_map(|i| {
            let x = i as f32 * step;
            (0..max_j).par_bridge().filter_map(move |j| {
                let y = j as f32 * step;
                let mut rng = SmallRng::seed_from_u64(hash((i, j)));
                let mut color = Color::from(*image.get_pixel(
                    (x / size.x * image.width() as f32) as u32,
                    (y / size.y * image.height() as f32) as u32,
                ));
                let dx = wiggle_range * (time + rng.gen_range(0.0..=f64::consts::TAU)).sin() as f32;
                let dy = wiggle_range * (time + rng.gen_range(0.0..=f64::consts::TAU)).sin() as f32;
                let dropoff = match kind {
                    ImagePlotKind::Portrait(_) => 1.0,
                    ImagePlotKind::Background => {
                        let dist_from_center =
                            (Vec2::new(x + dx, y + dy) - max_size * 0.5).length();
                        1.0 - dist_from_center / ((size.x + size.y) * 0.25)
                    }
                };
                color.a *= alpha * dropoff;
                if color.a < 1.0 / 255.0 {
                    return None;
                }
                let color_mul = color_mul * dropoff;
                Some((
                    pos2(x, y) + rect.left_top().to_vec2(),
                    vec2(dx, dy),
                    color * color_mul,
                ))
            })
        }));
        points.par_sort_by(|(a, ..), (b, ..)| {
            <f32 as Plottable>::cmp(&a.x, &b.x).then_with(|| <f32 as Plottable>::cmp(&a.y, &b.y))
        });
        let painter = ui.painter();
        for (point, offset, color) in points {
            painter.circle_filled(point + offset, step * 0.5, color);
        }
    });
}
