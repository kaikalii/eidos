use std::f64;

use eframe::egui::{plot::*, *};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rand::prelude::*;

use crate::{game::Game, plot::time, GameState};

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

const SPACING: f64 = 10.0;
const RADIUS: f64 = 2.0;
const VARIANCE: f64 = 0.5;
const OFFSET: f64 = SPACING * VARIANCE;

struct Logo {
    points: Vec<PlotPoint>,
    max: PlotPoint,
}

static LOGO: Lazy<Logo> = Lazy::new(|| {
    let mut points = Vec::new();
    let mut rng = thread_rng();
    let mut max = PlotPoint::new(0.0, 0.0);
    for (i, line) in LOGO_ASCII
        .split('\n')
        .rev()
        .filter(|s| !s.trim().is_empty())
        .enumerate()
    {
        for (j, char) in line.chars().enumerate() {
            if !char.is_whitespace() {
                for _ in 0..20 {
                    let x = SPACING * (j as f64) + rng.gen_range(-OFFSET..=OFFSET);
                    let y = 1.5 * SPACING * (i as f64) + rng.gen_range(-OFFSET..=OFFSET);
                    points.push(PlotPoint::new(x, y));
                    max.x = max.x.max(x);
                    max.y = max.y.max(y);
                }
            }
        }
    }
    Logo { points, max }
});

pub fn main_menu(ctx: &Context) -> Result<(), GameState> {
    CentralPanel::default().show(ctx, main_menu_ui).inner
}

fn main_menu_ui(ui: &mut Ui) -> Result<(), GameState> {
    logo_ui(ui);
    let mut res = Ok(());
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.y = 20.0;
        if ui.button(RichText::new("New Game").heading()).clicked() {
            res = Err(GameState::Game(Game::default().into()));
        }
        if ui.button(RichText::new("Quit").heading()).clicked() {
            res = Err(GameState::Quit);
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
            let mut rng = SmallRng::seed_from_u64(0);
            let points = LOGO
                .points
                .iter()
                .map(|p| {
                    PlotPoint::new(
                        p.x + (rng.gen::<f64>() * f64::consts::TAU * time).sin() * OFFSET,
                        p.y + (rng.gen::<f64>() * f64::consts::TAU * time).sin() * OFFSET,
                    )
                })
                .collect_vec();
            plot_ui.points(
                Points::new(PlotPoints::Owned(points))
                    .color(Color32::LIGHT_BLUE)
                    .radius(RADIUS as f32),
            );
        });
}
