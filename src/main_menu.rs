use std::f64;

use eframe::egui::{plot::*, *};
use once_cell::sync::Lazy;
use rand::prelude::*;

use crate::{new_game::NewGame, plot::time, GameState};

const LOGO_ASCII: &str = "
   ▄████████   ▄█   ████████▄    ▄██████▄      ▄████████
  ███    ███  ███   ███   ▀███  ███    ███    ███    ███
  ███    █▀   ███▌  ███    ███  ███    ███    ███    █▀ 
 ▄███▄▄▄      ███▌  ███    ███  ███    ███    ███       
▀▀███▀▀▀      ███▌  ███    ███  ███    ███  ▀███████████
  ███    █▄   ███   ███    ███  ███    ███           ███
  ███    ███  ███   ███   ▄███  ███    ███     ▄█    ███
  ██████████  █▀    ████████▀    ▀██████▀    ▄████████▀ 
";

const SPACING: f32 = 10.0;
const RADIUS: f32 = 2.0;
const VARIANCE: f32 = 0.5;
const OFFSET: f32 = SPACING * VARIANCE;

struct Logo {
    points: Vec<Pos2>,
    max: Pos2,
}

static LOGO: Lazy<Logo> = Lazy::new(|| {
    let mut points = Vec::new();
    let mut rng = thread_rng();
    let mut max = Pos2::ZERO;
    for (i, line) in LOGO_ASCII
        .split('\n')
        .rev()
        .filter(|s| !s.trim().is_empty())
        .enumerate()
    {
        for (j, char) in line.chars().enumerate() {
            if !char.is_whitespace() {
                for _ in 0..20 {
                    let x = SPACING * (j as f32) + rng.gen_range(-OFFSET..=OFFSET);
                    let y = 1.5 * SPACING * (i as f32) + rng.gen_range(-OFFSET..=OFFSET);
                    points.push(pos2(x, y));
                    max.x = max.x.max(x);
                    max.y = max.y.max(y);
                }
            }
        }
    }
    Logo { points, max }
});

pub fn main_menu(ctx: &Context) -> Option<GameState> {
    CentralPanel::default().show(ctx, main_menu_ui).inner
}

fn main_menu_ui(ui: &mut Ui) -> Option<GameState> {
    logo_ui(ui);
    let mut res = None;
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.y = 20.0;
        if ui.button(RichText::new("New Game").heading()).clicked() {
            res = Some(GameState::NewGame(NewGame::default()));
        }
        if ui.button(RichText::new("Quit").heading()).clicked() {
            res = Some(GameState::Quit);
        }
    });
    res
}

fn logo_ui(ui: &mut Ui) {
    Plot::new("logo")
        .view_aspect(3.0)
        .data_aspect(1.0)
        .allow_zoom(false)
        .allow_scroll(false)
        .show_background(false)
        .show_axes([false; 2])
        .show_x(false)
        .show_y(false)
        .include_x(-SPACING)
        .include_x(LOGO.max.x + SPACING)
        .include_y(-SPACING)
        .include_x(LOGO.max.y + SPACING)
        .show(ui, |plot_ui| {
            let time = time();
            let circle_pos = pos2(
                LOGO.max.x * ((time * 0.9).sin() as f32 * 0.5 + 0.5),
                LOGO.max.y * ((time * 3.3).sin() as f32 * 0.5 + 0.5),
            );
            let circle_radius = LOGO.max.y * 0.15;
            let mut rng = SmallRng::seed_from_u64(0);
            let mut circled = Vec::new();
            let mut uncircled = Vec::with_capacity(LOGO.points.len());
            for pos in &LOGO.points {
                let wiggled = pos2(
                    pos.x + (rng.gen::<f64>() * f64::consts::TAU * time).sin() as f32 * OFFSET,
                    pos.y + (rng.gen::<f64>() * f64::consts::TAU * time).sin() as f32 * OFFSET,
                );
                let point = PlotPoint::new(wiggled.x, wiggled.y);
                if pos.distance(circle_pos) < circle_radius {
                    circled.push(point);
                } else {
                    uncircled.push(point);
                }
            }
            plot_ui.points(
                Points::new(PlotPoints::Owned(uncircled))
                    .color(Color32::LIGHT_BLUE)
                    .radius(RADIUS),
            );
            plot_ui.points(
                Points::new(PlotPoints::Owned(circled))
                    .color(Color32::GOLD)
                    .radius(RADIUS),
            );
        });
}
