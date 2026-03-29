//! Composable material system — assemble a 3-D shader from feature flags
//! without writing WGSL by hand.
//!
//! [`Material`] is parameterised over two marker types that encode which
//! optional bind-groups are needed at render time:
//!
//! | Type parameter | Meaning |
//! |---|---|
//! | `T = NoTexture` / `HasTexture` | whether a diffuse texture is bound |
//! | `S = NoShadow`  / `HasShadow`  | whether a shadow map is bound |
//!
//! The combination is chosen at build time via the builder, and the compiler
//! enforces the correct [`render`](Material::render) signature for each variant.
//!
//! # Example
//!
//! ```no_run
//! # use nene::app::{App, WindowId, run};
//! # use nene::math::{Mat4, Vec4};
//! # use nene::renderer::{AmbientLight, Context, DirectionalLight, GpuMesh, HasShadow, Material,
//! #     MaterialBuilder, NoTexture, RenderPass, ShadowMap};
//! # use nene::app::Config;
//! struct Demo {
//!     mat:        Option<Material<NoTexture, HasShadow>>,
//!     mesh:       Option<GpuMesh>,
//!     shadow_map: Option<ShadowMap>,
//!     ambient:    AmbientLight,
//!     directional: DirectionalLight,
//! }
//! impl App for Demo {
//!     fn new() -> Self { todo!() }
//!
//!     fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
//!         self.mat = Some(
//!             MaterialBuilder::new()
//!                 .ambient()
//!                 .directional()
//!                 .shadow()
//!                 .build(ctx),
//!         );
//!         self.shadow_map = Some(ctx.create_shadow_map(1024));
//!     }
//!
//!     fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &nene::input::Input) {
//!         let Some(mat) = &mut self.mat else { return };
//!         mat.uniform.view_proj   = Mat4::IDENTITY;
//!         mat.uniform.model       = Mat4::IDENTITY;
//!         mat.uniform.ambient     = self.ambient;
//!         mat.uniform.directional = self.directional;
//!         mat.flush(ctx);
//!         let (Some(mesh), Some(sm)) = (&self.mesh, &self.shadow_map) else { return };
//!         ctx.shadow_pass(sm, |pass| mat.shadow_draw(pass, mesh));
//!     }
//!
//!     fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
//!         let (Some(mat), Some(mesh), Some(sm)) =
//!             (&self.mat, &self.mesh, &self.shadow_map) else { return };
//!         mat.render(pass, mesh, sm);
//!     }
//!
//!     fn windows() -> Vec<Config> { vec![Config::default()] }
//! }
//! ```

use std::marker::PhantomData;

use super::{
    Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexAttribute,
    VertexFormat, VertexLayout,
    context::Context,
    light::{AMBIENT_LIGHT_WGSL, AmbientLight, DIRECTIONAL_LIGHT_WGSL, DirectionalLight},
    mesh::{GpuMesh, InstancedMesh},
    shadow::{SHADOW_WGSL, ShadowMap},
    texture::Texture,
};

// ── Texture / shadow marker types ─────────────────────────────────────────────

/// Marker: material does **not** sample a diffuse texture.
pub struct NoTexture;
/// Marker: material samples a diffuse texture at bind group 1.
pub struct HasTexture;
/// Marker: material does **not** read a shadow map.
pub struct NoShadow;
/// Marker: material reads a PCF shadow map at bind group 2.
pub struct HasShadow;

// ── MaterialUniform ───────────────────────────────────────────────────────────

/// Fat uniform shared by all [`Material`] variants.
///
/// All fields are present regardless of which features are active; the
/// generated WGSL shader only reads what it needs.  Set the relevant fields
/// and call [`Material::flush`] once per frame.
#[derive(Clone, Copy, encase::ShaderType)]
pub struct MaterialUniform {
    pub view_proj: glam::Mat4,
    pub model: glam::Mat4,
    /// Light-space view-projection used by shadow passes.
    pub light_vp: glam::Mat4,
    /// Tint / flat color (used when no texture is bound).
    pub color: glam::Vec4,
    pub ambient: AmbientLight,
    pub directional: DirectionalLight,
}

impl Default for MaterialUniform {
    fn default() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY,
            model: glam::Mat4::IDENTITY,
            light_vp: glam::Mat4::IDENTITY,
            color: glam::Vec4::ONE,
            ambient: AmbientLight::default(),
            directional: DirectionalLight::default(),
        }
    }
}

// ── InstanceData ──────────────────────────────────────────────────────────────

/// Per-instance data for [`MaterialBuilder::instanced`] rendering.
///
/// Upload a `Vec<InstanceData>` via [`Context::create_instance_buffer`] /
/// [`Context::update_instance_buffer`], then call
/// [`Material::render_instanced`] each frame.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    /// Per-instance model transform (column-major).
    pub model: [[f32; 4]; 4],
    /// Per-instance tint color (RGBA).
    pub color: [f32; 4],
}

impl InstanceData {
    pub fn new(model: glam::Mat4, color: glam::Vec4) -> Self {
        Self {
            model: model.to_cols_array_2d(),
            color: color.into(),
        }
    }

    /// Vertex layout for slot 1 (instance buffer). Attributes start at
    /// location 3 to follow the per-vertex `pos`/`normal`/`uv` at 0–2.
    pub(crate) fn layout() -> VertexLayout {
        // Instance attributes start at location 6, after Vertex's 0-5.
        VertexLayout {
            stride: std::mem::size_of::<InstanceData>() as u64,
            attributes: vec![
                VertexAttribute {
                    offset: 0,
                    location: 6,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: 16,
                    location: 7,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: 32,
                    location: 8,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: 48,
                    location: 9,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: 64,
                    location: 10,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// ── Feature flags ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
pub(crate) struct Features {
    pub(crate) texture: bool,
    pub(crate) ambient: bool,
    pub(crate) directional: bool,
    pub(crate) shadow: bool,
    pub(crate) casts_shadow: bool,
    pub(crate) instanced: bool,
}

// ── MaterialBuilder ───────────────────────────────────────────────────────────

/// Builder for a composable [`Material`].
///
/// Start with [`MaterialBuilder::new`], chain feature methods, and finish
/// with [`build`](MaterialBuilder::build).
///
/// Calling [`.texture()`](Self::texture) or [`.shadow()`](Self::shadow)
/// changes the builder's type, so the compiled [`Material`] has the correct
/// [`render`](Material::render) signature.
pub struct MaterialBuilder<T = NoTexture, S = NoShadow> {
    feat: Features,
    init: MaterialUniform,
    custom_wgsl: Option<String>,
    _phantom: PhantomData<(T, S)>,
}

impl Default for MaterialBuilder {
    fn default() -> Self {
        Self {
            feat: Features::default(),
            init: MaterialUniform::default(),
            custom_wgsl: None,
            _phantom: PhantomData,
        }
    }
}

impl MaterialBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, S> MaterialBuilder<T, S> {
    /// Initial tint color written into the uniform.
    pub fn color(mut self, c: glam::Vec4) -> Self {
        self.init.color = c;
        self
    }

    /// Apply ambient lighting from [`MaterialUniform::ambient`].
    pub fn ambient(mut self) -> Self {
        self.feat.ambient = true;
        self
    }

    /// Apply directional lighting from [`MaterialUniform::directional`].
    pub fn directional(mut self) -> Self {
        self.feat.directional = true;
        self
    }

    /// Generate a depth-only pipeline for [`Material::shadow_draw`].
    pub fn casts_shadow(mut self) -> Self {
        self.feat.casts_shadow = true;
        self
    }

    /// Enable GPU instancing via [`InstanceData`] at vertex slot 1.
    ///
    /// Use [`Material::render_instanced`] to draw.
    pub fn instanced(mut self) -> Self {
        self.feat.instanced = true;
        self
    }

    /// Override the auto-generated WGSL with a custom shader.
    pub fn shader(mut self, wgsl: impl Into<String>) -> Self {
        self.custom_wgsl = Some(wgsl.into());
        self
    }

    /// Sample a diffuse texture at group 1.
    ///
    /// Transitions to `MaterialBuilder<HasTexture, S>` — the resulting
    /// [`Material::render`] will require a `&Texture` argument.
    pub fn texture(self) -> MaterialBuilder<HasTexture, S> {
        MaterialBuilder {
            feat: Features {
                texture: true,
                ..self.feat
            },
            init: self.init,
            custom_wgsl: self.custom_wgsl,
            _phantom: PhantomData,
        }
    }

    /// Read a PCF shadow map at group 2. Implies [`casts_shadow`](Self::casts_shadow).
    ///
    /// Transitions to `MaterialBuilder<T, HasShadow>` — the resulting
    /// [`Material::render`] will require a `&ShadowMap` argument.
    pub fn shadow(self) -> MaterialBuilder<T, HasShadow> {
        MaterialBuilder {
            feat: Features {
                shadow: true,
                casts_shadow: true,
                ..self.feat
            },
            init: self.init,
            custom_wgsl: self.custom_wgsl,
            _phantom: PhantomData,
        }
    }

    /// Consume the builder and create a [`Material`] on the GPU.
    pub fn build(self, ctx: &mut Context) -> Material<T, S> {
        let main_wgsl = self.custom_wgsl.unwrap_or_else(|| gen_main_wgsl(self.feat));
        let mut desc = PipelineDescriptor::new(main_wgsl, crate::mesh::Vertex::layout())
            .with_uniform()
            .with_depth();
        if self.feat.texture {
            desc = desc.with_texture().with_alpha_blend();
        }
        if self.feat.shadow {
            desc = desc.with_shadow_map();
        }
        if self.feat.instanced {
            desc = desc.with_instance_layout(InstanceData::layout());
        }
        let pipeline = ctx.create_pipeline(desc);

        let shadow_pipeline = if self.feat.casts_shadow {
            let sdesc = PipelineDescriptor::new(
                gen_shadow_wgsl(self.feat.instanced),
                crate::mesh::Vertex::layout(),
            )
            .with_uniform()
            .depth_only();
            let sdesc = if self.feat.instanced {
                sdesc.with_instance_layout(InstanceData::layout())
            } else {
                sdesc
            };
            Some(ctx.create_pipeline(sdesc))
        } else {
            None
        };

        let ubuf = ctx.create_uniform_buffer(&self.init);
        Material {
            pipeline,
            shadow_pipeline,
            ubuf,
            uniform: self.init,
            _phantom: PhantomData,
        }
    }
}

// ── Material ──────────────────────────────────────────────────────────────────

/// A GPU material assembled from feature flags.
///
/// The type parameters `T` and `S` encode which resources must be provided at
/// render time:
///
/// | Type | `render` signature |
/// |---|---|
/// | `Material` (`NoTexture, NoShadow`) | `render(pass, mesh)` |
/// | `Material<HasTexture>` | `render(pass, mesh, &texture)` |
/// | `Material<NoTexture, HasShadow>` | `render(pass, mesh, &shadow_map)` |
/// | `Material<HasTexture, HasShadow>` | `render(pass, mesh, &texture, &shadow_map)` |
///
/// Mutate [`Material::uniform`] each frame, call [`flush`](Material::flush)
/// once to upload, then call a render method.
pub struct Material<T = NoTexture, S = NoShadow> {
    pipeline: Pipeline,
    shadow_pipeline: Option<Pipeline>,
    ubuf: UniformBuffer,
    /// CPU-side copy of the uniform. Mutate fields freely; call
    /// [`flush`](Material::flush) to upload changes to the GPU.
    pub uniform: MaterialUniform,
    _phantom: PhantomData<(T, S)>,
}

// ── Shared methods (all variants) ─────────────────────────────────────────────

impl<T, S> Material<T, S> {
    /// Upload [`uniform`](Material::uniform) to the GPU. Call once per frame
    /// after mutating any fields.
    pub fn flush(&self, ctx: &mut Context) {
        ctx.update_uniform_buffer(&self.ubuf, &self.uniform);
    }

    /// Depth-only draw for the shadow pass. Call inside [`Context::shadow_pass`].
    /// No-op if the material was not built with `.casts_shadow()` or `.shadow()`.
    pub fn shadow_draw(&self, pass: &mut RenderPass, mesh: &GpuMesh) {
        let Some(sp) = &self.shadow_pipeline else {
            return;
        };
        pass.set_pipeline(sp);
        pass.set_uniform(0, &self.ubuf);
        pass.set_vertex_buffer(0, &mesh.vbuf);
        pass.draw_indexed(&mesh.ibuf);
    }

    /// Instanced draw. Only valid for materials built with
    /// [`.instanced()`](MaterialBuilder::instanced).
    pub fn render_instanced(&self, pass: &mut RenderPass, mesh: &InstancedMesh) {
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_vertex_buffer(0, &mesh.vbuf);
        pass.set_instance_buffer(1, &mesh.inst_buf);
        pass.draw_indexed_instanced(&mesh.ibuf, mesh.count());
    }

    fn draw_inner(
        &self,
        pass: &mut RenderPass,
        mesh: &GpuMesh,
        texture: Option<&Texture>,
        shadow_map: Option<&ShadowMap>,
    ) {
        let (vbuf, ibuf) = (&mesh.vbuf, &mesh.ibuf);
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        if let Some(t) = texture {
            pass.set_texture(1, t);
        }
        if let Some(sm) = shadow_map {
            pass.set_shadow_map(2, sm);
        }
        pass.set_vertex_buffer(0, vbuf);
        pass.draw_indexed(ibuf);
    }
}

// ── render() — one impl per feature combination ───────────────────────────────

impl Material<NoTexture, NoShadow> {
    /// Draw the mesh.
    pub fn render(&self, pass: &mut RenderPass, mesh: &GpuMesh) {
        self.draw_inner(pass, mesh, None, None);
    }
}

impl Material<HasTexture, NoShadow> {
    /// Draw the mesh with a diffuse texture.
    pub fn render(&self, pass: &mut RenderPass, mesh: &GpuMesh, texture: &Texture) {
        self.draw_inner(pass, mesh, Some(texture), None);
    }
}

impl Material<NoTexture, HasShadow> {
    /// Draw the mesh, reading from a shadow map.
    pub fn render(&self, pass: &mut RenderPass, mesh: &GpuMesh, shadow_map: &ShadowMap) {
        self.draw_inner(pass, mesh, None, Some(shadow_map));
    }
}

impl Material<HasTexture, HasShadow> {
    /// Draw the mesh with a diffuse texture and shadow map.
    pub fn render(
        &self,
        pass: &mut RenderPass,
        mesh: &GpuMesh,
        texture: &Texture,
        shadow_map: &ShadowMap,
    ) {
        self.draw_inner(pass, mesh, Some(texture), Some(shadow_map));
    }
}

// ── WGSL generation ───────────────────────────────────────────────────────────

/// The MaterialU WGSL struct declaration — always identical regardless of features.
/// Depends on AmbientLight and DirectionalLight types being declared first.
const MATERIAL_U_WGSL: &str = "
struct MaterialU {
    view_proj:   mat4x4<f32>,
    model:       mat4x4<f32>,
    light_vp:    mat4x4<f32>,
    color:       vec4<f32>,
    ambient:     AmbientLight,
    directional: DirectionalLight,
}
@group(0) @binding(0) var<uniform> u: MaterialU;
";

pub(crate) fn gen_main_wgsl(feat: Features) -> String {
    let needs_normal = feat.ambient || feat.directional || feat.shadow;

    // Assign VOut locations in order
    let mut loc = 0u32;
    let mut next = || {
        let l = loc;
        loc += 1;
        l
    };
    let normal_loc = needs_normal.then(&mut next);
    let uv_loc = feat.texture.then(&mut next);
    let lspace_loc = feat.shadow.then(&mut next);
    let color_loc = feat.instanced.then(next);

    let mut s = String::new();

    s.push_str(AMBIENT_LIGHT_WGSL);
    s.push_str(DIRECTIONAL_LIGHT_WGSL);
    if feat.shadow {
        s.push_str(SHADOW_WGSL);
    }
    s.push_str(MATERIAL_U_WGSL);

    if feat.texture {
        s.push_str(
            "@group(1) @binding(0) var t_diffuse: texture_2d<f32>;\n\
             @group(1) @binding(1) var s_diffuse: sampler;\n",
        );
    }
    if feat.shadow {
        s.push_str(
            "@group(2) @binding(0) var shadow_tex:  texture_depth_2d;\n\
             @group(2) @binding(1) var shadow_samp: sampler_comparison;\n",
        );
    }

    // VOut struct
    s.push_str("struct VOut {\n    @builtin(position) clip: vec4<f32>,\n");
    if let Some(l) = normal_loc {
        s.push_str(&format!("    @location({l}) normal: vec3<f32>,\n"));
    }
    if let Some(l) = uv_loc {
        s.push_str(&format!("    @location({l}) uv: vec2<f32>,\n"));
    }
    if let Some(l) = lspace_loc {
        s.push_str(&format!("    @location({l}) light_space: vec4<f32>,\n"));
    }
    if let Some(l) = color_loc {
        s.push_str(&format!("    @location({l}) color: vec4<f32>,\n"));
    }
    s.push_str("}\n");

    // Vertex shader inputs
    s.push_str(
        "@vertex\nfn vs_main(\n\
         \t@location(0) pos: vec3<f32>,\n\
         \t@location(1) nor: vec3<f32>,\n\
         \t@location(2) uv:  vec2<f32>,\n",
    );
    if feat.instanced {
        // Instance attributes start at location 6 (after Vertex's 0–5).
        s.push_str(
            "\t@location(6)  i_col0:  vec4<f32>,\n\
             \t@location(7)  i_col1:  vec4<f32>,\n\
             \t@location(8)  i_col2:  vec4<f32>,\n\
             \t@location(9)  i_col3:  vec4<f32>,\n\
             \t@location(10) i_color: vec4<f32>,\n",
        );
    }
    s.push_str(") -> VOut {\n\tvar o: VOut;\n");

    // Model matrix and world position
    if feat.instanced {
        s.push_str("\tlet model = mat4x4<f32>(i_col0, i_col1, i_col2, i_col3);\n");
    }
    let model = if feat.instanced { "model" } else { "u.model" };
    s.push_str(&format!("\tlet world = {model} * vec4(pos, 1.0);\n"));
    s.push_str("\to.clip = u.view_proj * world;\n");

    if normal_loc.is_some() {
        s.push_str(&format!(
            "\to.normal = normalize(({model} * vec4(nor, 0.0)).xyz);\n"
        ));
    }
    if uv_loc.is_some() {
        s.push_str("\to.uv = uv;\n");
    }
    if lspace_loc.is_some() {
        s.push_str("\to.light_space = u.light_vp * world;\n");
    }
    if color_loc.is_some() {
        s.push_str("\to.color = i_color;\n");
    }
    s.push_str("\treturn o;\n}\n");

    // Fragment shader
    s.push_str("@fragment\nfn fs_main(v: VOut) -> @location(0) vec4<f32> {\n");

    if feat.texture {
        s.push_str(
            "\tlet albedo = textureSample(t_diffuse, s_diffuse, v.uv);\n\
             \tvar rgb = albedo.rgb;\n\
             \tlet alpha = albedo.a;\n",
        );
    } else if feat.instanced {
        s.push_str("\tvar rgb = v.color.rgb;\n\tlet alpha = v.color.a;\n");
    } else {
        s.push_str("\tvar rgb = u.color.rgb;\n\tlet alpha = u.color.a;\n");
    }

    if feat.ambient || feat.directional {
        s.push_str("\tvar light = vec3<f32>(0.0);\n");
        if feat.ambient {
            s.push_str("\tlight += ambient_light(u.ambient);\n");
        }
        if feat.directional {
            if feat.shadow {
                s.push_str(
                    "\tlet shad = shadow_factor(shadow_tex, shadow_samp, v.light_space, 0.0);\n\
                     \tlight += directional_light(u.directional, v.normal) * shad;\n",
                );
            } else {
                s.push_str("\tlight += directional_light(u.directional, v.normal);\n");
            }
        }
        s.push_str("\trgb = rgb * light;\n");
    }

    s.push_str("\treturn vec4(rgb, alpha);\n}\n");
    s
}

pub(crate) fn gen_shadow_wgsl(instanced: bool) -> String {
    let mut s = String::new();
    s.push_str(AMBIENT_LIGHT_WGSL);
    s.push_str(DIRECTIONAL_LIGHT_WGSL);
    s.push_str(MATERIAL_U_WGSL);

    s.push_str(
        "@vertex\nfn vs_main(\n\
         \t@location(0) pos: vec3<f32>,\n\
         \t@location(1) _n:  vec3<f32>,\n\
         \t@location(2) _uv: vec2<f32>,\n",
    );
    if instanced {
        s.push_str(
            "\t@location(6)  i_col0:   vec4<f32>,\n\
             \t@location(7)  i_col1:   vec4<f32>,\n\
             \t@location(8)  i_col2:   vec4<f32>,\n\
             \t@location(9)  i_col3:   vec4<f32>,\n\
             \t@location(10) _i_color: vec4<f32>,\n",
        );
    }
    s.push_str(") -> @builtin(position) vec4<f32> {\n");
    if instanced {
        s.push_str(
            "\tlet model = mat4x4<f32>(i_col0, i_col1, i_col2, i_col3);\n\
             \treturn u.light_vp * model * vec4(pos, 1.0);\n",
        );
    } else {
        s.push_str("\treturn u.light_vp * u.model * vec4(pos, 1.0);\n");
    }
    s.push_str("}\n");
    s
}
