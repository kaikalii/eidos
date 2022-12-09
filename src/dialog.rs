use std::{
    collections::HashMap,
    env::{current_dir, current_exe},
    fs,
    path::PathBuf,
    process::exit,
};

use anyhow::{anyhow, bail};
use chumsky::{prelude::*, text::whitespace};
use eframe::egui::*;
use enum_iterator::all;
use indexmap::IndexMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::{field::GenericInputFieldKind, game::Game, player::Gender, word::Word, world::World};

pub fn fatal_error(message: impl ToString) -> ! {
    fatal_error_impl(message.to_string())
}
fn fatal_error_impl(message: String) -> ! {
    eframe::run_native(
        "Error",
        eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(400.0, 300.0)),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(FatalErrorWindow { message })
        }),
    );
    exit(1)
}

pub fn resources_path() -> PathBuf {
    let mut path = current_dir()
        .map_err(fatal_error)
        .unwrap()
        .join("resources");
    if path.exists() {
        return path;
    }
    path = current_exe()
        .map_err(fatal_error)
        .unwrap()
        .parent()
        .unwrap()
        .join("resources");
    if path.exists() {
        return path;
    }
    fatal_error("Unable to find resources directory")
}

struct FatalErrorWindow {
    message: String,
}

impl eframe::App for FatalErrorWindow {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("There was an error initializing the game");
            ui.label(&self.message);
        });
    }
}

type DialogScenes = HashMap<String, DialogScene<Vec<DialogFragment>>>;

pub static DIALOG_SCENES: Lazy<DialogScenes> =
    Lazy::new(|| load_scenes().map_err(fatal_error).unwrap());

fn load_scenes() -> anyhow::Result<DialogScenes> {
    let mut map = HashMap::new();
    for entry in fs::read_dir(resources_path().join("dialog"))
        .map_err(|e| fatal_error(format!("Unable to open dialog directory: {e}")))
        .unwrap()
    {
        let entry = entry.unwrap();
        if entry.file_type()?.is_file() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let yaml = fs::read_to_string(&path)?;
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                let scene: DialogScene<SerializedLine> = serde_yaml::from_str(&yaml)
                    .map_err(|e| anyhow!("Unable to read {name} dialog: {e}"))?;
                if scene.nodes.is_empty() {
                    continue;
                }
                let scene: DialogScene<Vec<DialogFragment>> = scene
                    .try_into()
                    .map_err(|e| anyhow!("Error parsing fragment in {name}: {e}"))?;
                for (node_name, node) in &scene.nodes {
                    for child_name in node.children.keys() {
                        if !scene.nodes.contains_key(child_name) {
                            bail!("In {name} scene, node {node_name}'s child {child_name} does not exist")
                        }
                    }
                }
                map.insert(name, scene);
            }
        }
    }
    Ok(map)
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct DialogScene<T> {
    pub nodes: IndexMap<String, DialogNode<T>>,
}

#[derive(Debug, Deserialize)]
pub struct DialogNode<T> {
    #[serde(default = "Vec::new")]
    pub lines: Vec<Line<T>>,
    #[serde(default = "IndexMap::new")]
    pub children: IndexMap<String, T>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Line<T> {
    Command(DialogCommand),
    Text(T),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SerializedLine {
    String(String),
    Catch(serde_yaml::Value),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogCommand {
    Speaker(Option<String>),
    RevealWord(Word),
    RevealAllWords,
    RevealManaBar,
    RevealField(GenericInputFieldKind),
}

#[derive(Debug)]
pub enum DialogFragment {
    String(String),
    Variable(DialogVariable),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DialogVariable {
    Gendered(GenderedWord),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenderedWord {
    Sub,
    Obj,
    Pos,
    Reflexive,
    SubIs,
    SubWas,
    Subs,
    Has,
    Nibling,
}

impl TryFrom<DialogScene<SerializedLine>> for DialogScene<Vec<DialogFragment>> {
    type Error = anyhow::Error;
    fn try_from(scene: DialogScene<SerializedLine>) -> Result<Self, Self::Error> {
        let parser = line_parser();
        let mut nodes = IndexMap::new();
        for (name, node) in scene.nodes {
            let mut lines = Vec::new();
            for line in node.lines {
                lines.push(match line {
                    Line::Text(SerializedLine::String(text)) => {
                        parser.parse(text).map_err(|mut e| anyhow!(e.remove(0)))?
                    }
                    Line::Command(com) => Line::Command(com),
                    Line::Text(SerializedLine::Catch(value)) => {
                        bail!(
                            "`{}` is not a valid command",
                            serde_yaml::to_string(&value).unwrap()[3..].trim()
                        )
                    }
                });
            }
            let mut children = IndexMap::new();
            for (name, text) in node.children {
                if let SerializedLine::String(text) = text {
                    if let Line::Text(text) =
                        parser.parse(text).map_err(|mut e| anyhow!(e.remove(0)))?
                    {
                        children.insert(name, text);
                    }
                }
            }
            nodes.insert(name, DialogNode { lines, children });
        }
        Ok(DialogScene { nodes })
    }
}

trait FragmentParser<T>: Parser<char, T, Error = Simple<char>> {}

impl<P, T> FragmentParser<T> for P where P: Parser<char, T, Error = Simple<char>> {}

fn line_parser() -> impl FragmentParser<Line<Vec<DialogFragment>>> {
    fragments().map(Line::Text).then_ignore(end())
}

fn fragments() -> impl FragmentParser<Vec<DialogFragment>> {
    choice((
        variable().map(DialogFragment::Variable),
        string_fragment().map(DialogFragment::String),
    ))
    .repeated()
}

fn bracketed<T>(inner: impl FragmentParser<T>) -> impl FragmentParser<T> {
    just('(')
        .ignore_then(inner.padded_by(whitespace()))
        .then_ignore(just(')'))
}

fn string_fragment() -> impl FragmentParser<String> {
    none_of("()").repeated().at_least(1).collect()
}

fn variable() -> impl FragmentParser<DialogVariable> {
    bracketed(string_fragment().try_map(|string, span| {
        match serde_yaml::from_str::<DialogVariable>(&string) {
            Ok(command) => Ok(command),
            Err(e) => Err(Simple::<char>::custom(span, e)),
        }
    }))
}

pub struct DialogState {
    scene: String,
    node: String,
    line: usize,
    character: usize,
    speaker: Option<String>,
}

const DIALOG_SPEED: usize = 4;

impl Game {
    pub fn set_dialog(&mut self, scene_name: &str) {
        let scene = &DIALOG_SCENES[scene_name];
        let dialog = DialogState {
            scene: scene_name.into(),
            node: scene.nodes.first().unwrap().0.clone(),
            line: 0,
            character: 0,
            speaker: None,
        };
        self.ui_state.dialog = Some(dialog);
    }
    pub fn dialog_ui(&mut self, ui: &mut Ui) {
        if self.ui_state.dialog.is_none() {
            return;
        }
        ui.group(|ui| self.dialog_ui_impl(ui));
    }
    fn progress_dialog(&mut self) {
        let Some(dialog) = &mut self.ui_state.dialog else {
            return;
        };
        let scene = &DIALOG_SCENES[&dialog.scene];
        let node = &scene.nodes[&dialog.node];

        if dialog.line < node.lines.len().saturating_sub(1) {
            dialog.line += 1;
            dialog.character = 0;
        } else {
            let node_index = scene.nodes.get_index_of(&dialog.node).unwrap();
            if let Some((node_name, _)) = scene.nodes.get_index(node_index + 1) {
                dialog.node = node_name.clone();
                dialog.line = 0;
                dialog.character = 0;
            } else if node.children.is_empty() {
                self.ui_state.dialog = None;
            }
        }
    }
    fn dialog_ui_impl(&mut self, ui: &mut Ui) {
        // Get dialog scene data
        let Some(dialog) = &mut self.ui_state.dialog else {
            return;
        };
        let scene = &DIALOG_SCENES[&dialog.scene];
        let node = &scene.nodes[&dialog.node];
        if node.lines.is_empty() {
            self.progress_dialog();
            self.dialog_ui_impl(ui);
            return;
        }
        let line = &node.lines[dialog.line];
        match line {
            Line::Text(fragments) => {
                // Space the group
                ui.allocate_at_least(vec2(ui.max_rect().width(), 0.0), Sense::hover());
                let line_text = self.world.format_dialog_fragments(fragments);
                let char_indices = line_text.char_indices().collect_vec();
                let char_index = dialog.character / DIALOG_SPEED;
                ui.horizontal(|ui| {
                    // Show speaker
                    if let Some(speaker) = &dialog.speaker {
                        ui.heading(format!("{speaker}:"));
                    }
                    // Show line text
                    if !line_text.is_empty() {
                        let line_text = &line_text[..=char_indices[char_index].0];
                        ui.horizontal_wrapped(|ui| ui.heading(line_text));
                    }
                });
                // Show continue or choices
                let max_dialog_char = (char_indices.len().saturating_sub(1)) * DIALOG_SPEED;
                dialog.character = (dialog.character + 1).min(max_dialog_char);
                let mut next = || {
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        ui.button("Next").clicked()
                    })
                    .inner
                };
                if dialog.character < max_dialog_char {
                    // Revealing the text
                    if next() {
                        dialog.character = max_dialog_char;
                    }
                } else if node.children.is_empty() {
                    // No choices
                    if line_text.is_empty() || next() {
                        self.progress_dialog();
                    }
                } else {
                    // Choices
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        for (name, fragments) in &node.children {
                            if ui
                                .button(self.world.format_dialog_fragments(fragments))
                                .clicked()
                            {
                                dialog.node = name.clone();
                                dialog.line = 0;
                                dialog.character = 0;
                            }
                        }
                    });
                }
            }
            Line::Command(command) => {
                let progression = &mut self.world.player.progression;
                match command {
                    DialogCommand::Speaker(speaker) => dialog.speaker = speaker.clone(),
                    DialogCommand::RevealWord(word) => {
                        progression.known_words.insert(*word);
                    }
                    DialogCommand::RevealAllWords => progression.known_words.extend(all::<Word>()),
                    DialogCommand::RevealManaBar => progression.mana_bar = true,
                    DialogCommand::RevealField(kind) => {
                        progression.known_fields.insert(*kind);
                    }
                }
                self.progress_dialog();
                self.dialog_ui_impl(ui);
            }
        }
    }
}

impl World {
    fn format_dialog_fragments(&self, fragments: &[DialogFragment]) -> String {
        let mut formatted = String::new();
        for (i, frag) in fragments.iter().enumerate() {
            let s = match frag {
                DialogFragment::String(s) => s,
                DialogFragment::Variable(var) => match var {
                    DialogVariable::Gendered(pronoun) => match (pronoun, self.player.gender) {
                        (GenderedWord::Sub, Gender::Male) => "he",
                        (GenderedWord::Obj, Gender::Male) => "him",
                        (GenderedWord::Pos, Gender::Male) => "his",
                        (GenderedWord::Reflexive, Gender::Male) => "himself",
                        (GenderedWord::SubIs, Gender::Male) => "he is",
                        (GenderedWord::SubWas, Gender::Male) => "he was",
                        (GenderedWord::Subs, Gender::Male) => "he's",
                        (GenderedWord::Has, Gender::Male) => "has",
                        (GenderedWord::Nibling, Gender::Male) => "nephew",
                        (GenderedWord::Sub, Gender::Female) => "she",
                        (GenderedWord::Obj, Gender::Female) => "her",
                        (GenderedWord::Pos, Gender::Female) => "her",
                        (GenderedWord::Reflexive, Gender::Female) => "herself",
                        (GenderedWord::SubIs, Gender::Female) => "she is",
                        (GenderedWord::SubWas, Gender::Female) => "she was",
                        (GenderedWord::Subs, Gender::Female) => "she's",
                        (GenderedWord::Has, Gender::Female) => "has",
                        (GenderedWord::Nibling, Gender::Female) => "niece",
                        (GenderedWord::Sub, Gender::Enby) => "they",
                        (GenderedWord::Obj, Gender::Enby) => "them",
                        (GenderedWord::Pos, Gender::Enby) => "their",
                        (GenderedWord::Reflexive, Gender::Enby) => "themselves",
                        (GenderedWord::SubIs, Gender::Enby) => "they are",
                        (GenderedWord::SubWas, Gender::Enby) => "they were",
                        (GenderedWord::Subs, Gender::Enby) => "they're",
                        (GenderedWord::Has, Gender::Enby) => "have",
                        (GenderedWord::Nibling, Gender::Enby) => "nieph",
                    },
                },
            };
            if i == 0
                || formatted.trim().ends_with(['.', '?', '!'])
                || formatted.trim().ends_with(".\"")
                || formatted.trim().ends_with("?\"")
                || formatted.trim().ends_with("!\"")
            {
                formatted.extend(s.chars().next().into_iter().flat_map(|c| c.to_uppercase()));
                formatted.extend(s.chars().skip(1));
            } else {
                formatted.push_str(s);
            }
        }
        formatted
    }
}
