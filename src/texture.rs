use eframe::{egui::*, epaint::mutex::Mutex};
use once_cell::sync::Lazy;

pub struct Textures {
    pub circle_gradient: TextureHandle,
}

pub fn textures<T>(f: impl FnOnce(&Textures) -> T) -> T {
    f(TEXTURES.lock().as_ref().unwrap())
}

static TEXTURES: Lazy<Mutex<Option<Textures>>> = Lazy::new(Default::default);

pub fn load_textures(ctx: &Context) {
    *TEXTURES.lock() = Some(Textures {
        circle_gradient: load_texture(
            ctx,
            "circle_gradient",
            include_bytes!("../resources/textures/circle_gradient.png"),
        ),
    });
}

fn load_texture(ctx: &Context, name: &str, data: &[u8]) -> TextureHandle {
    let image = image::load_from_memory(data).unwrap().into_rgba8();
    let image_data = ColorImage {
        size: [image.width() as usize, image.height() as usize],
        pixels: image
            .pixels()
            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect(),
    };
    ctx.load_texture(name, image_data, TextureOptions::default())
}
