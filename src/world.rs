use std::collections::HashMap;

use eframe::egui::*;
use rapier2d::prelude::*;

use crate::{field::*, game::TICK_RATE, math::rotate, physics::PhysicsContext, word::Word};

pub struct World {
    pub player_pos: Pos2,
    pub player: Player,
    pub objects: HashMap<RigidBodyHandle, Object>,
    pub physics: PhysicsContext,
    pub spell_field: Option<GenericField>,
    pub outputs: OutputFields,
    pub controls: Controls,
}

pub const MANA_REGEN_RATE: f32 = 1.0;
pub const MAX_MANA_EXHAUSTION: f32 = 5.0;

pub struct Player {
    pub body_handle: RigidBodyHandle,
    pub mana: f32,
    pub max_mana: f32,
    pub mana_exhaustion: f32,
    pub spell: Vec<Word>,
}

#[derive(Default)]
pub struct OutputFields {
    pub scalars: HashMap<ScalarOutputFieldKind, ScalarField>,
    pub vectors: HashMap<VectorOutputFieldKind, VectorField>,
}

#[derive(Default)]
pub struct Controls {
    pub x_slider: Option<f32>,
    pub y_slider: Option<f32>,
}

impl Controls {
    pub fn get(&self, kind: ControlKind) -> f32 {
        match kind {
            ControlKind::XSlider => self.x_slider.unwrap_or(0.0),
            ControlKind::YSlider => self.y_slider.unwrap_or(0.0),
        }
    }
}

impl Player {
    pub fn field_scale(&self) -> f32 {
        if self.mana_exhaustion > 0.0 {
            0.0
        } else {
            1.0
        }
    }
    pub fn do_work(&mut self, work: f32) {
        self.mana -= work;
        if self.mana < 0.0 {
            self.mana = 0.0;
            self.mana_exhaustion = MAX_MANA_EXHAUSTION;
        }
    }
    fn regen_mana(&mut self) {
        if self.mana_exhaustion > 0.0 {
            self.mana_exhaustion = (self.mana_exhaustion - TICK_RATE * MANA_REGEN_RATE).max(0.0);
        } else {
            self.mana = (self.mana + TICK_RATE * MANA_REGEN_RATE).min(self.max_mana);
        }
    }
    pub fn can_cast(&self) -> bool {
        self.mana_exhaustion <= 0.0
    }
}

impl Default for World {
    fn default() -> Self {
        // Init world
        let mut world = World {
            player_pos: Pos2::ZERO,
            player: Player {
                body_handle: RigidBodyHandle::default(),
                mana: 40.0,
                max_mana: 40.0,
                mana_exhaustion: 0.0,
                spell: Vec::new(),
            },
            physics: PhysicsContext::default(),
            objects: HashMap::new(),
            outputs: OutputFields::default(),
            spell_field: None,
            controls: Controls::default(),
        };
        // Add objects
        // Ground
        world.add_object(
            GraphicalShape::HalfSpace(Vec2::Y)
                .offset(Vec2::ZERO)
                .density(3.0),
            RigidBodyBuilder::fixed(),
            |c| c.restitution(0.5),
        );
        // Rock?
        world.add_object(
            GraphicalShape::Circle(1.0).offset(Vec2::ZERO).density(2.0),
            RigidBodyBuilder::dynamic().translation([3.0, 10.0].into()),
            |c| c,
        );
        // Player
        world.player.body_handle = world.add_object(
            vec![
                GraphicalShape::Capsule {
                    half_height: 0.25,
                    radius: 0.25,
                }
                .offset(Vec2::ZERO),
                GraphicalShape::Circle(0.3).offset(vec2(0.0, 0.5)),
            ],
            RigidBodyBuilder::dynamic().translation([0.0, 0.5].into()),
            |c| c,
        );
        world
    }
}
pub struct Object {
    pub pos: Pos2,
    pub rot: f32,
    pub shapes: Vec<OffsetShape>,
    pub body_handle: RigidBodyHandle,
}

#[derive(Clone)]
pub struct OffsetShape {
    pub shape: GraphicalShape,
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

#[derive(Clone)]
pub enum GraphicalShape {
    Circle(f32),
    Box(Vec2),
    HalfSpace(Vec2),
    Capsule { half_height: f32, radius: f32 },
}

impl GraphicalShape {
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
    pub fn find_object_filtered_at(
        &self,
        p: Pos2,
        filter: impl Fn(&Object, &RigidBody) -> bool,
    ) -> Option<(&Object, &OffsetShape)> {
        puffin::profile_function!();
        self.objects.values().find_map(|obj| {
            puffin::profile_function!();
            if !filter(obj, &self.physics.bodies[obj.body_handle]) {
                return None;
            }
            let transformed_point = rotate(p.to_vec2() - obj.pos.to_vec2(), -obj.rot).to_pos2();
            let shape = obj
                .shapes
                .iter()
                .find(|shape| shape.contains(transformed_point))?;
            Some((obj, shape))
        })
    }
    pub fn find_object_at(&self, p: Pos2) -> Option<(&Object, &OffsetShape)> {
        self.find_object_filtered_at(p, |_, _| true)
    }
    pub fn sample_scalar_field(&self, kind: GenericScalarFieldKind, pos: Pos2) -> f32 {
        puffin::profile_function!(kind.to_string());
        match kind {
            GenericScalarFieldKind::Input(kind) => self.sample_input_scalar_field(kind, pos),
            GenericScalarFieldKind::Output(kind) => self.sample_output_scalar_field(kind, pos),
        }
    }
    pub fn sample_vector_field(&self, kind: GenericVectorFieldKind, pos: Pos2) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        match kind {
            GenericVectorFieldKind::Input(kind) => self.sample_input_vector_field(kind, pos),
            GenericVectorFieldKind::Output(kind) => self.sample_output_vector_field(kind, pos),
        }
    }
    pub fn sample_input_scalar_field(&self, kind: ScalarInputFieldKind, pos: Pos2) -> f32 {
        puffin::profile_function!(kind.to_string());
        match kind {
            ScalarInputFieldKind::Density => self
                .find_object_at(pos)
                .map(|(_, shape)| shape.density)
                .unwrap_or(0.0),
            ScalarInputFieldKind::Elevation => {
                let mut test = pos;
                while test.y > 0.0 {
                    puffin::profile_scope!("elevation test");
                    if self
                        .find_object_filtered_at(test, |_, body| body.body_type().is_fixed())
                        .is_some()
                    {
                        return pos.y - test.y;
                    }
                    test.y -= 0.5;
                }
                pos.y
            }
        }
    }
    pub fn sample_input_vector_field(&self, kind: VectorInputFieldKind, _pos: Pos2) -> Vec2 {
        match kind {}
    }
    pub fn sample_output_scalar_field(&self, kind: ScalarOutputFieldKind, _pos: Pos2) -> f32 {
        match kind {}
    }
    pub fn sample_output_vector_field(&self, kind: VectorOutputFieldKind, pos: Pos2) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        self.outputs
            .vectors
            .get(&kind)
            .map(|field| field.sample(self, pos))
            .unwrap_or_default()
            * self.player.field_scale()
    }
    pub fn update(&mut self) {
        // Run physics
        let work_done = self.run_physics();
        // Update mana
        self.player.do_work(work_done);
        if !self.player.can_cast() {
            self.outputs.scalars.clear();
            self.outputs.vectors.clear();
        }
        if work_done.abs() == 0.0 {
            self.player.regen_mana();
        }
    }
}
