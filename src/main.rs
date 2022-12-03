#![allow(unstable_name_collisions)]

mod cad;
mod controls;
mod render;

use cad::Cad;
use eframe::egui::*;

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
                cad: Cad::default(),
            })
        }),
    );
}

struct Game {
    cad: Cad,
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| self.cad.ui(ui));
    }
}
