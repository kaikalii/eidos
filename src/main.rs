#![allow(unstable_name_collisions)]

mod controls;
mod error;
mod field;
mod function;
mod plot;
mod runtime;
mod value;
mod world;

use eframe::egui::*;
use world::World;

pub use {error::*, field::*, function::*, runtime::*, value::*, world::*};

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
                world: World::default(),
            })
        }),
    );
}

struct Game {
    world: World,
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            self.world.ui(ui);
        });
        ctx.request_repaint();
    }
}
