use std::{
    env::{current_dir, current_exe},
    path::PathBuf,
    process::exit,
};

use eframe::egui::*;

pub fn resources_path() -> PathBuf {
    let mut path = current_dir()
        .map_err(fatal_error)
        .unwrap()
        .join("resources");
    if path.exists() {
        return path;
    }
    path = current_exe()
        .map_err(fatal_error)
        .unwrap()
        .parent()
        .unwrap()
        .join("resources");
    if path.exists() {
        return path;
    }
    fatal_error("Unable to find resources directory")
}

pub fn fatal_error(message: impl ToString) -> ! {
    fatal_error_impl(message.to_string())
}
fn fatal_error_impl(message: String) -> ! {
    eframe::run_native(
        "Error",
        eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(400.0, 300.0)),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(FatalErrorWindow { message })
        }),
    );
    exit(1)
}

struct FatalErrorWindow {
    message: String,
}

impl eframe::App for FatalErrorWindow {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("There was an error initializing the game");
            ui.label(&self.message);
        });
    }
}
