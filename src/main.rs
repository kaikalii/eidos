#![allow(unstable_name_collisions)]

mod controls;
mod dialog;
mod error;
mod field;
mod function;
mod game;
mod main_menu;
mod math;
mod physics;
mod player;
mod plot;
mod stack;
mod word;
mod world;

use dialog::DIALOG_SCENES;
use eframe::egui::*;
use game::Game;
use main_menu::main_menu;

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
            Box::new(if cfg!(debug_assertions) {
                GameState::Game(Game::default().into())
            } else {
                GameState::MainMenu
            })
        }),
    );
}

pub enum GameState {
    MainMenu,
    Game(Box<Game>),
    Quit,
}

impl eframe::App for GameState {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // Profiler
        #[cfg(all(feature = "profile", not(debug_assertions)))]
        Window::new("Profiler").collapsible(true).show(ctx, |ui| {
            puffin_egui::profiler_ui(ui);
        });
        puffin::GlobalProfiler::lock().new_frame();

        // Resize
        let screen_size = ctx.input().screen_rect.size();
        let window_size = screen_size * ctx.pixels_per_point();
        let target_ppp = ((window_size.x * window_size.y).sqrt() / 701.0).clamp(1.2, 3.0);
        if (target_ppp - ctx.pixels_per_point()).abs() > 0.001 {
            ctx.set_pixels_per_point(target_ppp);
        }

        let new_state = match self {
            GameState::MainMenu => main_menu(ctx),
            GameState::Game(game) => game.show(ctx),
            GameState::Quit => {
                frame.close();
                Ok(())
            }
        };

        if let Err(new_state) = new_state {
            *self = new_state;
        }

        ctx.request_repaint();
    }
}
