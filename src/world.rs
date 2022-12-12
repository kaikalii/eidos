use std::{
    collections::HashMap,
    f32::consts::PI,
    iter::{empty, once},
};

use eframe::egui::*;
use enum_iterator::all;
use indexmap::IndexMap;
use rapier2d::prelude::*;

use crate::{
    field::*,
    math::Convert,
    npc::{Npc, NpcId, ScheduleTask, NPCS},
    object::*,
    person::{Person, PersonId},
    physics::PhysicsContext,
    player::Player,
    word::Word,
};

pub struct World {
    pub player: Player,
    pub npcs: HashMap<NpcId, Npc>,
    pub objects: IndexMap<RigidBodyHandle, Object>,
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
            objects: IndexMap::new(),
            active_spells: ActiveSpells::default(),
            controls: Controls::default(),
        };
        // Add objects
        // Ground
        world.add_object(
            ObjectKind::Ground,
            ObjectDef::new(RigidBodyType::Fixed).shapes(
                GraphicalShape::HalfSpace(Vec2::Y)
                    .offset(Vec2::ZERO)
                    .density(3.0),
            ),
            ObjectProperties::default(),
            |rb| rb,
            |c| c.restitution(0.5),
        );
        // Player
        const HEIGHT: f32 = 4.0 / 7.0 * 1.75;
        const HEAD_HEIGHT: f32 = 1.0 / 3.0 * HEIGHT;
        const HEAD_WIDTH: f32 = 2.0 / 3.0 * HEAD_HEIGHT;
        const TORSO_HEIGHT: f32 = HEIGHT - HEAD_HEIGHT / 2.0;
        const TORSO_WIDTH: f32 = 3.0 / 8.0 * TORSO_HEIGHT;
        world.player.person.body_handle = world.add_object(
            ObjectKind::Player,
            ObjectDef::new(RigidBodyType::Dynamic).shapes(vec![
                GraphicalShape::capsule_wh(TORSO_WIDTH, TORSO_HEIGHT)
                    .offset(vec2(0.0, -HEAD_HEIGHT / 2.0)),
                GraphicalShape::capsule_wh(HEAD_WIDTH, HEAD_HEIGHT)
                    .offset(vec2(0.0, TORSO_HEIGHT / 2.0)),
            ]),
            ObjectProperties {
                magic: world.player.person.max_mana / 5.0,
            },
            |rb| {
                rb.rotation(PI / 2.0)
                    .translation([0.0, 0.5 + TORSO_WIDTH].into())
            },
            |c| c,
        );
        // Npcs
        for npc_id in all::<NpcId>() {
            let def = &NPCS[&npc_id];
            let npc = Npc {
                active: false,
                person: Person::new(def.max_mana),
                task: ScheduleTask::Stand,
            };
            world.npcs.insert(npc_id, npc);
        }
        // Place
        world.load_place("magician_house");
        world
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
}

pub enum ShapeLayer {
    Foreground,
    Background,
    Far,
}

impl ShapeLayer {
    pub fn multiplier(&self) -> f32 {
        match self {
            ShapeLayer::Foreground => 1.0,
            ShapeLayer::Background => 0.5,
            ShapeLayer::Far => 0.2,
        }
    }
}

impl World {
    pub fn find_object_filtered_at(
        &self,
        p: Pos2,
        filter: impl Fn(&Object, &RigidBody) -> bool,
    ) -> Option<(&Object, &OffsetShape, ShapeLayer)> {
        puffin::profile_function!();
        self.objects.values().find_map(|obj| {
            puffin::profile_function!();
            if !filter(obj, &self.physics.bodies[obj.body_handle]) {
                return None;
            }
            let transformed_point = obj.transform_point(p);
            for (shapes, layer) in [
                (&obj.def.shapes, ShapeLayer::Foreground),
                (&obj.def.background, ShapeLayer::Background),
                (&obj.def.far, ShapeLayer::Far),
            ] {
                if let Some(shape) = shapes
                    .iter()
                    .find(|shape| shape.contains(transformed_point))
                {
                    return Some((obj, shape, layer));
                }
            }
            None
        })
    }
    pub fn find_object_at(&self, p: Pos2) -> Option<(&Object, &OffsetShape, ShapeLayer)> {
        self.find_object_filtered_at(p, |_, _| true)
    }
    pub fn sample_scalar_field(
        &self,
        kind: GenericScalarFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> f32 {
        puffin::profile_function!(kind.to_string());
        match kind {
            GenericScalarFieldKind::Input(kind) => {
                self.sample_input_scalar_field(kind, pos, allow_recursion)
            }
            GenericScalarFieldKind::Output(kind) => self.sample_output_scalar_field(kind, pos),
        }
    }
    pub fn sample_vector_field(
        &self,
        kind: GenericVectorFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        match kind {
            GenericVectorFieldKind::Input(kind) => self.sample_input_vector_field(kind, pos),
            GenericVectorFieldKind::Output(kind) => {
                self.sample_output_vector_field(kind, pos, allow_recursion)
            }
        }
    }
    pub fn sample_input_scalar_field(
        &self,
        kind: ScalarInputFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> f32 {
        puffin::profile_function!(kind.to_string());
        match kind {
            ScalarInputFieldKind::Density => self
                .find_object_at(pos)
                .map(|(_, shape, layer)| shape.density * layer.multiplier())
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
                let mul = if let Some((obj, _, layer)) = self.find_object_at(pos) {
                    if let ShapeLayer::Foreground = layer {
                        return obj.props.magic;
                    } else {
                        layer.multiplier()
                    }
                } else {
                    1.0
                };
                if !allow_recursion {
                    return 1.0;
                }
                let mut sum = 0.0;
                for (person_id, spells) in &self.active_spells.scalars {
                    for spell in spells.values().flatten() {
                        sum += spell
                            .field
                            .sample_relative(self, *person_id, pos, false)
                            .abs();
                    }
                }
                for (person_id, spells) in &self.active_spells.vectors {
                    for spell in spells.values().flatten() {
                        sum += spell
                            .field
                            .sample_relative(self, *person_id, pos, false)
                            .length();
                    }
                }
                sum * mul
            }
        }
    }
    pub fn sample_input_vector_field(&self, kind: VectorInputFieldKind, _pos: Pos2) -> Vec2 {
        match kind {}
    }
    pub fn sample_output_scalar_field(&self, kind: ScalarOutputFieldKind, _pos: Pos2) -> f32 {
        match kind {}
    }
    pub fn sample_output_vector_field(
        &self,
        kind: VectorOutputFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        self.active_spells
            .vectors
            .iter()
            .filter_map(|(person_id, spells)| spells.get(&kind).map(|spells| (person_id, spells)))
            .flat_map(|(person_id, spells)| spells.iter().map(move |spell| (person_id, spell)))
            .fold(Vec2::ZERO, |acc, (person_id, spell)| {
                acc + spell
                    .field
                    .sample_relative(self, *person_id, pos, allow_recursion)
                    * self.person(*person_id).field_scale()
            })
    }
    pub fn person_is_at(&self, person_id: PersonId, pos: Pos2) -> bool {
        let object = &self.objects[&self.person(person_id).body_handle];
        let point = object.transform_point(pos);
        object.def.shapes.iter().any(|shape| shape.contains(point))
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
    pub fn load_place(&mut self, place_name: &str) {
        let Some(place) = PLACES.get(place_name) else {
            return;
        };
        // Add objects
        for po in &place.objects {
            let object = OBJECTS[&po.name].clone();
            self.add_object_def(po.pos, object);
        }
        // (De)activate npcs
        for npc_id in all::<NpcId>() {
            let mut npc = self.npcs.get_mut(&npc_id).unwrap();
            let (npc_place, pos) = npc.desired_place();
            let pos = pos;
            if npc_place == place_name {
                if !npc.active {
                    npc.active = true;
                    npc.person.pos = pos;
                    let magic = npc.person.max_mana / 5.0;
                    const HEIGHT: f32 = 1.75;
                    const UPPER_HEIGHT: f32 = 4.0 / 7.0 * HEIGHT;
                    const LOWER_HEIGHT: f32 = HEIGHT - UPPER_HEIGHT;
                    const HEAD_HEIGHT: f32 = 1.0 / 3.0 * UPPER_HEIGHT;
                    const HEAD_WIDTH: f32 = 2.0 / 3.0 * HEAD_HEIGHT;
                    const TORSO_HEIGHT: f32 = UPPER_HEIGHT - HEAD_HEIGHT / 2.0;
                    const TORSO_WIDTH: f32 = 3.0 / 8.0 * TORSO_HEIGHT;
                    self.npcs.get_mut(&npc_id).unwrap().person.body_handle = self.add_object(
                        ObjectKind::Npc,
                        ObjectDef::new(RigidBodyType::Dynamic).shapes(vec![
                            GraphicalShape::capsule_wh(TORSO_WIDTH, TORSO_HEIGHT)
                                .offset(vec2(0.0, (LOWER_HEIGHT - HEAD_HEIGHT) / 2.0)),
                            GraphicalShape::capsule_wh(HEAD_WIDTH, HEAD_HEIGHT)
                                .offset(vec2(0.0, (LOWER_HEIGHT + TORSO_HEIGHT) / 2.0)),
                            GraphicalShape::capsule_wh(TORSO_WIDTH, LOWER_HEIGHT)
                                .offset(vec2(0.0, -(HEAD_HEIGHT + TORSO_HEIGHT) / 2.0)),
                        ]),
                        ObjectProperties { magic },
                        |rb| rb.translation(pos.convert()).lock_rotations(),
                        |c| c,
                    );
                }
            } else {
                npc.active = false;
                self.objects.remove(&npc.person.body_handle);
                self.physics.remove(npc.person.body_handle);
            }
        }
    }
}
