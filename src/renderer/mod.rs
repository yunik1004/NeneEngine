mod buffer;
mod builtin;
mod context;
mod flat_object;
pub mod light;
pub(crate) mod material;
mod mesh;
mod pass;
mod pipeline;
pub mod postprocess;
pub mod shadow;
pub mod texture;
mod uniform;

pub use builtin::{Pos2, Pos2Uv, Pos3, Pos3Norm};

// Internal re-exports (pub(crate)) — used by engine subsystems (sprite, text, particle, etc.)
pub(crate) use buffer::{IndexBuffer, InstanceBuffer, VertexBuffer};
pub(crate) use builtin::{BuiltinPipeline, MvpUniform, TransformUniform};
pub(crate) use context::RenderContext;
pub use context::{Context, HeadlessContext};
pub use flat_object::FlatObject;
pub use light::{
    AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight, POINT_LIGHT_WGSL,
    PointLight, PointLightArray, point_light_array_wgsl,
};
pub use material::{InstanceData, Material, MaterialBuilder, MaterialUniform};
pub use mesh::{InstancedMesh, Mesh};
pub use pass::RenderPass;
pub(crate) use pipeline::{
    Pipeline, PipelineDescriptor, VertexAttribute, VertexFormat, VertexLayout,
};
pub use shadow::{SHADOW_WGSL, ShadowMap};
pub use texture::{FilterMode, RenderTarget, Texture};
pub(crate) use uniform::UniformBuffer;
