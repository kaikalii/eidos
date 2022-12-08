#![allow(unstable_name_collisions)]

pub mod controls;
pub mod dialog;
pub mod error;
pub mod field;
pub mod function;
pub mod game;
pub mod math;
pub mod physics;
pub mod player;
pub mod plot;
pub mod stack;
pub mod word;
pub mod world;

use dialog::DIALOG_SCENES;
use eframe::egui::*;
use game::Game;

fn main() {
    once_cell::sync::Lazy::force(&DIALOG_SCENES);
    puffin::set_scopes_on(cfg!(all(feature = "profile", not(debug_assertions))));
    eframe::run_native(
        "Eidos",
        eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(1280.0, 800.0)),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(1.5);
            Box::new(Game::default())
        }),
    );
}
