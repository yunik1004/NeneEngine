//! Pre-built pipelines and vertex types used internally by the renderer.

use super::{PipelineDescriptor, VertexAttribute, VertexFormat, VertexLayout};

// ── Uniform types ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, encase::ShaderType)]
pub(crate) struct TransformUniform {
    pub(crate) mvp: glam::Mat4,
    pub(crate) color: glam::Vec4,
}

impl TransformUniform {
    pub(crate) fn new(mvp: glam::Mat4, color: glam::Vec4) -> Self {
        Self { mvp, color }
    }
}

// ── WGSL shaders ──────────────────────────────────────────────────────────────

// ── WGSL generator ────────────────────────────────────────────────────────────

/// Generate WGSL for the 2-D flat-color pipeline (vec2 position).
fn gen_flat_wgsl() -> String {
    "\
struct FlatU { mvp: mat4x4<f32>, color: vec4<f32> }
@group(0) @binding(0) var<uniform> u: FlatU;
@vertex fn vs_main(@location(0) pos: vec2<f32>,
) -> @builtin(position) vec4<f32> { return u.mvp * vec4(pos, 0.0, 1.0); }
@fragment fn fs_main() -> @location(0) vec4<f32> { return u.color; }
"
    .to_string()
}

// ── BuiltinPipeline ───────────────────────────────────────────────────────────

/// Pre-built pipeline variants — pass to [`Context::create_builtin_pipeline`].
pub(crate) enum BuiltinPipeline {
    /// 2-D solid-color triangles with an MVP uniform transform.
    Transform2d,
}

impl BuiltinPipeline {
    pub(crate) fn descriptor(&self) -> PipelineDescriptor {
        match self {
            BuiltinPipeline::Transform2d => PipelineDescriptor::new(
                gen_flat_wgsl(),
                VertexLayout {
                    stride: 8,
                    attributes: vec![VertexAttribute {
                        offset: 0,
                        location: 0,
                        format: VertexFormat::Float32x2,
                    }],
                },
            )
            .with_uniform(),
        }
    }
}
