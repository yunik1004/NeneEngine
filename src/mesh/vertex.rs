use crate::math::Mat4;
use crate::renderer::{VertexAttribute, VertexFormat, VertexLayout};

/// A unified mesh vertex covering all rendering modes.
///
/// Fill only the fields your use case needs:
/// - **Colored geometry** — set `position` and `color`; leave `normal`/`uv` at defaults.
/// - **Textured / lit geometry** — set `position`, `normal`, `uv`; `color` defaults to opaque white.
/// - **Skeletal animation** — set all fields including `joints` and `weights`.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal:   [f32; 3],
    pub uv:       [f32; 2],
    /// Per-vertex RGBA color. Defaults to opaque white `[1, 1, 1, 1]`.
    pub color:    [f32; 4],
    /// Joint indices for skeletal animation (up to 4). Zero for static meshes.
    pub joints:   [u8; 4],
    /// Blend weights for each joint (should sum to 1.0). Zero for static meshes.
    pub weights:  [f32; 4],
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            normal:   [0.0, 1.0, 0.0],
            uv:       [0.0, 0.0],
            color:    [1.0, 1.0, 1.0, 1.0],
            joints:   [0; 4],
            weights:  [0.0; 4],
        }
    }
}

impl Vertex {
    /// Vertex buffer layout for wgpu pipelines.
    ///
    /// Slot assignments:
    /// - `@location(0)` position
    /// - `@location(1)` normal
    /// - `@location(2)` uv
    /// - `@location(3)` color
    /// - `@location(4)` joints  (Uint8x4)
    /// - `@location(5)` weights
    pub(crate) fn layout() -> VertexLayout {
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
                    offset: offset_of!(Self, color) as u64,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    location: 4,
                    offset: offset_of!(Self, joints) as u64,
                    format: VertexFormat::Uint8x4,
                },
                VertexAttribute {
                    location: 5,
                    offset: offset_of!(Self, weights) as u64,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Raw RGBA8 image data returned from a model material.
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels, row-major.
    pub data: Vec<u8>,
}

/// A CPU-side mesh primitive (triangles).
///
/// Obtain from [`Model::load`] or construct directly.
/// Upload to the GPU with [`GpuMesh::from_mesh`](crate::renderer::GpuMesh::from_mesh).
pub struct Mesh {
    pub vertices:   Vec<Vertex>,
    pub indices:    Vec<u32>,
    /// World-space transform accumulated from the node hierarchy (set by the model loader).
    pub transform:  Mat4,
    /// Base colour texture from the model material, if present.
    pub base_color: Option<Image>,
    /// Whether `joints` and `weights` carry meaningful skeletal animation data.
    pub skinned:    bool,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            indices,
            transform: Mat4::IDENTITY,
            base_color: None,
            skinned: false,
        }
    }
}
