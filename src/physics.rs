use eframe::epaint::{pos2, vec2, Pos2, Vec2};
use rapier2d::{
    na::{Unit, Vector2},
    prelude::*,
};

use crate::{
    game::Game,
    world::{GraphicalShape, Object},
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

impl Game {
    pub fn initialize_physics(&mut self) {
        // Ground
        self.add_object(
            GraphicalShape::HalfSpace(Vec2::Y),
            RigidBodyBuilder::fixed(),
            |c| c.density(3.0),
        );

        self.add_object(
            GraphicalShape::Circle(1.0),
            RigidBodyBuilder::dynamic().translation([3.0, 10.0].into()),
            |c| c.density(2.0).restitution(1.0),
        );
        // Player
        self.player.body_handle = self.add_object(
            GraphicalShape::Capsule {
                half_height: 0.25,
                radius: 0.25,
            },
            RigidBodyBuilder::dynamic().translation([0.0, 0.5].into()),
            |c| c.density(1.0),
        );
    }
    pub fn run_physics(&mut self) {
        self.physics.step();
        for obj in self.world.objects.values_mut() {
            obj.pos = self
                .physics
                .bodies
                .get(obj.body_handle)
                .unwrap()
                .translation()
                .convert();
        }
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
            GraphicalShape::Box(size) => SharedShape::new(Cuboid::new(size.convert())),
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
        let body_handle = self.physics.bodies.insert(body);
        let object = Object {
            pos,
            shape: graphical_shape,
            shape_offset: collider.translation().convert(),
            density: collider.density(),
            body_handle,
        };
        self.physics
            .colliders
            .insert_with_parent(collider, body_handle, &mut self.physics.bodies);
        self.world.objects.insert(body_handle, object);
        body_handle
    }
}

pub trait Convert<U> {
    fn convert(self) -> U;
}

impl Convert<Vector2<f32>> for Vec2 {
    fn convert(self) -> Vector2<f32> {
        vector!(self.x, self.y)
    }
}

impl Convert<Vec2> for Vector2<f32> {
    fn convert(self) -> Vec2 {
        vec2(self.x, self.y)
    }
}

impl Convert<Vector2<f32>> for Pos2 {
    fn convert(self) -> Vector2<f32> {
        vector!(self.x, self.y)
    }
}

impl Convert<Pos2> for Vector2<f32> {
    fn convert(self) -> Pos2 {
        pos2(self.x, self.y)
    }
}

impl Convert<Point<f32>> for Pos2 {
    fn convert(self) -> Point<f32> {
        [self.x, self.y].into()
    }
}

impl Convert<Pos2> for Point<f32> {
    fn convert(self) -> Pos2 {
        pos2(self.x, self.y)
    }
}
