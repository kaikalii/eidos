use eframe::egui::*;

use crate::math::{rect_poly, regular_poly};

pub struct World {
    pub static_objects: Vec<Object>,
}

pub struct Object {
    pub shape: Vec<Vec2>,
    pub density: f32,
}

impl Object {
    pub fn new(shape: impl IntoIterator<Item = Vec2>, density: f32) -> Self {
        Object {
            shape: shape.into_iter().collect(),
            density,
        }
    }
}

impl Default for World {
    fn default() -> Self {
        World {
            static_objects: vec![
                Object::new(rect_poly(vec2(2.0, 3.0), vec2(6.0, 6.0)), 1.0),
                Object::new(regular_poly(vec2(-3.0, -3.0), 1.5, 5, 0.0), 0.8),
            ],
        }
    }
}
