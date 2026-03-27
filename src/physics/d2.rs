use rapier2d::prelude as r;

use crate::math::Vec2;

// ── RayHit ────────────────────────────────────────────────────────────────────

/// Result of a successful ray cast in the 2D physics world.
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    /// The collider that was hit.
    pub collider: ColliderHandle,
    /// The rigid body the collider is attached to, if any.
    pub body: Option<RigidBodyHandle>,
    /// Time of impact — distance along the ray if the direction vector is
    /// normalised, otherwise `toi * dir.length()`.
    pub toi: f32,
    /// Surface normal at the hit point (points outward from the shape).
    pub normal: Vec2,
}

// ── Handles ───────────────────────────────────────────────────────────────────

/// Handle to a rigid body inside a [`World`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RigidBodyHandle(r::RigidBodyHandle);

/// Handle to a collider inside a [`World`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColliderHandle(r::ColliderHandle);

// ── BodyType ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RigidBodyType {
    Dynamic,
    Fixed,
    KinematicPositionBased,
    KinematicVelocityBased,
}

impl From<r::RigidBodyType> for RigidBodyType {
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

/// Builder for 2D rigid bodies.
pub struct RigidBodyBuilder(r::RigidBodyBuilder);

impl RigidBodyBuilder {
    pub fn dynamic() -> Self {
        Self(r::RigidBodyBuilder::dynamic())
    }
    pub fn fixed() -> Self {
        Self(r::RigidBodyBuilder::fixed())
    }
    pub fn kinematic_position_based() -> Self {
        Self(r::RigidBodyBuilder::kinematic_position_based())
    }
    pub fn kinematic_velocity_based() -> Self {
        Self(r::RigidBodyBuilder::kinematic_velocity_based())
    }

    pub fn translation(self, x: f32, y: f32) -> Self {
        Self(self.0.translation(glam::Vec2::new(x, y)))
    }
    pub fn linvel(self, x: f32, y: f32) -> Self {
        Self(self.0.linvel(glam::Vec2::new(x, y)))
    }
    pub fn angvel(self, v: f32) -> Self {
        Self(self.0.angvel(v))
    }
    pub fn gravity_scale(self, scale: f32) -> Self {
        Self(self.0.gravity_scale(scale))
    }
    pub fn linear_damping(self, d: f32) -> Self {
        Self(self.0.linear_damping(d))
    }
    pub fn angular_damping(self, d: f32) -> Self {
        Self(self.0.angular_damping(d))
    }
    pub fn can_sleep(self, v: bool) -> Self {
        Self(self.0.can_sleep(v))
    }

    fn build(self) -> r::RigidBody {
        self.0.build()
    }
}

// ── ColliderBuilder ───────────────────────────────────────────────────────────

/// Builder for 2D collider shapes.
pub struct ColliderBuilder(r::ColliderBuilder);

impl ColliderBuilder {
    pub fn ball(radius: f32) -> Self {
        Self(r::ColliderBuilder::ball(radius))
    }
    pub fn cuboid(hx: f32, hy: f32) -> Self {
        Self(r::ColliderBuilder::cuboid(hx, hy))
    }
    pub fn sensor(self, v: bool) -> Self {
        Self(self.0.sensor(v))
    }
    pub fn friction(self, v: f32) -> Self {
        Self(self.0.friction(v))
    }
    pub fn restitution(self, v: f32) -> Self {
        Self(self.0.restitution(v))
    }

    fn build(self) -> r::Collider {
        self.0.build()
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

/// 2D physics world.
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
    gravity: [f32; 2],
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self::with_gravity([0.0, -9.81])
    }

    pub fn with_gravity(gravity: [f32; 2]) -> Self {
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
        let g = r::Vector::new(self.gravity[0], self.gravity[1]);
        self.pipeline.step(
            g,
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

    // ── Add / remove ──────────────────────────────────────────────────────────

    pub fn add_body(&mut self, builder: RigidBodyBuilder) -> RigidBodyHandle {
        RigidBodyHandle(self.bodies.insert(builder.build()))
    }

    pub fn add_collider(
        &mut self,
        col: ColliderBuilder,
        parent: RigidBodyHandle,
    ) -> ColliderHandle {
        ColliderHandle(
            self.colliders
                .insert_with_parent(col.build(), parent.0, &mut self.bodies),
        )
    }

    pub fn add_free_collider(&mut self, col: ColliderBuilder) -> ColliderHandle {
        ColliderHandle(self.colliders.insert(col.build()))
    }

    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle.0,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.colliders
            .remove(handle.0, &mut self.islands, &mut self.bodies, true);
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    pub fn is_alive(&self, handle: RigidBodyHandle) -> bool {
        self.bodies.get(handle.0).is_some()
    }

    pub fn position(&self, handle: RigidBodyHandle) -> Option<Vec2> {
        let t = self.bodies.get(handle.0)?.translation();
        Some(Vec2::new(t.x, t.y))
    }

    pub fn velocity(&self, handle: RigidBodyHandle) -> Option<Vec2> {
        let v = self.bodies.get(handle.0)?.linvel();
        Some(Vec2::new(v.x, v.y))
    }

    pub fn body_type(&self, handle: RigidBodyHandle) -> Option<RigidBodyType> {
        Some(self.bodies.get(handle.0)?.body_type().into())
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    pub fn set_position(&mut self, handle: RigidBodyHandle, pos: Vec2) {
        if let Some(body) = self.bodies.get_mut(handle.0) {
            body.set_translation(r::Vector::new(pos.x, pos.y), true);
        }
    }

    pub fn set_velocity(&mut self, handle: RigidBodyHandle, vel: Vec2) {
        if let Some(body) = self.bodies.get_mut(handle.0) {
            body.set_linvel(r::Vector::new(vel.x, vel.y), true);
        }
    }

    // ── Spatial queries ───────────────────────────────────────────────────────

    /// Cast a ray and return the **closest** hit, if any.
    ///
    /// - `origin` / `dir` — ray start and direction (need not be normalised).
    /// - `max_toi` — maximum travel distance (in units of `dir.length()`).
    ///   Pass `f32::MAX` to cast indefinitely.
    /// - `solid` — if `true` a ray that starts *inside* a shape reports a hit
    ///   at `toi = 0`; if `false` it exits the shape and hits the far side.
    pub fn cast_ray(&self, origin: Vec2, dir: Vec2, max_toi: f32, solid: bool) -> Option<RayHit> {
        let qp = self.query_pipeline();
        let ray = r::Ray::new(
            r::Vector::new(origin.x, origin.y),
            r::Vector::new(dir.x, dir.y),
        );
        let (ch, hit) = qp.cast_ray_and_get_normal(&ray, max_toi, solid)?;
        Some(self.make_ray_hit(
            ch,
            hit.time_of_impact,
            Vec2::new(hit.normal.x, hit.normal.y),
        ))
    }

    /// Cast a ray and return **all** colliders it passes through (unordered).
    ///
    /// Same parameters as [`cast_ray`](Self::cast_ray).
    pub fn cast_ray_all(&self, origin: Vec2, dir: Vec2, max_toi: f32, solid: bool) -> Vec<RayHit> {
        let qp = self.query_pipeline();
        let ray = r::Ray::new(
            r::Vector::new(origin.x, origin.y),
            r::Vector::new(dir.x, dir.y),
        );
        qp.intersect_ray(ray, max_toi, solid)
            .map(|(ch, _, hit)| {
                self.make_ray_hit(
                    ch,
                    hit.time_of_impact,
                    Vec2::new(hit.normal.x, hit.normal.y),
                )
            })
            .collect()
    }

    /// Return all colliders whose shape **contains** `point`.
    pub fn intersect_point(&self, point: Vec2) -> Vec<ColliderHandle> {
        let qp = self.query_pipeline();
        qp.intersect_point(r::Vector::new(point.x, point.y))
            .map(|(ch, _)| ColliderHandle(ch))
            .collect()
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn query_pipeline(&self) -> r::QueryPipeline<'_> {
        self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            r::QueryFilter::default(),
        )
    }

    fn make_ray_hit(&self, ch: r::ColliderHandle, toi: f32, normal: Vec2) -> RayHit {
        let body = self
            .colliders
            .get(ch)
            .and_then(|c| c.parent())
            .map(RigidBodyHandle);
        RayHit {
            collider: ColliderHandle(ch),
            body,
            toi,
            normal,
        }
    }
}
