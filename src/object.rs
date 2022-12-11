use std::{collections::HashMap, fs};

use anyhow::{anyhow, bail};
use eframe::egui::*;
use once_cell::sync::Lazy;
use rapier2d::prelude::*;
use serde::{Deserialize, Deserializer};

use crate::utils::{fatal_error, resources_path};

pub struct Object {
    pub kind: ObjectKind,
    pub def: ObjectDef,
    pub pos: Pos2,
    pub rot: f32,
    pub body_handle: RigidBodyHandle,
    pub props: Properties,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectKind {
    Player,
    Npc,
    Object,
    Background,
    Ground,
}

#[derive(Default)]
pub struct Properties {
    pub magic: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OffsetShape {
    pub shape: GraphicalShape,
    #[serde(deserialize_with = "vec2_as_array")]
    pub offset: Vec2,
    pub density: f32,
}

impl OffsetShape {
    pub fn contains(&self, pos: Pos2) -> bool {
        self.shape.contains(pos - self.offset)
    }
    pub fn density(self, density: f32) -> Self {
        Self { density, ..self }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphicalShape {
    Circle(f32),
    Box(#[serde(deserialize_with = "vec2_as_array")] Vec2),
    HalfSpace(#[serde(deserialize_with = "vec2_as_array")] Vec2),
    Capsule { half_height: f32, radius: f32 },
}

impl GraphicalShape {
    pub fn capsule_wh(width: f32, height: f32) -> Self {
        GraphicalShape::Capsule {
            half_height: (height - width) / 2.0,
            radius: width / 2.0,
        }
    }
    pub fn offset(self, offset: Vec2) -> OffsetShape {
        OffsetShape {
            shape: self,
            offset,
            density: 1.0,
        }
    }
    pub fn contains(&self, pos: Pos2) -> bool {
        match self {
            GraphicalShape::Circle(radius) => pos.distance(Pos2::ZERO) < *radius,
            GraphicalShape::Box(size) => pos.x.abs() < size.x / 2.0 && pos.y.abs() < size.y / 2.0,
            GraphicalShape::HalfSpace(normal) => pos.y < -normal.x / normal.y * pos.x,
            GraphicalShape::Capsule {
                half_height,
                radius,
            } => {
                pos.x.abs() < *radius && pos.y.abs() < *half_height
                    || pos.distance(pos2(0.0, *half_height)) < *radius
                    || pos.distance(pos2(0.0, -*half_height)) < *radius
            }
        }
    }
}

pub trait IntoShapes {
    fn into_shapes(self) -> Vec<OffsetShape>;
}

impl IntoShapes for OffsetShape {
    fn into_shapes(self) -> Vec<OffsetShape> {
        vec![self]
    }
}

impl IntoShapes for GraphicalShape {
    fn into_shapes(self) -> Vec<OffsetShape> {
        vec![OffsetShape {
            shape: self,
            offset: Vec2::ZERO,
            density: 1.0,
        }]
    }
}

impl IntoShapes for Vec<OffsetShape> {
    fn into_shapes(self) -> Vec<OffsetShape> {
        self
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObjectDef {
    #[serde(rename = "type")]
    pub ty: RigidBodyType,
    #[serde(default)]
    pub shapes: Vec<OffsetShape>,
    #[serde(default)]
    pub background: Vec<OffsetShape>,
    #[serde(default)]
    pub far: Vec<OffsetShape>,
}

impl ObjectDef {
    pub fn new(ty: RigidBodyType) -> Self {
        ObjectDef {
            ty,
            shapes: Vec::new(),
            background: Vec::new(),
            far: Vec::new(),
        }
    }
    pub fn shapes(self, shapes: impl IntoShapes) -> Self {
        Self {
            shapes: shapes.into_shapes(),
            ..self
        }
    }
    pub fn background(self, shapes: impl IntoShapes) -> Self {
        Self {
            background: shapes.into_shapes(),
            ..self
        }
    }
    pub fn far(self, shapes: impl IntoShapes) -> Self {
        Self {
            far: shapes.into_shapes(),
            ..self
        }
    }
}

pub static OBJECTS: Lazy<HashMap<String, ObjectDef>> = Lazy::new(|| {
    let yaml = fs::read_to_string(resources_path().join("objects.yaml"));
    let yaml = yaml
        .as_deref()
        .unwrap_or(include_str!("../resources/objects.yaml"));
    match serde_yaml::from_str::<HashMap<String, ObjectDef>>(yaml) {
        Ok(objects) => objects,
        Err(e) => fatal_error(format!("Unable to read objects file: {e}")),
    }
});

#[derive(Debug, Clone, Deserialize)]
pub struct PlacedObject {
    pub name: String,
    #[serde(deserialize_with = "pos2_as_array")]
    pub pos: Pos2,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Place {
    #[serde(deserialize_with = "vec2_as_array")]
    pub offset: Vec2,
    pub objects: Vec<PlacedObject>,
}

pub static PLACES: Lazy<HashMap<String, Place>> =
    Lazy::new(|| load_places().unwrap_or_else(|e| fatal_error(e)));

fn load_places() -> anyhow::Result<HashMap<String, Place>> {
    let mut map = HashMap::new();
    for entry in fs::read_dir(resources_path().join("places"))
        .map_err(|e| anyhow!("Unable to open places directory: {e}"))?
    {
        let entry = entry.unwrap();
        if entry.file_type()?.is_file() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let yaml = fs::read_to_string(&path)?;
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                let place: Place = serde_yaml::from_str(&yaml)
                    .map_err(|e| anyhow!("Unable to read {name} place: {e}"))?;
                for po in &place.objects {
                    if !OBJECTS.contains_key(&po.name) {
                        bail!("Error in {name} place");
                    }
                }
                map.insert(name, place);
            }
        }
    }
    Ok(map)
}

fn vec2_as_array<'de, D>(deserializer: D) -> Result<Vec2, D::Error>
where
    D: Deserializer<'de>,
{
    let [x, y] = <[f32; 2]>::deserialize(deserializer)?;
    Ok(vec2(x, y))
}

fn pos2_as_array<'de, D>(deserializer: D) -> Result<Pos2, D::Error>
where
    D: Deserializer<'de>,
{
    let [x, y] = <[f32; 2]>::deserialize(deserializer)?;
    Ok(pos2(x, y))
}
