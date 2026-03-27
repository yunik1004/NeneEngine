/// Whether a rigid body is simulated, fixed, or kinematically controlled.
///
/// Shared by both 2D ([`d2`](super::d2)) and 3D ([`d3`](super::d3)) worlds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RigidBodyType {
    Dynamic,
    Fixed,
    KinematicPositionBased,
    KinematicVelocityBased,
}
