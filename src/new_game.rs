use eframe::egui::*;

use crate::{
    game::Game,
    player::{Gender, Player},
    GameState,
};

pub struct NewGame {
    pub gender: Gender,
    pub name: String,
}

impl Default for NewGame {
    fn default() -> Self {
        NewGame {
            gender: Gender::Male,
            name: String::new(),
        }
    }
}

impl NewGame {
    pub fn show(&mut self, ctx: &Context) -> Result<(), GameState> {
        let mut res = Ok(());
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back").clicked() {
                res = Err(GameState::MainMenu);
            }
            ui.add_space((ui.available_height() - 100.0) / 2.0);
            ui.spacing_mut().item_spacing.y = 20.0;
            Grid::new(()).show(ui, |ui| {
                // Name
                ui.label("Name");
                let name_res = TextEdit::singleline(&mut self.name)
                    .desired_width(100.0)
                    .show(ui);
                if name_res.response.changed() {
                    self.name = self
                        .name
                        .chars()
                        .next()
                        .into_iter()
                        .flat_map(char::to_uppercase)
                        .chain(self.name.chars().skip(1))
                        .collect();
                }
                ui.end_row();

                // Gender
                ui.label("Gender");
                ui.horizontal(|ui| {
                    for (gender, symbol, hover_text) in [
                        (Gender::Male, "♂", "uses he/him/his"),
                        (Gender::Female, "♀", "uses she/her/hers"),
                        (Gender::Enby, "⚧", "uses they/them/their"),
                    ] {
                        ui.selectable_value(
                            &mut self.gender,
                            gender,
                            RichText::new(symbol).heading(),
                        )
                        .on_hover_text(hover_text);
                    }
                });
                ui.end_row();

                // Start
                if ui
                    .add_enabled(!self.name.is_empty(), Button::new("Start"))
                    .clicked()
                {
                    res = Err(GameState::Game(
                        Game::new(Player::new(self.name.clone(), self.gender)).into(),
                    ));
                }
            });
        });
        res
    }
}
