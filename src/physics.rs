use std::panic::{catch_unwind, AssertUnwindSafe};

use eframe::epaint::{Pos2, Vec2};
use itertools::Itertools;
use rapier2d::{na::Unit, prelude::*};

use crate::{
    field::*,
    math::{angle_diff, Convert},
    object::*,
    world::{World, ABSOLUTE_ZERO, AIR_DENSITY_AT_GROUND_TEMP, GROUND_TEMP},
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
    pub queries: QueryPipeline,
}

impl Default for PhysicsContext {
    fn default() -> Self {
        PhysicsContext {
            pipline: PhysicsPipeline::default(),
            gravity: vector!(0.0, 0.0),
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::default(),
            broad_phase: BroadPhase::default(),
            narrow_phase: NarrowPhase::default(),
            bodies: RigidBodySet::default(),
            colliders: ColliderSet::default(),
            impulse_joints: ImpulseJointSet::default(),
            multibody_joints: MultibodyJointSet::default(),
            ccd_solver: CCDSolver::default(),
            queries: QueryPipeline::default(),
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
        );
        self.queries
            .update(&self.islands, &self.bodies, &self.colliders);
    }
    pub fn dt(&self) -> f32 {
        self.integration_parameters.dt
    }
    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }
}

fn air_density_at_temp(temp: f32) -> f32 {
    (GROUND_TEMP - ABSOLUTE_ZERO) / (temp - ABSOLUTE_ZERO) * AIR_DENSITY_AT_GROUND_TEMP
}

impl World {
    /// Run a physics step
    pub fn run_physics(&mut self) {
        puffin::profile_function!();
        // Set forces
        for &handle in self.objects.keys().collect_vec() {
            if !self.physics.bodies[handle].is_dynamic() {
                continue;
            }
            let pos = self.objects[&handle].pr.pos;
            let gravity_acc =
                self.sample_output_vector_field(VectorOutputFieldKind::Gravity, pos, true);
            let field_force =
                self.sample_output_vector_field(VectorOutputFieldKind::Force, pos, true);
            let order = self.sample_output_scalar_field(ScalarOutputFieldKind::Order, pos, true);
            let obj = &self.objects[&handle];
            let diff = obj.ordered_pr.pos - obj.pr.pos;
            let order_force = order
                * diff.length()
                * diff.normalized()
                * (-0.5 * diff.normalized().dot(obj.vel.normalized()) + 1.5);
            let temp = self.temperature_at(pos);
            let body = &mut self.physics.bodies[handle];
            let gravity_force = gravity_acc * body.mass();
            let volume: f32 = body
                .colliders()
                .iter()
                .map(|&handle| self.physics.colliders[handle].volume())
                .sum();
            let buoyant_force = -air_density_at_temp(temp) * volume * gravity_acc;
            let total_force = field_force + gravity_force + buoyant_force + order_force;
            let sensor =
                order > 0.0 && order_force.length() > gravity_force.length() + field_force.length();
            for &collider_handle in body.colliders() {
                let collider = self.physics.colliders.get_mut(collider_handle).unwrap();
                collider.set_sensor(sensor);
            }
            body.reset_forces(true);
            body.add_force(total_force.convert(), true);
            body.reset_torques(true);
            if order.abs() > 0.0 {
                let angle = angle_diff(obj.pr.rot, obj.ordered_pr.rot);
                let order_torque = order * angle;
                body.add_torque(order_torque, true);
            }
        }
        // Step physics
        self.physics.step();
        // Set object positions from physics system
        for obj in self.objects.values_mut() {
            let body = self.physics.bodies.get(obj.body_handle).unwrap();
            obj.pr.pos = body.translation().convert();
            obj.vel = body
                .velocity_at_point(&Point::from(*body.translation()))
                .convert();
            obj.pr.rot = body.rotation().angle();
        }
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

const OBJECT: Group = Group::GROUP_1;
const BACKGROUND: Group = Group::GROUP_2;
const GROUND: Group = Group::GROUP_3;

impl World {
    pub fn add_object_def(&mut self, pos: Pos2, def: ObjectDef) {
        self.add_object(
            ObjectKind::Object,
            def,
            |rb| rb.translation(pos.convert()),
            |c| c,
        );
    }
    pub fn add_object(
        &mut self,
        kind: ObjectKind,
        def: ObjectDef,
        body_builder: impl Fn(RigidBodyBuilder) -> RigidBodyBuilder,
        build_collider: impl Fn(ColliderBuilder) -> ColliderBuilder,
    ) -> RigidBodyHandle {
        // Create body
        let body = body_builder(RigidBodyBuilder::new(def.ty))
            .linear_damping(0.5)
            .angular_damping(1.0)
            .build();
        let pos = body.translation().convert();
        let rot = body.rotation().angle();
        let body_handle = self.physics.bodies.insert(body);
        // Create colliders
        let foreground_groups = match kind {
            ObjectKind::Object => InteractionGroups::new(OBJECT, OBJECT | GROUND),
            ObjectKind::Ground => InteractionGroups::new(GROUND, OBJECT | BACKGROUND),
        };
        let mut foreground_handles = Vec::new();
        let mut background_handles = Vec::new();
        // Foreground colliders
        for offset_shape in &def.shapes {
            let shared_shape = graphical_shape_to_shared(&offset_shape.shape);
            let collider = build_collider(ColliderBuilder::new(shared_shape))
                .translation(offset_shape.offset.convert())
                .density(offset_shape.density)
                .collision_groups(foreground_groups)
                .build();
            foreground_handles.push(self.physics.colliders.insert_with_parent(
                collider,
                body_handle,
                &mut self.physics.bodies,
            ));
        }
        // Background colliders
        for offset_shape in &def.background {
            let shared_shape = graphical_shape_to_shared(&offset_shape.shape);
            let collider = build_collider(ColliderBuilder::new(shared_shape))
                .translation(offset_shape.offset.convert())
                .density(offset_shape.density)
                .collision_groups(InteractionGroups::new(BACKGROUND, BACKGROUND | GROUND))
                .build();
            background_handles.push(self.physics.colliders.insert_with_parent(
                collider,
                body_handle,
                &mut self.physics.bodies,
            ));
        }
        // Create object
        let transform = PosRot { pos, rot };
        let object = Object {
            kind,
            heat: def.props.constant_heat.unwrap_or(GROUND_TEMP),
            def,
            pr: transform,
            ordered_pr: transform,
            vel: Vec2::ZERO,
            body_handle,
            foreground_handles,
            background_handles,
        };
        self.objects.insert(body_handle, object);
        self.objects.sort_by(|_, a, _, b| a.kind.cmp(&b.kind));
        body_handle
    }
    pub fn get_light_at(&self, pos: Pos2) -> f32 {
        let mut max = 0f32;
        for light_obj in self.objects.values() {
            if light_obj.def.props.light == 0.0 {
                continue;
            }
            let dist = light_obj.pr.pos.distance(pos);
            let ray = Ray::new(
                pos.convert(),
                (light_obj.pr.pos - pos).normalized().convert(),
            );
            let mut soft_count = 0;
            let mut hard = false;
            let _ = catch_unwind(AssertUnwindSafe(|| {
                self.physics.queries.intersections_with_ray(
                    &self.physics.bodies,
                    &self.physics.colliders,
                    &ray,
                    dist,
                    true,
                    QueryFilter::default().exclude_rigid_body(light_obj.body_handle),
                    |handle, _| {
                        let body_handle = self.physics.colliders[handle].parent().unwrap();
                        let obj = &self.objects[&body_handle];
                        if obj.background_handles.contains(&handle) {
                            soft_count += 1;
                            true
                        } else {
                            hard = true;
                            false
                        }
                    },
                );
            }));
            if hard {
                continue;
            }
            let intensity =
                light_obj.def.props.light / (1.0 + dist.powi(2)) / (soft_count + 1) as f32;
            max = max.max(intensity);
        }
        max
    }
}
