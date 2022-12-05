#![allow(unstable_name_collisions)]

pub mod controls;
pub mod error;
pub mod field;
pub mod function;
pub mod game;
pub mod math;
pub mod plot;
pub mod runtime;
pub mod value;
pub mod world;

use eframe::egui::*;
use game::Game;

fn main() {
    eframe::run_native(
        "Eidos",
        eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(1000.0, 500.0)),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(Game::default())
        }),
    );
}
