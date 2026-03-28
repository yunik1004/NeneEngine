//! Pre-built pipelines so simple apps don't need to write WGSL.
//!
//! # Quick start
//!
//! ```no_run
//! use nene::app::{App, WindowId, run};
//! use nene::renderer::{
//!     BuiltinPipeline, ColorUniform, Context, Pipeline, Pos2, RenderPass, UniformBuffer,
//!     VertexBuffer,
//! };
//! use nene::window::Config;
//!
//! struct MyApp {
//!     pipeline: Option<Pipeline>,
//!     vb:       Option<VertexBuffer>,
//!     ub:       Option<UniformBuffer>,
//! }
//!
//! impl App for MyApp {
//!     fn new() -> Self { MyApp { pipeline: None, vb: None, ub: None } }
//!
//!     fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
//!         self.pipeline = Some(ctx.create_builtin_pipeline(BuiltinPipeline::Flat2d));
//!         self.vb = Some(ctx.create_vertex_buffer(&[
//!             Pos2::new(-0.5, -0.5),
//!             Pos2::new( 0.5, -0.5),
//!             Pos2::new( 0.0,  0.5),
//!         ]));
//!         self.ub = Some(ctx.create_uniform_buffer(&ColorUniform::rgba(1.0, 0.2, 0.4, 1.0)));
//!     }
//!
//!     fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
//!         let (Some(p), Some(vb), Some(ub)) =
//!             (&self.pipeline, &self.vb, &self.ub) else { return };
//!         pass.set_pipeline(p);
//!         pass.set_uniform(0, ub);
//!         pass.set_vertex_buffer(0, vb);
//!         pass.draw(0..3);
//!     }
//!
//!     fn windows() -> Vec<Config> { vec![Config::default()] }
//! }
//!
//! fn main() { run::<MyApp>(); }
//! ```

use bytemuck::{Pod, Zeroable};

use super::{PipelineDescriptor, VertexAttribute, VertexFormat, VertexLayout};

// ── Vertex types ──────────────────────────────────────────────────────────────

/// 2-D position vertex — NDC space, location 0.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Pos2 {
    pub pos: [f32; 2],
}

impl Pos2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { pos: [x, y] }
    }

    pub fn layout() -> VertexLayout {
        VertexLayout {
            stride: 8,
            attributes: vec![VertexAttribute {
                offset: 0,
                location: 0,
                format: VertexFormat::Float32x2,
            }],
        }
    }
}

/// 2-D position + UV vertex — locations 0, 1.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Pos2Uv {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

impl Pos2Uv {
    pub fn new(x: f32, y: f32, u: f32, v: f32) -> Self {
        Self {
            pos: [x, y],
            uv: [u, v],
        }
    }

    pub fn layout() -> VertexLayout {
        VertexLayout {
            stride: 16,
            attributes: vec![
                VertexAttribute {
                    offset: 0,
                    location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: 8,
                    location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// 3-D position vertex — location 0.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Pos3 {
    pub pos: [f32; 3],
}

impl Pos3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { pos: [x, y, z] }
    }

    pub fn layout() -> VertexLayout {
        VertexLayout {
            stride: 12,
            attributes: vec![VertexAttribute {
                offset: 0,
                location: 0,
                format: VertexFormat::Float32x3,
            }],
        }
    }
}

/// 3-D position + normal vertex — locations 0, 1.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Pos3Norm {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

impl Pos3Norm {
    pub fn new(x: f32, y: f32, z: f32, nx: f32, ny: f32, nz: f32) -> Self {
        Self {
            pos: [x, y, z],
            normal: [nx, ny, nz],
        }
    }

    pub fn layout() -> VertexLayout {
        VertexLayout {
            stride: 24,
            attributes: vec![
                VertexAttribute {
                    offset: 0,
                    location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: 12,
                    location: 1,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// ── Uniform types ─────────────────────────────────────────────────────────────

/// Single MVP matrix uniform.
///
/// Used with [`BuiltinPipeline::Textured3d`].
#[derive(Clone, Copy, encase::ShaderType)]
pub struct MvpUniform {
    pub mvp: glam::Mat4,
}

impl MvpUniform {
    pub fn new(mvp: glam::Mat4) -> Self {
        Self { mvp }
    }

    pub fn identity() -> Self {
        Self {
            mvp: glam::Mat4::IDENTITY,
        }
    }
}

/// Flat RGBA color uniform.
///
/// Used with [`BuiltinPipeline::Flat2d`].
#[derive(Clone, Copy, encase::ShaderType)]
pub struct ColorUniform {
    pub color: glam::Vec4,
}

impl ColorUniform {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: glam::Vec4::new(r, g, b, a),
        }
    }

    pub fn white() -> Self {
        Self::rgba(1.0, 1.0, 1.0, 1.0)
    }
}

/// MVP transform + tint color, for 2-D and 3-D pipelines.
///
/// Used with [`BuiltinPipeline::Transform2d`] and [`BuiltinPipeline::Flat3d`].
#[derive(Clone, Copy, encase::ShaderType)]
pub struct TransformUniform {
    pub mvp: glam::Mat4,
    pub color: glam::Vec4,
}

impl TransformUniform {
    pub fn new(mvp: glam::Mat4, color: glam::Vec4) -> Self {
        Self { mvp, color }
    }

    pub fn identity_white() -> Self {
        Self {
            mvp: glam::Mat4::IDENTITY,
            color: glam::Vec4::ONE,
        }
    }
}

// ── WGSL shaders ──────────────────────────────────────────────────────────────

// ── WGSL generator ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum VertexIn {
    Pos2,       // @location(0) pos: vec2<f32>
    Pos2Uv,     // @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>
    Pos3,       // @location(0) pos: vec3<f32>
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
    let is_3d = matches!(vin, VertexIn::Pos3 | VertexIn::MeshVertex);
    let has_uv = matches!(vin, VertexIn::Pos2Uv | VertexIn::MeshVertex);
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
    match vin {
        VertexIn::Pos2Uv => s.push_str(" @location(1) uv: vec2<f32>,"),
        VertexIn::MeshVertex => {
            s.push_str(" @location(1) _n: vec3<f32>, @location(2) uv: vec2<f32>,")
        }
        _ => {}
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
/// | `Flat2d` | [`Pos2`] | [`ColorUniform`] | opaque | off |
/// | `Flat2dAlpha` | [`Pos2`] | [`ColorUniform`] | alpha | off |
/// | `Transform2d` | [`Pos2`] | [`TransformUniform`] | opaque | off |
/// | `Sprite` | [`Pos2Uv`] | — (texture) | alpha | off |
/// | `Flat3d` | [`Pos3`] | [`TransformUniform`] | opaque | on |
/// | `Textured3d` | `MeshVertex` | [`MvpUniform`] + texture | alpha | on |
pub enum BuiltinPipeline {
    /// 2-D solid-color triangles in NDC space.
    Flat2d,
    /// Like `Flat2d` but with alpha blending (for transparent 2-D shapes).
    Flat2dAlpha,
    /// 2-D solid-color triangles with an MVP uniform transform.
    Transform2d,
    /// 2-D textured sprite in NDC space — bind a [`Texture`](super::Texture).
    Sprite,
    /// 3-D solid-color geometry with MVP transform and depth testing.
    Flat3d,
    /// 3-D textured mesh — `MeshVertex` layout, [`MvpUniform`] at group 0,
    /// texture + sampler at group 1, depth on, alpha blend on.
    Textured3d,
}

impl BuiltinPipeline {
    pub(crate) fn descriptor(&self) -> PipelineDescriptor {
        match self {
            BuiltinPipeline::Flat2d => PipelineDescriptor::new(
                gen_flat_wgsl(VertexIn::Pos2, false, true, false, 0),
                Pos2::layout(),
            )
            .with_uniform(),
            BuiltinPipeline::Flat2dAlpha => PipelineDescriptor::new(
                gen_flat_wgsl(VertexIn::Pos2, false, true, false, 0),
                Pos2::layout(),
            )
            .with_uniform()
            .with_alpha_blend(),
            BuiltinPipeline::Transform2d => PipelineDescriptor::new(
                gen_flat_wgsl(VertexIn::Pos2, true, true, false, 0),
                Pos2::layout(),
            )
            .with_uniform(),
            BuiltinPipeline::Sprite => PipelineDescriptor::new(
                gen_flat_wgsl(VertexIn::Pos2Uv, false, false, true, 0),
                Pos2Uv::layout(),
            )
            .with_texture()
            .with_alpha_blend(),
            BuiltinPipeline::Flat3d => PipelineDescriptor::new(
                gen_flat_wgsl(VertexIn::Pos3, true, true, false, 0),
                Pos3::layout(),
            )
            .with_uniform()
            .with_depth(),
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
