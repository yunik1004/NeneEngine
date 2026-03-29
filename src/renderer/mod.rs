mod buffer;
mod builtin;
mod context;
mod flat_object;
mod gpu_batch;
pub mod light;
pub(crate) mod material;
mod mesh;
mod pass;
mod pipeline;
pub mod postprocess;
pub(crate) mod shadow;
pub(crate) mod texture;
mod uniform;

// Internal re-exports (pub(crate)) — used by engine subsystems (sprite, text, particle, etc.)
pub(crate) use buffer::{IndexBuffer, InstanceBuffer, VertexBuffer};
pub(crate) use builtin::{BuiltinPipeline, TransformUniform};
pub(crate) use context::RenderContext;
pub use context::{Context, HeadlessContext};
pub use flat_object::FlatObject;
pub(crate) use gpu_batch::GpuBatch;
pub use light::{
    AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight, POINT_LIGHT_WGSL,
    PointLight, PointLightArray, point_light_array_wgsl,
};
pub use material::{
    HasShadow, HasTexture, InstanceData, Material, MaterialBuilder, MaterialUniform, NoShadow,
    NoTexture,
};
pub use mesh::GpuMesh;
pub use pass::RenderPass;
pub(crate) use pipeline::{
    Pipeline, PipelineDescriptor, VertexAttribute, VertexFormat, VertexLayout,
};
pub use shadow::{SHADOW_WGSL, ShadowMap};
pub use texture::{FilterMode, RenderTarget, Texture};
pub(crate) use uniform::{StorageBuffer, UniformBuffer};

/// Generate indices for `count` axis-aligned quads (2 triangles each, 6 indices per quad).
///
/// Assumes vertices are laid out as sequential groups of 4: `[v0, v1, v2, v3, v4, ...]`
/// where each group forms one quad (two triangles: `0-1-2` and `0-2-3`).
pub fn quad_indices(count: u32) -> Vec<u32> {
    (0..count)
        .flat_map(|i| {
            let b = i * 4;
            [b, b + 1, b + 2, b, b + 2, b + 3]
        })
        .collect()
}
