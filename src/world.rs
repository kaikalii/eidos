use std::{collections::HashMap, iter::once};

use eframe::egui::*;
use indexmap::IndexMap;
use rapier2d::prelude::*;
use rayon::prelude::*;

use crate::{
    field::*,
    npc::{Npc, NpcId},
    object::*,
    person::{Person, PersonId},
    physics::PhysicsContext,
    player::Player,
};

pub struct World {
    pub player: Player,
    pub npcs: HashMap<NpcId, Npc>,
    pub objects: IndexMap<RigidBodyHandle, Object>,
    pub min_bound: Pos2,
    pub max_bound: Pos2,
    pub heat_grid: Vec<Vec<f32>>,
    pub physics: PhysicsContext,
    pub controls: Controls,
}

const HEAT_GRID_RESOLUTION: f32 = 0.25;
pub const DEFAULT_TEMP: f32 = 0.0;
pub const BODY_TEMP: f32 = 17.0;
pub const GRAVITY: Vec2 = vec2(0.0, -10.0);

#[derive(Default)]
pub struct Controls {
    pub x_slider: Option<f32>,
    pub y_slider: Option<f32>,
    pub activation: bool,
}

impl Controls {
    pub fn get(&self, kind: ControlKind) -> f32 {
        match kind {
            ControlKind::XSlider => self.x_slider.unwrap_or(0.0),
            ControlKind::YSlider => self.y_slider.unwrap_or(0.0),
            ControlKind::Activation => self.activation as u8 as f32,
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
            min_bound: Pos2::ZERO,
            max_bound: Pos2::ZERO,
            heat_grid: Vec::new(),
            objects: IndexMap::new(),
            controls: Controls::default(),
        };
        // Place
        world.load_place("magician_house");
        world
    }
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
    pub fn max_rect(&self) -> Rect {
        Rect::from_min_max(self.min_bound, self.max_bound)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
        let mut min_layer = ShapeLayer::Far;
        let mut best = None;
        for obj in self.objects.values() {
            if !filter(obj, &self.physics.bodies[obj.body_handle]) {
                continue;
            }
            let transformed_point = obj.transform_point(p);
            if let Some(shape) = obj
                .def
                .shapes
                .iter()
                .find(|shape| shape.contains(transformed_point))
            {
                return Some((obj, shape, ShapeLayer::Foreground));
            } else if let Some(shape) = obj
                .def
                .background
                .iter()
                .find(|shape| shape.contains(transformed_point))
            {
                if min_layer > ShapeLayer::Background {
                    min_layer = ShapeLayer::Background;
                    best = Some((obj, shape));
                }
            } else if let Some(shape) = obj
                .def
                .far
                .iter()
                .find(|shape| shape.contains(transformed_point))
            {
                min_layer = ShapeLayer::Far;
                best = Some((obj, shape));
            }
        }
        best.map(|(obj, shape)| (obj, shape, min_layer))
    }
    pub fn find_object_at(&self, p: Pos2) -> Option<(&Object, &OffsetShape, ShapeLayer)> {
        self.find_object_filtered_at(p, |_, _| true)
    }
    pub fn sample_scalar_field(
        &self,
        kind: ScalarFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> f32 {
        puffin::profile_function!(kind.to_string());
        match kind {
            ScalarFieldKind::Input(kind) => {
                self.sample_input_scalar_field(kind, pos, allow_recursion)
            }
            ScalarFieldKind::Output(kind) => {
                self.sample_output_scalar_field(kind, pos, allow_recursion)
            }
        }
    }
    pub fn sample_vector_field(
        &self,
        kind: VectorFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        match kind {
            VectorFieldKind::Input(kind) => self.sample_input_vector_field(kind, pos),
            VectorFieldKind::Output(kind) => {
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
                    test.y -= 0.25;
                }
                pos.y
            }
            ScalarInputFieldKind::Magic => {
                let mul = if let Some((obj, _, layer)) = self.find_object_at(pos) {
                    if let ShapeLayer::Foreground = layer {
                        return obj.def.props.magic;
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
                for person in self.people() {
                    for spell in person.active_spells.scalars.values().flatten() {
                        sum += spell.field.sample(self, pos, false).abs();
                    }
                    for spell in person.active_spells.vectors.values().flatten() {
                        sum += spell.field.sample(self, pos, false).length();
                    }
                }
                sum * mul
            }
            ScalarInputFieldKind::Light => self.get_light_at(pos),
            ScalarInputFieldKind::Temperature => {
                if let Some((obj, _, _)) = self.find_object_at(pos) {
                    return obj.heat;
                }
                let i = ((pos.x - self.min_bound.x) / HEAT_GRID_RESOLUTION + 0.5) as usize;
                let j = ((pos.y - self.min_bound.y) / HEAT_GRID_RESOLUTION + 0.5) as usize;
                self.heat_grid
                    .get(i)
                    .and_then(|col| col.get(j))
                    .copied()
                    .unwrap_or(DEFAULT_TEMP)
            }
        }
    }
    pub fn sample_input_vector_field(&self, kind: VectorInputFieldKind, _pos: Pos2) -> Vec2 {
        match kind {}
    }
    pub fn sample_output_scalar_field(
        &self,
        kind: ScalarOutputFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> f32 {
        puffin::profile_function!(kind.to_string());
        self.people()
            .filter_map(|person| person.active_spells.scalars.get(&kind))
            .flatten()
            .map(|spell| spell.field.sample(self, pos, allow_recursion))
            .sum()
    }
    pub fn sample_output_vector_field(
        &self,
        kind: VectorOutputFieldKind,
        pos: Pos2,
        allow_recursion: bool,
    ) -> Vec2 {
        puffin::profile_function!(kind.to_string());
        let from_spells = self
            .people()
            .filter_map(|person| person.active_spells.vectors.get(&kind))
            .flatten()
            .fold(Vec2::ZERO, |acc, spell| {
                acc + spell.field.sample(self, pos, allow_recursion)
            });
        match kind {
            VectorOutputFieldKind::Gravity => from_spells + GRAVITY,
            VectorOutputFieldKind::Force => from_spells,
        }
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
        self.run_physics();
        // Apply heat pressure
        for i in 0..self.heat_grid.len() {
            for j in 0..self.heat_grid[i].len() {
                let pos = Pos2::new(
                    self.min_bound.x + i as f32 * HEAT_GRID_RESOLUTION,
                    self.min_bound.y + j as f32 * HEAT_GRID_RESOLUTION,
                );
                let heat_pressure =
                    self.sample_output_scalar_field(ScalarOutputFieldKind::Heat, pos, true);
                self.heat_grid[i][j] += heat_pressure * 0.01;
            }
        }
        // Transer heat between objects and grid
        for obj in self.objects.values_mut() {
            let i = ((obj.pr.pos.x - self.min_bound.x) / HEAT_GRID_RESOLUTION + 0.5) as usize;
            let j = ((obj.pr.pos.y - self.min_bound.y) / HEAT_GRID_RESOLUTION + 0.5) as usize;
            if let Some(cell_heat) = self.heat_grid.get_mut(i).and_then(|col| col.get_mut(j)) {
                let diff = (obj.heat - *cell_heat) * 0.01;
                *cell_heat += diff;
                obj.heat -= diff;
            }
            if let Some(constant_heat) = obj.def.props.constant_heat {
                obj.heat = constant_heat;
            }
        }
        // Transfer heat between grid cells
        let new_grid: Vec<Vec<f32>> = self
            .heat_grid
            .par_iter()
            .enumerate()
            .map(|(i, col)| {
                let mut new_col = col.clone();
                let pos_x = self.min_bound.x + (i as f32 + 0.5) * HEAT_GRID_RESOLUTION;
                for j in 0..col.len() {
                    let pos = pos2(
                        pos_x,
                        self.min_bound.y + (j as f32 + 0.5) * HEAT_GRID_RESOLUTION,
                    );
                    let (c, s) = if self.find_object_at(pos).is_some() {
                        (4.6, 0.1)
                    } else {
                        (1.0, 1.0)
                    };
                    let center = col[j];
                    let left = self
                        .heat_grid
                        .get((i as isize - 1) as usize)
                        .map(|col| col[j])
                        .unwrap_or(DEFAULT_TEMP);
                    let right = self
                        .heat_grid
                        .get((i as isize + 1) as usize)
                        .map(|col| col[j])
                        .unwrap_or(DEFAULT_TEMP);
                    let up = *col.get((j as isize + 1) as usize).unwrap_or(&DEFAULT_TEMP);
                    let down = *col.get((j as isize - 1) as usize).unwrap_or(&DEFAULT_TEMP);
                    new_col[j] = (c * center + s * (left + right + up + down)) / 5.0;
                }
                new_col
            })
            .collect();
        self.heat_grid = new_grid;
    }
    fn hear_grid_width(&self) -> usize {
        ((self.max_bound.x - self.min_bound.x) / HEAT_GRID_RESOLUTION).ceil() as usize
    }
    fn hear_grid_height(&self) -> usize {
        ((self.max_bound.y - self.min_bound.y) / HEAT_GRID_RESOLUTION).ceil() as usize
    }
    pub fn load_place(&mut self, place_name: &str) {
        let Some(place) = PLACES.get(place_name) else {
            return;
        };
        // Set bounds
        self.min_bound.x = place.bounds.left;
        self.max_bound.x = place.bounds.right;
        self.min_bound.y = place.bounds.bottom;
        self.max_bound.y = place.bounds.top;
        // Remove old objects
        for (handle, _) in self.objects.drain(..) {
            self.physics.remove_body(handle);
        }
        // Add objects
        // Ground
        self.add_object(
            ObjectKind::Ground,
            ObjectDef::new(RigidBodyType::Fixed).shapes(
                GraphicalShape::HalfSpace(Vec2::Y)
                    .offset(Vec2::ZERO)
                    .density(3.0),
            ),
            |rb| rb,
            |c| c.restitution(0.5),
        );
        // Place objects
        for po in &place.objects {
            let object = OBJECTS[&po.name].clone();
            if let Some(repli) = &po.replication {
                for i in 0..repli.right {
                    for j in 0..repli.up {
                        let pos =
                            po.pos + vec2(repli.spacing.x * i as f32, repli.spacing.y * j as f32);
                        self.add_object_def(pos, object.clone());
                    }
                }
            } else {
                self.add_object_def(po.pos, object);
            }
        }
        // Init heat grid
        self.heat_grid = vec![vec![DEFAULT_TEMP; self.hear_grid_height()]; self.hear_grid_width()];
    }
}
