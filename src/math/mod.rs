pub use nalgebra::{
    Isometry3, Matrix2, Matrix3, Matrix4, Perspective3, Point2, Point3, Rotation2, Rotation3,
    UnitQuaternion, Vector2, Vector3, Vector4,
};

pub type Vec2 = Vector2<f32>;
pub type Vec3 = Vector3<f32>;
pub type Vec4 = Vector4<f32>;
pub type Mat2 = Matrix2<f32>;
pub type Mat3 = Matrix3<f32>;
pub type Mat4 = Matrix4<f32>;
pub type Quat = UnitQuaternion<f32>;
pub type IVec2 = Vector2<i32>;
pub type IVec3 = Vector3<i32>;
pub type IVec4 = Vector4<i32>;
pub type UVec2 = Vector2<u32>;
pub type UVec3 = Vector3<u32>;
pub type UVec4 = Vector4<u32>;
