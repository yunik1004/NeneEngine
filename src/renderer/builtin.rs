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

// ── Uniform types ─────────────────────────────────────────────────────────────

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

const FLAT2D_WGSL: &str = "
struct Color { color: vec4<f32> }
@group(0) @binding(0) var<uniform> u: Color;

@vertex fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4(pos, 0.0, 1.0);
}

@fragment fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
}
";

const TRANSFORM2D_WGSL: &str = "
struct Transform { mvp: mat4x4<f32>, color: vec4<f32> }
@group(0) @binding(0) var<uniform> u: Transform;

struct VOut { @builtin(position) clip: vec4<f32> }

@vertex fn vs_main(@location(0) pos: vec2<f32>) -> VOut {
    return VOut(u.mvp * vec4(pos, 0.0, 1.0));
}

@fragment fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
}
";

const SPRITE_WGSL: &str = "
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0)       uv:   vec2<f32>,
}

@vertex fn vs_main(@location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>) -> VOut {
    return VOut(vec4(pos, 0.0, 1.0), uv);
}

@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, v.uv);
}
";

const FLAT3D_WGSL: &str = "
struct Transform { mvp: mat4x4<f32>, color: vec4<f32> }
@group(0) @binding(0) var<uniform> u: Transform;

@vertex fn vs_main(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.mvp * vec4(pos, 1.0);
}

@fragment fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
}
";

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
}

impl BuiltinPipeline {
    pub(crate) fn descriptor(&self) -> PipelineDescriptor<'static> {
        match self {
            BuiltinPipeline::Flat2d => {
                PipelineDescriptor::new(FLAT2D_WGSL, Pos2::layout()).with_uniform()
            }
            BuiltinPipeline::Flat2dAlpha => PipelineDescriptor::new(FLAT2D_WGSL, Pos2::layout())
                .with_uniform()
                .with_alpha_blend(),
            BuiltinPipeline::Transform2d => {
                PipelineDescriptor::new(TRANSFORM2D_WGSL, Pos2::layout()).with_uniform()
            }
            BuiltinPipeline::Sprite => PipelineDescriptor::new(SPRITE_WGSL, Pos2Uv::layout())
                .with_texture()
                .with_alpha_blend(),
            BuiltinPipeline::Flat3d => PipelineDescriptor::new(FLAT3D_WGSL, Pos3::layout())
                .with_uniform()
                .with_depth(),
        }
    }
}
