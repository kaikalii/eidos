use std::panic::{catch_unwind, AssertUnwindSafe};

use eframe::epaint::{Pos2, Vec2};
use itertools::Itertools;
use rapier2d::{na::Unit, prelude::*};

use crate::{
    field::VectorOutputFieldKind,
    math::Convert,
    object::{GraphicalBinding, GraphicalShape, Object, ObjectDef, ObjectKind},
    world::{World, DEFAULT_TEMP},
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

impl World {
    /// Run a physics step
    pub fn run_physics(&mut self) {
        puffin::profile_function!();
        // Set forces
        for &handle in self.objects.keys().collect_vec() {
            if !self.physics.bodies[handle].is_dynamic() {
                continue;
            }
            let pos = self.objects[&handle].pos;
            let gravity_acc =
                self.sample_output_vector_field(VectorOutputFieldKind::Gravity, pos, true);
            let field_force =
                self.sample_output_vector_field(VectorOutputFieldKind::Force, pos, true);
            let body = &mut self.physics.bodies[handle];
            let gravity_force = gravity_acc * body.mass();
            let total_force = field_force + gravity_force;
            body.reset_forces(true);
            body.add_force(total_force.convert(), true);
        }
        // Step physics
        self.physics.step();
        // Set object positions from physics system
        for obj in self.objects.values_mut() {
            let body = self.physics.bodies.get(obj.body_handle).unwrap();
            obj.pos = body.translation().convert();
            obj.vel = body
                .velocity_at_point(&Point::from(*body.translation()))
                .convert();
            obj.rot = body.rotation().angle();
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

const PERSON: Group = Group::GROUP_1;
const OBJECT: Group = Group::GROUP_2;
const BACKGROUND: Group = Group::GROUP_3;
const GROUND: Group = Group::GROUP_4;

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
        let body = body_builder(RigidBodyBuilder::new(def.ty))
            .linear_damping(0.5)
            .angular_damping(1.0)
            .build();
        let pos = body.translation().convert();
        let rot = body.rotation().angle();
        let body_handle = self.physics.bodies.insert(body);
        let groups = match kind {
            ObjectKind::Player => InteractionGroups::new(PERSON, OBJECT | GROUND),
            ObjectKind::Npc => InteractionGroups::new(PERSON, OBJECT | GROUND),
            ObjectKind::Object => InteractionGroups::new(OBJECT, PERSON | OBJECT | GROUND),
            ObjectKind::Ground => InteractionGroups::new(GROUND, PERSON | OBJECT | BACKGROUND),
        };
        let mut foreground_handles = Vec::new();
        let mut background_handles = Vec::new();
        for offset_shape in &def.shapes {
            let shared_shape = graphical_shape_to_shared(&offset_shape.shape);
            let collider = build_collider(ColliderBuilder::new(shared_shape))
                .translation(offset_shape.offset.convert())
                .density(offset_shape.density)
                .collision_groups(groups)
                .build();
            foreground_handles.push(self.physics.colliders.insert_with_parent(
                collider,
                body_handle,
                &mut self.physics.bodies,
            ));
        }
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
        let object = Object {
            kind,
            heat: def.props.constant_heat.unwrap_or(DEFAULT_TEMP),
            def,
            pos,
            vel: Vec2::ZERO,
            rot,
            body_handle,
            foreground_handles,
            background_handles,
            binding: match kind {
                ObjectKind::Npc => GraphicalBinding::Npc,
                _ => GraphicalBinding::Linear,
            },
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
            let dist = light_obj.pos.distance(pos);
            let ray = Ray::new(pos.convert(), (light_obj.pos - pos).normalized().convert());
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
                        if matches!(obj.kind, ObjectKind::Player | ObjectKind::Npc)
                            || obj.background_handles.contains(&handle)
                        {
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
