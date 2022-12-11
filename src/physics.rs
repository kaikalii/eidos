use std::{
    collections::HashMap,
    f32::consts::{PI, TAU},
};

use eframe::epaint::Vec2;
use itertools::Itertools;
use rapier2d::{na::Unit, prelude::*};

use crate::{
    field::VectorOutputFieldKind,
    math::{modulus, Convert},
    world::{GraphicalShape, Object, OffsetShape, Properties, World},
};

pub struct PhysicsContext {
    pipline: PhysicsPipeline,
    gravity: Vector<Real>,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
}

impl Default for PhysicsContext {
    fn default() -> Self {
        PhysicsContext {
            pipline: PhysicsPipeline::default(),
            gravity: vector!(0.0, -9.81),
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::default(),
            broad_phase: BroadPhase::default(),
            narrow_phase: NarrowPhase::default(),
            bodies: RigidBodySet::default(),
            colliders: ColliderSet::default(),
            impulse_joints: ImpulseJointSet::default(),
            multibody_joints: MultibodyJointSet::default(),
            ccd_solver: CCDSolver::default(),
        }
    }
}

impl PhysicsContext {
    pub fn step(&mut self) {
        self.pipline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        )
    }
}

impl World {
    /// Run a physics step and return the amount of work done by output fields
    #[must_use]
    pub fn run_physics(&mut self) -> f32 {
        puffin::profile_function!();
        // Set forces
        let mut forces = HashMap::new();
        for &handle in self.objects.keys().collect_vec() {
            if !self.physics.bodies[handle].is_dynamic() {
                continue;
            }
            let pos = self.objects[&handle].pos;
            let vector = self.sample_output_vector_field(VectorOutputFieldKind::Force, pos);
            let body = &mut self.physics.bodies[handle];
            body.reset_forces(true);
            body.add_force(vector.convert(), true);
            if self.player.person.body_handle == handle
                || self
                    .npcs
                    .values()
                    .any(|npc| npc.person.body_handle == handle)
            {
                body.reset_torques(true);
                let angle = modulus(body.rotation().angle() + PI, TAU) - PI;
                let torque = -(angle / PI) * 0.05;
                body.add_torque(torque, true);
            }
            forces.insert(handle, vector);
        }
        // Step physics
        self.physics.step();
        // Set object positions from physics system
        let mut total_work = 0.0;
        for (handle, obj) in self.objects.iter_mut() {
            let body = self.physics.bodies.get(obj.body_handle).unwrap();
            let old_pos = obj.pos;
            obj.pos = body.translation().convert();
            let dpos = obj.pos - old_pos;
            obj.rot = body.rotation().angle();
            // Calculate work
            if dpos.length() > 0.0 {
                if let Some(force) = forces.get(handle).copied() {
                    let work_done = force.dot(dpos);
                    if work_done.abs() > 0.0 {
                        total_work += work_done
                    }
                }
            }
        }
        // Set player pos
        for id in self.person_ids() {
            self.person_mut(id).pos = self.objects[&self.person(id).body_handle].pos;
        }
        total_work
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

fn graphical_shape_to_shared(shape: &GraphicalShape) -> SharedShape {
    match shape {
        GraphicalShape::Circle(radius) => SharedShape::new(Ball::new(*radius)),
        GraphicalShape::Box(size) => SharedShape::new(Cuboid::new((*size * 0.5).convert())),
        GraphicalShape::HalfSpace(normal) => {
            SharedShape::new(HalfSpace::new(Unit::new_normalize(normal.convert())))
        }
        GraphicalShape::Capsule {
            half_height,
            radius,
        } => SharedShape::new(Capsule::new(
            [0.0, *half_height].into(),
            [0.0, -*half_height].into(),
            *radius,
        )),
    }
}

impl World {
    pub fn add_object(
        &mut self,
        props: Properties,
        shapes: impl IntoShapes,
        body_builder: RigidBodyBuilder,
        build_collider: impl Fn(ColliderBuilder) -> ColliderBuilder,
    ) -> RigidBodyHandle {
        let body = body_builder
            .linear_damping(0.5)
            .angular_damping(1.0)
            .build();
        let offset_shapes = shapes.into_shapes();
        let pos = body.translation().convert();
        let rot = body.rotation().angle();
        let body_handle = self.physics.bodies.insert(body);
        for offset_shape in &offset_shapes {
            let shared_shape = graphical_shape_to_shared(&offset_shape.shape);
            let collider = build_collider(ColliderBuilder::new(shared_shape))
                .translation(offset_shape.offset.convert())
                .density(offset_shape.density)
                .build();
            self.physics.colliders.insert_with_parent(
                collider,
                body_handle,
                &mut self.physics.bodies,
            );
        }
        let object = Object {
            props,
            pos,
            rot,
            shapes: offset_shapes,
            body_handle,
        };
        self.objects.insert(body_handle, object);
        body_handle
    }
}
