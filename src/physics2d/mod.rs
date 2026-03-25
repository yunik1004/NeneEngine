use rapier2d::prelude::*;

pub use rapier2d::prelude::{
    ColliderBuilder, ColliderHandle, RigidBody, RigidBodyHandle, RigidBodyType,
};

/// Builder for 2D rigid bodies.
pub struct RigidBodyBuilder(rapier2d::prelude::RigidBodyBuilder);

impl RigidBodyBuilder {
    pub fn dynamic() -> Self {
        Self(rapier2d::prelude::RigidBodyBuilder::dynamic())
    }
    pub fn fixed() -> Self {
        Self(rapier2d::prelude::RigidBodyBuilder::fixed())
    }
    pub fn kinematic_position_based() -> Self {
        Self(rapier2d::prelude::RigidBodyBuilder::kinematic_position_based())
    }
    pub fn kinematic_velocity_based() -> Self {
        Self(rapier2d::prelude::RigidBodyBuilder::kinematic_velocity_based())
    }

    pub fn translation(self, x: f32, y: f32) -> Self {
        Self(self.0.translation(glam::Vec2::new(x, y)))
    }
    pub fn linvel(self, x: f32, y: f32) -> Self {
        Self(self.0.linvel(glam::Vec2::new(x, y)))
    }
    pub fn angvel(self, angvel: f32) -> Self {
        Self(self.0.angvel(angvel))
    }
    pub fn gravity_scale(self, scale: f32) -> Self {
        Self(self.0.gravity_scale(scale))
    }
    pub fn linear_damping(self, damping: f32) -> Self {
        Self(self.0.linear_damping(damping))
    }
    pub fn angular_damping(self, damping: f32) -> Self {
        Self(self.0.angular_damping(damping))
    }
    pub fn can_sleep(self, can_sleep: bool) -> Self {
        Self(self.0.can_sleep(can_sleep))
    }
    pub fn build(self) -> RigidBody {
        self.0.build()
    }
}

/// 2D physics world. Manages rigid bodies, colliders, and simulation stepping.
pub struct World {
    bodies: RigidBodySet,
    colliders: ColliderSet,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    params: IntegrationParameters,
    gravity: [f32; 2],
}

impl World {
    pub fn new() -> Self {
        Self::with_gravity([0.0, -9.81])
    }

    pub fn with_gravity(gravity: [f32; 2]) -> Self {
        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            params: IntegrationParameters::default(),
            gravity,
        }
    }

    pub fn step(&mut self) {
        let gravity = rapier2d::math::Vector::new(self.gravity[0], self.gravity[1]);
        self.pipeline.step(
            gravity,
            &self.params,
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
    }

    pub fn step_dt(&mut self, dt: f32) {
        self.params.dt = dt;
        self.step();
    }

    pub fn add_body(&mut self, body: RigidBody) -> RigidBodyHandle {
        self.bodies.insert(body)
    }

    pub fn add_collider(&mut self, collider: Collider, parent: RigidBodyHandle) -> ColliderHandle {
        self.colliders
            .insert_with_parent(collider, parent, &mut self.bodies)
    }

    pub fn add_free_collider(&mut self, collider: Collider) -> ColliderHandle {
        self.colliders.insert(collider)
    }

    pub fn body(&self, handle: RigidBodyHandle) -> Option<&RigidBody> {
        self.bodies.get(handle)
    }

    pub fn body_mut(&mut self, handle: RigidBodyHandle) -> Option<&mut RigidBody> {
        self.bodies.get_mut(handle)
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

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.colliders
            .remove(handle, &mut self.islands, &mut self.bodies, true);
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
