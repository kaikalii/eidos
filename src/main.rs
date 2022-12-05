#![allow(unstable_name_collisions)]

mod controls;
mod render;
mod sva;

use eframe::egui::*;
use sva::Sva;

fn main() {
    eframe::run_native(
        "Eidos",
        eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(1000.0, 500.0)),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(Game {
                sva: Sva::default(),
            })
        }),
    );
}

struct Game {
    sva: Sva,
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.sva.ui(ui));
    }
}
