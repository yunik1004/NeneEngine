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

#[derive(Clone, Copy)]
enum VertexIn {
    Pos2,       // @location(0) pos: vec2<f32>
    MeshVertex, // @location(0) pos: vec3<f32>, @location(1) _n: vec3<f32>, @location(2) uv: vec2<f32>
}

/// Generate WGSL for flat-color and simple-texture pipelines.
///
/// - `has_mvp`:     include `mvp: mat4x4<f32>` in the uniform struct at group 0
/// - `has_color`:   include `color: vec4<f32>` in the uniform struct at group 0
/// - `has_texture`: sample `t_diffuse` / `s_diffuse`
/// - `tex_group`:   bind group index for the texture (0 for sprite, 1 for textured mesh)
fn gen_flat_wgsl(
    vin: VertexIn,
    has_mvp: bool,
    has_color: bool,
    has_texture: bool,
    tex_group: u32,
) -> String {
    let mut s = String::new();
    let is_3d = matches!(vin, VertexIn::MeshVertex);
    let has_uv = matches!(vin, VertexIn::MeshVertex);
    let pass_uv = has_uv && has_texture;

    // Uniform struct (group 0 when texture is at group 1 or not present)
    if has_mvp || has_color {
        s.push_str("struct FlatU {\n");
        if has_mvp {
            s.push_str("    mvp:   mat4x4<f32>,\n");
        }
        if has_color {
            s.push_str("    color: vec4<f32>,\n");
        }
        s.push_str("}\n@group(0) @binding(0) var<uniform> u: FlatU;\n");
    }

    // Texture bindings
    if has_texture {
        s.push_str(&format!(
            "@group({tex_group}) @binding(0) var t_diffuse: texture_2d<f32>;\n\
             @group({tex_group}) @binding(1) var s_diffuse: sampler;\n"
        ));
    }

    // VOut struct (only needed to carry UV to the fragment stage)
    if pass_uv {
        s.push_str(
            "struct VOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> }\n",
        );
    }

    // Vertex shader
    let pos_type = if is_3d { "vec3<f32>" } else { "vec2<f32>" };
    s.push_str(&format!("@vertex fn vs_main(@location(0) pos: {pos_type},"));
    if let VertexIn::MeshVertex = vin {
        s.push_str(" @location(1) _n: vec3<f32>, @location(2) uv: vec2<f32>,")
    }

    let clip = match (has_mvp, is_3d) {
        (true, true) => "u.mvp * vec4(pos, 1.0)",
        (true, false) => "u.mvp * vec4(pos, 0.0, 1.0)",
        (false, true) => "vec4(pos, 1.0)",
        (false, false) => "vec4(pos, 0.0, 1.0)",
    };

    if pass_uv {
        s.push_str(&format!(") -> VOut {{ return VOut({clip}, uv); }}\n"));
    } else {
        s.push_str(&format!(
            ") -> @builtin(position) vec4<f32> {{ return {clip}; }}\n"
        ));
    }

    // Fragment shader
    if pass_uv {
        s.push_str("@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {\n");
        s.push_str("    return textureSample(t_diffuse, s_diffuse, v.uv);\n");
    } else {
        s.push_str("@fragment fn fs_main() -> @location(0) vec4<f32> {\n");
        s.push_str("    return u.color;\n");
    }
    s.push_str("}\n");

    s
}

// ── BuiltinPipeline ───────────────────────────────────────────────────────────

/// Pre-built pipeline variants — pass to [`Context::create_builtin_pipeline`].
///
/// | Variant | Vertex | Uniform | Blend | Depth |
/// |---------|--------|---------|-------|-------|
/// | `Transform2d` | `Vec2` | [`TransformUniform`] | opaque | off |
/// | `Textured3d` | `MeshVertex` | [`MvpUniform`] + texture | alpha | on |
pub(crate) enum BuiltinPipeline {
    /// 2-D solid-color triangles with an MVP uniform transform.
    Transform2d,
    /// 3-D textured mesh — `MeshVertex` layout, [`MvpUniform`] at group 0,
    /// texture + sampler at group 1, depth on, alpha blend on.
    Textured3d,
}

impl BuiltinPipeline {
    pub(crate) fn descriptor(&self) -> PipelineDescriptor {
        match self {
            BuiltinPipeline::Transform2d => PipelineDescriptor::new(
                gen_flat_wgsl(VertexIn::Pos2, true, true, false, 0),
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
            BuiltinPipeline::Textured3d => PipelineDescriptor::new(
                gen_flat_wgsl(VertexIn::MeshVertex, true, false, true, 1),
                crate::mesh::MeshVertex::layout(),
            )
            .with_uniform()
            .with_texture()
            .with_depth()
            .with_alpha_blend(),
        }
    }
}
