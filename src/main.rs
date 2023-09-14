mod color;
mod conduit;
mod controls;
mod dialog;
mod error;
mod field;
mod function;
mod game;
mod image;
mod main_menu;
mod math;
mod new_game;
mod npc;
mod object;
mod person;
mod physics;
mod player;
mod plot;
mod stack;
mod texture;
mod utils;
mod word;
mod world;

use dialog::DIALOG_SCENES;
use eframe::egui::*;
use game::Game;
use main_menu::main_menu;
use new_game::NewGame;
use npc::NPCS;
use object::{OBJECTS, PLACES};
use once_cell::sync::Lazy;
use player::{Gender, Player};
use texture::load_textures;

fn main() {
    // Load resources
    Lazy::force(&DIALOG_SCENES);
    Lazy::force(&OBJECTS);
    Lazy::force(&PLACES);
    Lazy::force(&NPCS);
    // Enable profiling
    puffin::set_scopes_on(cfg!(all(feature = "profile", not(debug_assertions))));
    // Run
    eframe::run_native(
        "Eidos",
        eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(1280.0, 800.0)),
            ..Default::default()
        },
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;
            ctx.set_pixels_per_point(1.5);
            load_textures(ctx);
            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert(
                "emoji".into(),
                FontData::from_static(include_bytes!("../resources/NotoEmoji-Regular.ttf")),
            );
            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .push("emoji".into());
            ctx.set_fonts(fonts);

            Box::new(if cfg!(feature = "title") {
                GameState::MainMenu
            } else {
                GameState::Game(Game::new(Player::new("Kai".into(), Gender::Male)).into())
            })
        }),
    )
    .unwrap();
}

pub enum GameState {
    MainMenu,
    NewGame(NewGame),
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
        let screen_size = ctx.input(|input| input.screen_rect.size());
        let window_size = screen_size * ctx.pixels_per_point();
        let ppp_scale = match self {
            GameState::NewGame(_) => 2.0,
            _ => 1.0,
        };
        let ppp_divider = 700.0 / ppp_scale;
        let target_ppp = ((window_size.x * window_size.y).sqrt() / ppp_divider)
            .clamp(1.2 * ppp_scale, 3.0 * ppp_scale);
        if (target_ppp - ctx.pixels_per_point()).abs() > 0.001 {
            ctx.set_pixels_per_point(target_ppp);
        }

        let new_state = match self {
            GameState::MainMenu => main_menu(ctx),
            GameState::NewGame(new_game) => new_game.show(ctx),
            GameState::Game(game) => game.show(ctx),
            GameState::Quit => {
                frame.close();
                return;
            }
        };

        if let Some(new_state) = new_state {
            *self = new_state;
        }

        ctx.request_repaint();
    }
}
