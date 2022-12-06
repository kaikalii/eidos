use rapier2d::{na::Unit, prelude::*};

use crate::{
    field::VectorOutputFieldKind,
    math::Convert,
    world::{GraphicalShape, Object, World},
};

pub struct PhysicsContext {
    pipline: PhysicsPipeline,
    gravity: Vector<Real>,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
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
    pub fn run_physics(&mut self) {
        // Set forces
        if let Some(field) = self
            .outputs
            .vectors
            .get(&VectorOutputFieldKind::Force)
            .cloned()
        {
            for handle in self.objects.keys() {
                let pos = self.objects[handle].pos;
                let vector = field.sample(self, pos);
                let body = &mut self.physics.bodies[*handle];
                body.reset_forces(true);
                body.add_force(vector.convert(), true);
            }
        }
        // Step physics
        self.physics.step();
        // Set object positions from physics system
        for obj in self.objects.values_mut() {
            let body = self.physics.bodies.get(obj.body_handle).unwrap();
            obj.pos = body.translation().convert();
            obj.rot = body.rotation().angle();
        }
        self.player_pos = self.objects[&self.player.body_handle].pos;
    }
    pub fn add_object(
        &mut self,
        graphical_shape: GraphicalShape,
        body_builder: RigidBodyBuilder,
        build_collider: impl FnOnce(ColliderBuilder) -> ColliderBuilder,
    ) -> RigidBodyHandle {
        let body = body_builder.build();
        let shape = match &graphical_shape {
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
        };
        let collider = build_collider(ColliderBuilder::new(shape)).build();
        let pos = body.translation().convert();
        let rot = body.rotation().angle();
        let body_handle = self.physics.bodies.insert(body);
        let object = Object {
            pos,
            rot,
            shape: graphical_shape,
            shape_offset: collider.translation().convert(),
            density: collider.density(),
            body_handle,
        };
        self.physics
            .colliders
            .insert_with_parent(collider, body_handle, &mut self.physics.bodies);
        self.objects.insert(body_handle, object);
        body_handle
    }
}
