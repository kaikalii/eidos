use std::{borrow::Cow, collections::HashMap, fs};

use anyhow::{anyhow, bail};
use chumsky::{prelude::*, text::whitespace};
use eframe::{egui::*, epaint::ahash::HashSet};
use enum_iterator::all;
use indexmap::IndexMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::{
    field::InputFieldKind,
    game::Game,
    image::{image_plot, ImagePlotKind},
    player::Gender,
    utils::{fatal_error, resources_path},
    word::Word,
    world::World,
};

type DialogScenes = HashMap<String, DialogScene<DeserializedLine>>;

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
                let scene: DialogScene<DeserializedLine> = scene
                    .try_into()
                    .map_err(|e| anyhow!("Error parsing fragment in {name}: {e}"))?;
                for (node_name, node) in &scene.nodes {
                    validate_children(&name, &scene, node_name, &node.children)?;
                }
                map.insert(name, scene);
            }
        }
    }
    Ok(map)
}

fn validate_children(
    scene_name: &str,
    scene: &DialogScene<DeserializedLine>,
    node_name: &str,
    children: &NodeChildren<DeserializedLine>,
) -> anyhow::Result<()> {
    let child_nodes = match children {
        NodeChildren::Choices(choices) => choices.keys().collect_vec(),
        NodeChildren::Jump { jump } => vec![jump],
        NodeChildren::Condition { then, els, .. } => {
            validate_children(scene_name, scene, node_name, then)?;
            validate_children(scene_name, scene, node_name, els)?;
            Vec::new()
        }
        NodeChildren::Wait { then: node, .. } => vec![node],
        NodeChildren::List(list) => {
            for child in list {
                validate_children(scene_name, scene, node_name, child)?;
            }
            Vec::new()
        }
        NodeChildren::Next(_) => Vec::new(),
    };
    for child_name in child_nodes {
        if !scene.nodes.contains_key(child_name) {
            bail!("In {scene_name} scene, node {node_name}'s child {child_name} does not exist")
        }
    }
    Ok(())
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
    #[serde(default = "NodeChildren::default")]
    pub children: NodeChildren<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum NodeChildren<T> {
    Condition {
        #[serde(rename = "if")]
        condition: Condition,
        then: Box<Self>,
        #[serde(rename = "else")]
        els: Box<Self>,
    },
    Wait {
        #[serde(rename = "wait")]
        condition: WaitCondition,
        then: String,
    },
    Choices(IndexMap<String, Vec<T>>),
    Jump {
        jump: String,
    },
    Next(Vec<T>),
    List(Vec<Self>),
}

impl<T> Default for NodeChildren<T> {
    fn default() -> Self {
        NodeChildren::Choices(IndexMap::new())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    FieldKnown(InputFieldKind),
    Flag(String),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitCondition {
    KnowField(InputFieldKind),
    SayWord(Word),
    EmptyStack,
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
    Left(Option<Speaker>),
    Right(Option<Speaker>),
    Background(Option<String>),
    Speaker(Option<CurrentSpeaker>),
    RevealWord(Word),
    RevealAllWords,
    RevealManaBar,
    RevealFree,
    RevealConduit,
    RevealField(InputFieldKind),
    Set(String),
    Unset(String),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Speaker {
    Npc(String),
    Image { name: String, image: String },
}

impl Speaker {
    fn name(&self) -> &str {
        match self {
            Speaker::Npc(name) => name,
            Speaker::Image { name, .. } => name,
        }
    }
    fn image(&self) -> Cow<str> {
        match self {
            Speaker::Npc(name) => Cow::Owned(format!("{}.png", name)),
            Speaker::Image { image, .. } => image.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum CurrentSpeaker {
    Stranger { stranger: String },
    Npc(String),
}

impl CurrentSpeaker {
    fn name(&self) -> &str {
        match self {
            CurrentSpeaker::Stranger { stranger } => stranger,
            CurrentSpeaker::Npc(name) => name,
        }
    }
    fn display(&self) -> &str {
        match self {
            CurrentSpeaker::Stranger { .. } => "Stranger",
            CurrentSpeaker::Npc(name) => name,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DialogFragment {
    String(String),
    Variable(DialogVariable),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DialogVariable {
    Variable(Variable),
    Gendered(GenderedWord),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variable {
    Name,
}

#[derive(Debug, Clone, Deserialize)]
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

type DeserializedLine = Vec<DialogFragment>;

impl TryFrom<DialogScene<SerializedLine>> for DialogScene<DeserializedLine> {
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
            nodes.insert(
                name,
                DialogNode {
                    lines,
                    children: node.children.try_into()?,
                },
            );
        }
        Ok(DialogScene { nodes })
    }
}

impl TryFrom<NodeChildren<SerializedLine>> for NodeChildren<DeserializedLine> {
    type Error = anyhow::Error;
    fn try_from(children: NodeChildren<SerializedLine>) -> Result<Self, Self::Error> {
        let parser = line_parser();
        Ok(match children {
            NodeChildren::Choices(choices) => {
                let mut children: IndexMap<_, Vec<_>> = IndexMap::new();
                for (name, texts) in choices {
                    for text in texts {
                        if let SerializedLine::String(text) = text {
                            if let Line::Text(text) =
                                parser.parse(text).map_err(|mut e| anyhow!(e.remove(0)))?
                            {
                                children.entry(name.clone()).or_default().push(text);
                            }
                        }
                    }
                }
                NodeChildren::Choices(children)
            }
            NodeChildren::Jump { jump } => NodeChildren::Jump { jump },
            NodeChildren::Condition {
                condition,
                then,
                els,
            } => NodeChildren::Condition {
                condition,
                then: Box::new((*then).try_into()?),
                els: Box::new((*els).try_into()?),
            },
            NodeChildren::Wait {
                condition,
                then: node,
            } => NodeChildren::Wait {
                condition,
                then: node,
            },
            NodeChildren::List(list) => {
                NodeChildren::List(list.into_iter().map(TryInto::try_into).try_collect()?)
            }
            NodeChildren::Next(texts) => {
                let mut children = Vec::new();
                for text in texts {
                    if let SerializedLine::String(text) = text {
                        if let Line::Text(text) =
                            parser.parse(text).map_err(|mut e| anyhow!(e.remove(0)))?
                        {
                            children.push(text);
                        }
                    }
                }
                NodeChildren::Next(children)
            }
        })
    }
}

trait FragmentParser<T>: Parser<char, T, Error = Simple<char>> {}

impl<P, T> FragmentParser<T> for P where P: Parser<char, T, Error = Simple<char>> {}

fn line_parser() -> impl FragmentParser<Line<DeserializedLine>> {
    fragments().map(Line::Text).then_ignore(end())
}

fn fragments() -> impl FragmentParser<DeserializedLine> {
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
    left_speaker: Option<Speaker>,
    right_speaker: Option<Speaker>,
    speaker: Option<CurrentSpeaker>,
    can_cast: bool,
    flags: HashSet<String>,
}

const DIALOG_SPEED: usize = 3;

impl DialogState {
    pub fn allows_casting(&self) -> bool {
        if self.can_cast {
            return true;
        }
        let node = &DIALOG_SCENES[&self.scene].nodes[&self.node];
        self.line == node.lines.len() - 1 && node.children.enables_casting()
    }
    pub fn speakers_ui(&self, ui: &mut Ui) -> bool {
        if self.left_speaker.is_none() && self.right_speaker.is_none() {
            return false;
        }
        const PORTRAIT_HEIGHT: f32 = 200.0;
        if let Some(speaker) = &self.left_speaker {
            let focused = self
                .speaker
                .as_ref()
                .map_or(true, |curr| curr.name() == speaker.name());
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                image_plot(
                    ui,
                    &speaker.image(),
                    Vec2::splat(PORTRAIT_HEIGHT),
                    ImagePlotKind::Portrait(focused),
                );
            });
        }
        if let Some(speaker) = &self.right_speaker {
            let focused = self
                .speaker
                .as_ref()
                .map_or(true, |curr| curr.name() == speaker.name());
            ui.with_layout(Layout::bottom_up(Align::Max), |ui| {
                image_plot(
                    ui,
                    &speaker.image(),
                    Vec2::splat(PORTRAIT_HEIGHT),
                    ImagePlotKind::Portrait(focused),
                );
            });
        }
        true
    }
}

impl NodeChildren<DeserializedLine> {
    fn enables_casting(&self) -> bool {
        match self {
            NodeChildren::Condition { then, els, .. } => {
                then.enables_casting() || els.enables_casting()
            }
            NodeChildren::Wait { condition, .. } => match condition {
                WaitCondition::KnowField(_) => true,
                WaitCondition::SayWord(_) => true,
                WaitCondition::EmptyStack => true,
            },
            NodeChildren::Choices(_) => false,
            NodeChildren::Jump { .. } => false,
            NodeChildren::List(list) => list.iter().any(Self::enables_casting),
            NodeChildren::Next(_) => false,
        }
    }
}

impl DialogState {
    fn check_condition(&self, world: &World, condition: &Condition) -> bool {
        match condition {
            Condition::FieldKnown(kind) => world.player.progression.known_fields.contains(kind),
            Condition::Flag(flag) => self.flags.contains(flag),
            Condition::Not(inner) => !self.check_condition(world, inner),
            Condition::And(conditions) => conditions
                .iter()
                .all(|condition| self.check_condition(world, condition)),
            Condition::Or(conditions) => conditions
                .iter()
                .any(|condition| self.check_condition(world, condition)),
        }
    }
}

impl Game {
    pub fn set_dialog(&mut self, scene_name: &str) {
        let scene = &DIALOG_SCENES[scene_name];
        let dialog = DialogState {
            scene: scene_name.into(),
            node: scene.nodes.first().unwrap().0.clone(),
            line: 0,
            character: 0,
            speaker: None,
            can_cast: false,
            left_speaker: None,
            right_speaker: None,
            flags: HashSet::default(),
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
            } else if matches!(&node.children, NodeChildren::Choices(choices) if choices.is_empty())
            {
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
                        ui.heading(format!("{}:", speaker.display()));
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
                } else if dialog.line < node.lines.len() - 1 {
                    if next() {
                        self.progress_dialog();
                    }
                } else {
                    self.node_children_ui(ui, line_text, node.children.clone());
                }
            }
            Line::Command(command) => {
                let progression = &mut self.world.player.progression;
                match command {
                    DialogCommand::Left(speaker) => dialog.left_speaker = speaker.clone(),
                    DialogCommand::Right(speaker) => dialog.right_speaker = speaker.clone(),
                    DialogCommand::Background(image) => self.ui_state.background = image.clone(),
                    DialogCommand::Speaker(speaker) => dialog.speaker = speaker.clone(),
                    DialogCommand::RevealWord(word) => {
                        progression.known_words.insert(*word);
                    }
                    DialogCommand::RevealAllWords => progression.known_words.extend(all::<Word>()),
                    DialogCommand::RevealManaBar => progression.mana_bar = true,
                    DialogCommand::RevealField(kind) => {
                        progression.known_fields.insert(*kind);
                        self.ui_state.fields_display.insert(
                            (*kind).into(),
                            self.ui_state.default_field_display((*kind).into()),
                        );
                    }
                    DialogCommand::RevealFree => progression.free = true,
                    DialogCommand::RevealConduit => progression.conduit = true,
                    DialogCommand::Set(flag) => {
                        dialog.flags.insert(flag.clone());
                    }
                    DialogCommand::Unset(flag) => {
                        dialog.flags.remove(flag);
                    }
                }
                self.progress_dialog();
                self.dialog_ui_impl(ui);
            }
        }
    }
    fn node_children_ui(
        &mut self,
        ui: &mut Ui,
        line_text: String,
        children: NodeChildren<DeserializedLine>,
    ) {
        let dialog = self.ui_state.dialog.as_mut().unwrap();
        let mut next = || {
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.button("Next").clicked()
            })
            .inner
        };
        match children {
            NodeChildren::Choices(choices) if choices.is_empty() => {
                // No choices
                if line_text.is_empty() || next() {
                    self.progress_dialog();
                }
            }
            NodeChildren::Choices(choices) => {
                // Choices
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    for (name, fragments) in choices.iter().rev() {
                        for fragments in fragments.iter().rev() {
                            if ui
                                .button(
                                    RichText::new(self.world.format_dialog_fragments(fragments))
                                        .heading(),
                                )
                                .clicked()
                            {
                                dialog.node = name.clone();
                                dialog.line = 0;
                                dialog.character = 0;
                            }
                        }
                    }
                });
            }
            NodeChildren::Jump { jump } => {
                if next() {
                    dialog.node = jump;
                    dialog.line = 0;
                    dialog.character = 0;
                }
            }
            NodeChildren::Condition {
                condition,
                then,
                els,
            } => {
                if dialog.check_condition(&self.world, &condition) {
                    self.node_children_ui(ui, line_text, *then)
                } else {
                    self.node_children_ui(ui, line_text, *els)
                }
            }
            NodeChildren::Wait {
                condition,
                then: node,
            } => {
                if self.world.wait_condition(&condition) {
                    dialog.node = node;
                    dialog.line = 0;
                    dialog.character = 0;
                }
                ui.allocate_exact_size(ui.available_size(), Sense::hover());
            }
            NodeChildren::List(list) => {
                for children in list {
                    self.node_children_ui(ui, line_text.clone(), children);
                }
            }
            NodeChildren::Next(fragments) => {
                let clicked = ui
                    .with_layout(Layout::bottom_up(Align::Min), |ui| {
                        fragments.iter().any(|fragments| {
                            ui.button(
                                RichText::new(self.world.format_dialog_fragments(fragments))
                                    .heading(),
                            )
                            .clicked()
                        })
                    })
                    .inner;
                if clicked {
                    self.progress_dialog();
                }
            }
        }
    }
}

impl World {
    fn wait_condition(&self, condition: &WaitCondition) -> bool {
        match condition {
            WaitCondition::SayWord(word) => self.player.person.stack.words().last() == Some(*word),
            WaitCondition::KnowField(kind) => self.player.progression.known_fields.contains(kind),
            WaitCondition::EmptyStack => self.player.person.stack.is_empty(),
        }
    }
    fn format_dialog_fragments(&self, fragments: &[DialogFragment]) -> String {
        let mut formatted = String::new();
        for (i, frag) in fragments.iter().enumerate() {
            let s = match frag {
                DialogFragment::String(s) => s,
                DialogFragment::Variable(var) => match var {
                    DialogVariable::Variable(var) => match var {
                        Variable::Name => &self.player.name,
                    },
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
