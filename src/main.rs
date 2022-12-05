#![allow(unstable_name_collisions)]

mod controls;
mod plot;
mod sva;
mod world;

use eframe::egui::*;
use sva::Sva;
use world::World;

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
                world: World::default(),
            })
        }),
    );
}

struct Game {
    sva: Sva,
    world: World,
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            self.sva.ui(ui);
            self.world.ui(ui);
        });
        ctx.request_repaint();
    }
}
