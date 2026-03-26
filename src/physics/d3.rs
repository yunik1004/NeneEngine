use rapier3d::prelude as r;

use crate::math::Vec3;

// ── Handles ───────────────────────────────────────────────────────────────────

/// Handle to a rigid body inside a [`World`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BodyHandle(r::RigidBodyHandle);

/// Handle to a collider inside a [`World`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColliderHandle(r::ColliderHandle);

// ── BodyType ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodyType {
    Dynamic,
    Fixed,
    KinematicPositionBased,
    KinematicVelocityBased,
}

impl From<r::RigidBodyType> for BodyType {
    fn from(t: r::RigidBodyType) -> Self {
        match t {
            r::RigidBodyType::Dynamic => Self::Dynamic,
            r::RigidBodyType::Fixed => Self::Fixed,
            r::RigidBodyType::KinematicPositionBased => Self::KinematicPositionBased,
            r::RigidBodyType::KinematicVelocityBased => Self::KinematicVelocityBased,
        }
    }
}

// ── BodyBuilder ───────────────────────────────────────────────────────────────

/// Builder for 3D rigid bodies.
pub struct BodyBuilder(r::RigidBodyBuilder);

impl BodyBuilder {
    pub fn dynamic() -> Self { Self(r::RigidBodyBuilder::dynamic()) }
    pub fn fixed() -> Self { Self(r::RigidBodyBuilder::fixed()) }
    pub fn kinematic_position_based() -> Self { Self(r::RigidBodyBuilder::kinematic_position_based()) }
    pub fn kinematic_velocity_based() -> Self { Self(r::RigidBodyBuilder::kinematic_velocity_based()) }

    pub fn translation(self, x: f32, y: f32, z: f32) -> Self {
        Self(self.0.translation(glam::Vec3::new(x, y, z)))
    }
    pub fn linvel(self, x: f32, y: f32, z: f32) -> Self {
        Self(self.0.linvel(glam::Vec3::new(x, y, z)))
    }
    pub fn angvel(self, x: f32, y: f32, z: f32) -> Self {
        Self(self.0.angvel(glam::Vec3::new(x, y, z)))
    }
    pub fn gravity_scale(self, scale: f32) -> Self { Self(self.0.gravity_scale(scale)) }
    pub fn linear_damping(self, d: f32) -> Self { Self(self.0.linear_damping(d)) }
    pub fn angular_damping(self, d: f32) -> Self { Self(self.0.angular_damping(d)) }
    pub fn can_sleep(self, v: bool) -> Self { Self(self.0.can_sleep(v)) }

    fn build(self) -> r::RigidBody { self.0.build() }
}

// ── ColliderBuilder ───────────────────────────────────────────────────────────

/// Builder for 3D collider shapes.
pub struct ColliderBuilder(r::ColliderBuilder);

impl ColliderBuilder {
    pub fn ball(radius: f32) -> Self { Self(r::ColliderBuilder::ball(radius)) }
    pub fn cuboid(hx: f32, hy: f32, hz: f32) -> Self { Self(r::ColliderBuilder::cuboid(hx, hy, hz)) }
    pub fn sensor(self, v: bool) -> Self { Self(self.0.sensor(v)) }
    pub fn friction(self, v: f32) -> Self { Self(self.0.friction(v)) }
    pub fn restitution(self, v: f32) -> Self { Self(self.0.restitution(v)) }

    fn build(self) -> r::Collider { self.0.build() }
}

// ── World ─────────────────────────────────────────────────────────────────────

/// 3D physics world.
pub struct World {
    bodies: r::RigidBodySet,
    colliders: r::ColliderSet,
    pipeline: r::PhysicsPipeline,
    islands: r::IslandManager,
    broad_phase: r::DefaultBroadPhase,
    narrow_phase: r::NarrowPhase,
    impulse_joints: r::ImpulseJointSet,
    multibody_joints: r::MultibodyJointSet,
    ccd_solver: r::CCDSolver,
    params: r::IntegrationParameters,
    gravity: [f32; 3],
}

impl Default for World {
    fn default() -> Self { Self::new() }
}

impl World {
    pub fn new() -> Self { Self::with_gravity([0.0, -9.81, 0.0]) }

    pub fn with_gravity(gravity: [f32; 3]) -> Self {
        Self {
            bodies: r::RigidBodySet::new(),
            colliders: r::ColliderSet::new(),
            pipeline: r::PhysicsPipeline::new(),
            islands: r::IslandManager::new(),
            broad_phase: r::DefaultBroadPhase::new(),
            narrow_phase: r::NarrowPhase::new(),
            impulse_joints: r::ImpulseJointSet::new(),
            multibody_joints: r::MultibodyJointSet::new(),
            ccd_solver: r::CCDSolver::new(),
            params: r::IntegrationParameters::default(),
            gravity,
        }
    }

    pub fn step(&mut self) {
        let g = r::Vector::new(self.gravity[0], self.gravity[1], self.gravity[2]);
        self.pipeline.step(
            g, &self.params,
            &mut self.islands, &mut self.broad_phase, &mut self.narrow_phase,
            &mut self.bodies, &mut self.colliders,
            &mut self.impulse_joints, &mut self.multibody_joints,
            &mut self.ccd_solver, &(), &(),
        );
    }

    pub fn step_dt(&mut self, dt: f32) {
        self.params.dt = dt;
        self.step();
    }

    // ── Add / remove ──────────────────────────────────────────────────────────

    pub fn add_body(&mut self, builder: BodyBuilder) -> BodyHandle {
        BodyHandle(self.bodies.insert(builder.build()))
    }

    pub fn add_collider(&mut self, col: ColliderBuilder, parent: BodyHandle) -> ColliderHandle {
        ColliderHandle(self.colliders.insert_with_parent(col.build(), parent.0, &mut self.bodies))
    }

    pub fn add_free_collider(&mut self, col: ColliderBuilder) -> ColliderHandle {
        ColliderHandle(self.colliders.insert(col.build()))
    }

    pub fn remove_body(&mut self, handle: BodyHandle) {
        self.bodies.remove(
            handle.0, &mut self.islands, &mut self.colliders,
            &mut self.impulse_joints, &mut self.multibody_joints, true,
        );
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.colliders.remove(handle.0, &mut self.islands, &mut self.bodies, true);
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    pub fn is_alive(&self, handle: BodyHandle) -> bool {
        self.bodies.get(handle.0).is_some()
    }

    pub fn position(&self, handle: BodyHandle) -> Option<Vec3> {
        let t = self.bodies.get(handle.0)?.translation();
        Some(Vec3::new(t.x, t.y, t.z))
    }

    pub fn velocity(&self, handle: BodyHandle) -> Option<Vec3> {
        let v = self.bodies.get(handle.0)?.linvel();
        Some(Vec3::new(v.x, v.y, v.z))
    }

    pub fn body_type(&self, handle: BodyHandle) -> Option<BodyType> {
        Some(self.bodies.get(handle.0)?.body_type().into())
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    pub fn set_position(&mut self, handle: BodyHandle, pos: Vec3) {
        if let Some(body) = self.bodies.get_mut(handle.0) {
            body.set_translation(r::Vector::new(pos.x, pos.y, pos.z), true);
        }
    }

    pub fn set_velocity(&mut self, handle: BodyHandle, vel: Vec3) {
        if let Some(body) = self.bodies.get_mut(handle.0) {
            body.set_linvel(r::Vector::new(vel.x, vel.y, vel.z), true);
        }
    }
}
