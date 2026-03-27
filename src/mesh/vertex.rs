use crate::math::Mat4;
use crate::renderer::{VertexAttribute, VertexFormat, VertexLayout};

/// A single vertex in a mesh.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl MeshVertex {
    pub fn layout() -> VertexLayout {
        use std::mem::offset_of;
        VertexLayout {
            stride: std::mem::size_of::<Self>() as u64,
            attributes: vec![
                VertexAttribute {
                    location: 0,
                    offset: offset_of!(Self, position) as u64,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    location: 1,
                    offset: offset_of!(Self, normal) as u64,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    location: 2,
                    offset: offset_of!(Self, uv) as u64,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Vertex with skinning data for skeletal animation.
///
/// `joints` contains up to 4 joint indices; `weights` are the corresponding
/// blend weights (should sum to 1.0). Both are sourced from `JOINTS_0` and
/// `WEIGHTS_0` glTF attributes.
///
/// In WGSL, declare the vertex inputs as:
/// ```wgsl
/// @location(0) position: vec3<f32>,
/// @location(1) normal:   vec3<f32>,
/// @location(2) uv:       vec2<f32>,
/// @location(3) joints:   vec4<u32>,  // Uint8x4 — values 0–255
/// @location(4) weights:  vec4<f32>,
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// Joint indices (up to 4), each in range 0–255.
    pub joints: [u8; 4],
    /// Blend weights for each joint (should sum to 1.0).
    pub weights: [f32; 4],
}

impl SkinnedVertex {
    pub fn layout() -> VertexLayout {
        use std::mem::offset_of;
        VertexLayout {
            stride: std::mem::size_of::<Self>() as u64,
            attributes: vec![
                VertexAttribute {
                    location: 0,
                    offset: offset_of!(Self, position) as u64,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    location: 1,
                    offset: offset_of!(Self, normal) as u64,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    location: 2,
                    offset: offset_of!(Self, uv) as u64,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    location: 3,
                    offset: offset_of!(Self, joints) as u64,
                    format: VertexFormat::Uint8x4,
                },
                VertexAttribute {
                    location: 4,
                    offset: offset_of!(Self, weights) as u64,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Raw RGBA8 image data returned from a glTF material.
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels, row-major.
    pub data: Vec<u8>,
}

/// A single mesh primitive (triangles).
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    /// World-space transform accumulated from the node hierarchy.
    pub transform: Mat4,
    /// Base color texture from the primitive's material, if present.
    pub base_color: Option<Image>,
}

/// A mesh primitive with per-vertex skinning data.
pub struct SkinnedMesh {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    /// Node transform from the glTF hierarchy (does not include skeletal deformation).
    pub transform: Mat4,
    /// Base color texture, if present in the material.
    pub base_color: Option<Image>,
}
