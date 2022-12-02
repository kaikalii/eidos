mod cad;
mod render;

use cad::Cad;
use eframe::egui::*;

fn main() {
    eframe::run_native(
        "Eidos",
        Default::default(),
        Box::new(|_| {
            Box::new(Game {
                cad: Cad::default(),
            })
        }),
    )
}

struct Game {
    cad: Cad,
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {}
}
