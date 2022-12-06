use std::collections::HashMap;

use eframe::egui::*;
use rapier2d::prelude::*;

use crate::field::{GenericField, ScalarInputFieldKind, VectorField, VectorOutputFieldKind};

#[derive(Default)]
pub struct World {
    pub objects: HashMap<RigidBodyHandle, Object>,
    pub spell_field: Option<GenericField>,
    pub outputs: OutputFields,
}

#[derive(Default)]
pub struct OutputFields {
    pub vectors: HashMap<VectorOutputFieldKind, VectorField>,
}

pub struct Object {
    pub pos: Pos2,
    pub shape: GraphicalShape,
    pub density: f32,
    pub shape_offset: Vec2,
    pub body_handle: RigidBodyHandle,
}

#[derive(Clone)]
pub enum GraphicalShape {
    Circle(f32),
    Box(Vec2),
    HalfSpace(Vec2),
    Capsule { half_height: f32, radius: f32 },
}

impl GraphicalShape {
    pub fn contains(&self, pos: Pos2) -> bool {
        match self {
            GraphicalShape::Circle(radius) => pos.distance(Pos2::ZERO) < *radius,
            GraphicalShape::Box(size) => pos.x.abs() < size.x / 2.0 && pos.y.abs() < size.x / 2.0,
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

impl World {
    pub fn find_object_at(&self, p: Pos2) -> Option<&Object> {
        self.objects
            .values()
            .find(|obj| obj.shape.contains(p - obj.pos.to_vec2() - obj.shape_offset))
    }
    pub fn sample_scalar_field(&self, kind: ScalarInputFieldKind, x: f32, y: f32) -> f32 {
        match kind {
            ScalarInputFieldKind::Density => self
                .find_object_at(pos2(x, y))
                .map(|obj| obj.density)
                .unwrap_or(0.0),
        }
    }
    pub fn sample_vector_field(&self, kind: VectorOutputFieldKind, x: f32, y: f32) -> Vec2 {
        self.outputs
            .vectors
            .get(&kind)
            .map(|field| field.sample(self, x, y))
            .unwrap_or_default()
    }
}
