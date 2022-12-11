use std::{
    collections::HashMap,
    iter::{empty, once},
};

use eframe::egui::*;
use rapier2d::prelude::*;

use crate::{
    field::*,
    math::rotate,
    person::{Npc, NpcId, Person, PersonId},
    physics::PhysicsContext,
    player::Player,
    word::Word,
};

pub struct World {
    pub player: Player,
    pub npcs: HashMap<NpcId, Npc>,
    pub objects: HashMap<RigidBodyHandle, Object>,
    pub physics: PhysicsContext,
    pub active_spells: ActiveSpells,
    pub controls: Controls,
}

type TypedActiveSpells<K, V> = HashMap<PersonId, HashMap<K, Vec<ActiveSpell<V>>>>;

#[derive(Default)]
pub struct ActiveSpells {
    pub scalars: TypedActiveSpells<ScalarOutputFieldKind, ScalarField>,
    pub vectors: TypedActiveSpells<VectorOutputFieldKind, VectorField>,
}

pub struct ActiveSpell<T> {
    pub field: T,
    pub words: Vec<Word>,
}

impl ActiveSpells {
    pub fn contains(&self, kind: GenericOutputFieldKind) -> bool {
        match kind {
            GenericOutputFieldKind::Scalar(kind) => self
                .scalars
                .values()
                .any(|fields| fields.contains_key(&kind)),
            GenericOutputFieldKind::Vector(kind) => self
                .vectors
                .values()
                .any(|fields| fields.contains_key(&kind)),
        }
    }
    pub fn remove(&mut self, person_id: PersonId, kind: GenericOutputFieldKind, i: usize) {
        match kind {
            GenericOutputFieldKind::Scalar(kind) => {
                self.scalars
                    .entry(person_id)
                    .or_default()
                    .entry(kind)
                    .or_default()
                    .remove(i);
            }
            GenericOutputFieldKind::Vector(kind) => {
                self.vectors
                    .entry(person_id)
                    .or_default()
                    .entry(kind)
                    .or_default()
                    .remove(i);
            }
        }
    }
    pub fn player_spell_words(
        &self,
        kind: GenericOutputFieldKind,
    ) -> Box<dyn ExactSizeIterator<Item = &[Word]> + '_> {
        match kind {
            GenericOutputFieldKind::Scalar(kind) => {
                let Some(spells) = self.scalars.get(&PersonId::Player) else {
                    return Box::new(empty());
                };
                let Some(spells) = spells.get(&kind) else {
                    return Box::new(empty());
                };
                Box::new(spells.iter().map(|spell| spell.words.as_slice()))
            }
            GenericOutputFieldKind::Vector(kind) => {
                let Some(spells) = self.vectors.get(&PersonId::Player) else {
                    return Box::new(empty());
                };
                let Some(spells) = spells.get(&kind) else {
                    return Box::new(empty());
                };
                Box::new(spells.iter().map(|spell| spell.words.as_slice()))
            }
        }
    }
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

impl World {
    pub fn new(player: Player) -> Self {
        // Init world
        let mut world = World {
            player,
            npcs: HashMap::new(),
            physics: PhysicsContext::default(),
            objects: HashMap::new(),
            active_spells: ActiveSpells::default(),
            controls: Controls::default(),
        };
        // Add objects
        // Ground
        world.add_object(
            Properties::default(),
            GraphicalShape::HalfSpace(Vec2::Y)
                .offset(Vec2::ZERO)
                .density(3.0),
            RigidBodyBuilder::fixed(),
            |c| c.restitution(0.5),
        );
        // Rock?
        world.add_object(
            Properties::default(),
            GraphicalShape::Circle(1.0).offset(Vec2::ZERO).density(2.0),
            RigidBodyBuilder::dynamic().translation([3.0, 10.0].into()),
            |c| c,
        );
        // Player
        const HEIGHT: f32 = 4.0 / 7.0 * 1.75;
        const HEAD_HEIGHT: f32 = 1.0 / 3.0 * HEIGHT;
        const HEAD_WIDTH: f32 = 2.0 / 3.0 * HEAD_HEIGHT;
        const TORSO_HEIGHT: f32 = HEIGHT - HEAD_HEIGHT / 2.0;
        const TORSO_WIDTH: f32 = 3.0 / 8.0 * TORSO_HEIGHT;
        world.player.person.body_handle = world.add_object(
            Properties { magic: 10.0 },
            vec![
                GraphicalShape::capsule_wh(TORSO_WIDTH, TORSO_HEIGHT).offset(vec2(0.0, 0.0)),
                GraphicalShape::capsule_wh(HEAD_WIDTH, HEAD_HEIGHT)
                    .offset(vec2(0.0, (TORSO_HEIGHT + HEAD_HEIGHT) / 2.0)),
            ],
            RigidBodyBuilder::dynamic().translation([0.0, HEIGHT / 2.0].into()),
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
    pub props: Properties,
}

#[derive(Default)]
pub struct Properties {
    pub magic: f32,
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
    #[allow(dead_code)]
    Box(Vec2),
    HalfSpace(Vec2),
    Capsule {
        half_height: f32,
        radius: f32,
    },
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
    #[track_caller]
    pub fn person(&self, person_id: PersonId) -> &Person {
        match person_id {
            PersonId::Player => &self.player.person,
            PersonId::Npc(npc_id) => {
                if let Some(npc) = self.npcs.get(&npc_id) {
                    &npc.person
                } else {
                    panic!("No npc with id {npc_id:?}");
                }
            }
        }
    }
    #[track_caller]
    pub fn person_mut(&mut self, person_id: PersonId) -> &mut Person {
        match person_id {
            PersonId::Player => &mut self.player.person,
            PersonId::Npc(npc_id) => {
                if let Some(npc) = self.npcs.get_mut(&npc_id) {
                    &mut npc.person
                } else {
                    panic!("No npc with id {npc_id:?}");
                }
            }
        }
    }
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
            ScalarInputFieldKind::Magic => {
                if let Some((obj, _)) = self.find_object_at(pos) {
                    return obj.props.magic;
                }
                let mut sum = 0.0;
                for (person_id, spells) in &self.active_spells.scalars {
                    for spell in spells.values().flatten() {
                        sum += spell.field.sample_relative(self, *person_id, pos).abs();
                    }
                }
                for (person_id, spells) in &self.active_spells.vectors {
                    for spell in spells.values().flatten() {
                        sum += spell.field.sample_relative(self, *person_id, pos).length();
                    }
                }
                sum
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
        self.active_spells
            .vectors
            .iter()
            .filter_map(|(person_id, spells)| spells.get(&kind).map(|spells| (person_id, spells)))
            .flat_map(|(person_id, spells)| spells.iter().map(move |spell| (person_id, spell)))
            .fold(Vec2::ZERO, |acc, (person_id, spell)| {
                acc + spell.field.sample_relative(self, *person_id, pos)
                    * self.person(*person_id).field_scale()
            })
    }
    pub fn people(&self) -> impl Iterator<Item = &Person> {
        self.person_ids_iter().map(|id| self.person(id))
    }
    pub fn person_ids_iter(&self) -> impl Iterator<Item = PersonId> + '_ {
        once(PersonId::Player).chain(self.npcs.keys().copied().map(PersonId::Npc))
    }
    pub fn person_ids(&self) -> Vec<PersonId> {
        self.person_ids_iter().collect()
    }
    pub fn update(&mut self) {
        // Run physics
        let work_done = self.run_physics();
        // Update mana
        for id in self.person_ids() {
            self.person_mut(id).do_work(work_done);
            let can_cast = self.person(id).can_cast();
            let scalars = self.active_spells.scalars.entry(id).or_default();
            let vectors = self.active_spells.vectors.entry(id).or_default();
            if !can_cast {
                scalars.clear();
                vectors.clear();
            }
            if scalars.values().all(|spells| spells.is_empty())
                && vectors.values().all(|spells| spells.is_empty())
            {
                self.person_mut(id).regen_mana();
            }
        }
    }
}
