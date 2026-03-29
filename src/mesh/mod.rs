mod model;
pub mod primitives;
mod renderer;
mod skeleton;
mod vertex;

pub use model::{Model, ModelError};
pub use primitives::{
    CubeBuilder, CylinderBuilder, QuadBuilder, SphereBuilder,
    circle, circle_segments, cube, cylinder, line, quad, rect, rect_outline,
    sphere, triangle, unit_cube, unit_quad, unit_sphere,
};
pub use renderer::{ColorMesh, LitShadowedModel};
pub use skeleton::{AnimChannel, Channel, Clip, Joint, JointPose, Skeleton};
pub use vertex::{Image, Mesh, Vertex};
