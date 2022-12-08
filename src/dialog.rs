use std::{
    collections::HashMap,
    env::{current_dir, current_exe},
    fs,
    iter::once,
    path::PathBuf,
    process::exit,
};

use anyhow::anyhow;
use chumsky::{prelude::*, text::whitespace};
use eframe::egui::*;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::{field::GenericFieldKind, word::Word};

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

type DialogScenes = HashMap<String, DialogScene<Line>>;

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
                let scene: DialogScene<String> = serde_yaml::from_str(&yaml)
                    .map_err(|e| anyhow!("Unable to read {name} dialog: {e}"))?;
                if scene.nodes.is_empty() {
                    continue;
                }
                let scene: DialogScene<Line> = scene
                    .try_into()
                    .map_err(|e| anyhow!("Error parsing fragment in {name}: {e}"))?;
                map.insert(name, scene);
            }
        }
    }
    Ok(map)
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct DialogScene<T> {
    pub nodes: IndexMap<String, DialogNode<T>>,
}

#[derive(Deserialize)]
pub struct DialogNode<T> {
    pub lines: Vec<T>,
    #[serde(default = "IndexMap::new")]
    pub children: IndexMap<String, T>,
}

pub struct Line {
    pub speaker: Option<String>,
    pub fragments: Vec<DialogFragment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogCommand {
    RevealWord(Word),
    RevealAllWords,
    RevealManaBar,
    RevealField(GenericFieldKind),
}

pub enum DialogFragment {
    String(String),
    Command(DialogCommand),
}

impl TryFrom<DialogScene<String>> for DialogScene<Line> {
    type Error = anyhow::Error;
    fn try_from(scene: DialogScene<String>) -> Result<Self, Self::Error> {
        let parser = line_parser();
        let mut nodes = IndexMap::new();
        for (name, node) in scene.nodes {
            if node.lines.is_empty() {
                continue;
            }
            let mut lines = Vec::new();
            for line in node.lines {
                lines.push(parser.parse(line).map_err(|mut e| anyhow!(e.remove(0)))?);
            }
            let mut children = IndexMap::new();
            for (name, text) in node.children {
                let text = parser.parse(text).map_err(|mut e| anyhow!(e.remove(0)))?;
                children.insert(name, text);
            }
            nodes.insert(name, DialogNode { lines, children });
        }
        Ok(DialogScene { nodes })
    }
}

trait FragmentParser<T>: Parser<char, T, Error = Simple<char>> {}

impl<P, T> FragmentParser<T> for P where P: Parser<char, T, Error = Simple<char>> {}

fn line_parser() -> impl FragmentParser<Line> {
    choice((start_with_command(), start_with_speaker())).then_ignore(end())
}

fn start_with_speaker() -> impl FragmentParser<Line> {
    let speaker = bracketed(string_fragment())
        .then_ignore(whitespace())
        .or_not();
    speaker
        .then(fragments())
        .map(|(speaker, fragments)| Line { speaker, fragments })
}

fn start_with_command() -> impl FragmentParser<Line> {
    command().then(fragments()).map(|(command, frags)| Line {
        speaker: None,
        fragments: once(DialogFragment::Command(command))
            .chain(frags)
            .collect(),
    })
}

fn fragments() -> impl FragmentParser<Vec<DialogFragment>> {
    choice((
        string_fragment().map(DialogFragment::String),
        command().map(DialogFragment::Command),
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

fn command() -> impl FragmentParser<DialogCommand> {
    bracketed(string_fragment().try_map(|string, span| {
        match serde_yaml::from_str::<DialogCommand>(&string) {
            Ok(command) => Ok(command),
            Err(e) => Err(Simple::<char>::custom(span, e)),
        }
    }))
}

#[test]
fn parse_fragment() {
    line_parser().parse("Hello!").unwrap();
}

#[test]
fn parse_command() {
    serde_yaml::from_str::<DialogCommand>("reveal_word: Sa").unwrap();
}
