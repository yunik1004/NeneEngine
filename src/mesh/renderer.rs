use std::marker::PhantomData;

use bytemuck::{Pod, Zeroable};

use crate::renderer::material::{Features, gen_main_wgsl, gen_shadow_wgsl};
use crate::renderer::{
    AmbientLight, BuiltinPipeline, Context, DirectionalLight, FilterMode, IndexBuffer,
    MaterialUniform, MvpUniform, Pipeline, PipelineDescriptor, RenderPass, ShadowMap, Texture,
    UniformBuffer, VertexAttribute, VertexBuffer, VertexFormat, VertexLayout,
};

use super::{ColorVertex, MeshVertex, Model};

// ── GpuVertex trait ───────────────────────────────────────────────────────────

pub trait GpuVertex: Pod + Zeroable + 'static {
    fn create_pipeline(ctx: &mut Context) -> Pipeline;
    const USES_TEXTURE: bool;
}

// ── Impls ─────────────────────────────────────────────────────────────────────

const COLOR_WGSL: &str = "
struct Transform { mvp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> u: Transform;

struct VIn  { @location(0) pos: vec3<f32>, @location(1) color: vec4<f32> }
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) color: vec4<f32> }

@vertex fn vs_main(v: VIn) -> VOut {
    return VOut(u.mvp * vec4(v.pos, 1.0), v.color);
}

@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    return v.color;
}
";

impl GpuVertex for ColorVertex {
    fn create_pipeline(ctx: &mut Context) -> Pipeline {
        ctx.create_pipeline(
            PipelineDescriptor::new(
                COLOR_WGSL,
                VertexLayout {
                    stride: std::mem::size_of::<ColorVertex>() as u64,
                    attributes: vec![
                        VertexAttribute {
                            offset: 0,
                            location: 0,
                            format: VertexFormat::Float32x3,
                        },
                        VertexAttribute {
                            offset: 12,
                            location: 1,
                            format: VertexFormat::Float32x4,
                        },
                    ],
                },
            )
            .with_uniform()
            .with_alpha_blend(),
        )
    }
    const USES_TEXTURE: bool = false;
}

impl GpuVertex for MeshVertex {
    fn create_pipeline(ctx: &mut Context) -> Pipeline {
        ctx.create_builtin_pipeline(BuiltinPipeline::Textured3d)
    }
    const USES_TEXTURE: bool = true;
}

// ── Renderer<V> ───────────────────────────────────────────────────────────────

pub struct Renderer<V: GpuVertex> {
    pipeline: Pipeline,
    vbuf: Option<VertexBuffer>,
    ibuf: Option<IndexBuffer>,
    vert_count: u32,
    ubuf: UniformBuffer,
    texture: Option<Texture>,
    _phantom: PhantomData<V>,
}

impl<V: GpuVertex> Renderer<V> {
    pub fn new(ctx: &mut Context) -> Self {
        Self {
            pipeline: V::create_pipeline(ctx),
            vbuf: None,
            ibuf: None,
            vert_count: 0,
            ubuf: ctx.create_uniform_buffer(&MvpUniform::identity()),
            texture: None,
            _phantom: PhantomData,
        }
    }

    pub fn set_geometry(&mut self, ctx: &mut Context, verts: &[V]) {
        if verts.is_empty() {
            self.vert_count = 0;
            return;
        }
        self.vbuf = Some(ctx.create_vertex_buffer(verts));
        self.vert_count = verts.len() as u32;
    }

    pub fn set_indices(&mut self, ctx: &mut Context, indices: &[u32]) {
        self.ibuf = if indices.is_empty() {
            None
        } else {
            Some(ctx.create_index_buffer(indices))
        };
    }

    pub fn set_texture(&mut self, texture: Texture) {
        self.texture = Some(texture);
    }

    pub fn set_transform(&mut self, ctx: &mut Context, mvp: glam::Mat4) {
        ctx.update_uniform_buffer(&self.ubuf, &MvpUniform::new(mvp));
    }

    pub fn render(&self, pass: &mut RenderPass) {
        let Some(vbuf) = &self.vbuf else { return };
        if V::USES_TEXTURE && self.texture.is_none() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        if let Some(tex) = &self.texture {
            pass.set_texture(1, tex);
        }
        pass.set_vertex_buffer(0, vbuf);
        if let Some(ibuf) = &self.ibuf {
            pass.draw_indexed(ibuf);
        } else {
            pass.draw(0..self.vert_count);
        }
    }
}

// ── LitShadowedModel ──────────────────────────────────────────────────────────

/// GPU representation of a [`Model`] rendered with ambient + directional
/// lighting and PCF shadow mapping.
///
/// # Usage
///
/// ```no_run
/// # use nene::app::{App, WindowId, run};
/// # use nene::math::{Mat4, Vec3};
/// # use nene::mesh::{LitShadowedModel, Model};
/// # use nene::renderer::{AmbientLight, Context, DirectionalLight, RenderPass, ShadowMap};
/// # use nene::window::Config;
/// struct Demo {
///     model:       Model,
///     ambient:     AmbientLight,
///     directional: DirectionalLight,
///     renderer:    Option<LitShadowedModel>,
///     shadow_map:  Option<ShadowMap>,
/// }
/// impl App for Demo {
///     fn new() -> Self { todo!() }
///     fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
///         self.renderer   = Some(LitShadowedModel::new(ctx, &self.model));
///         self.shadow_map = Some(ctx.create_shadow_map(1024));
///     }
///     fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &nene::input::Input) {
///         let (Some(renderer), Some(shadow_map)) = (&self.renderer, &self.shadow_map) else { return };
///         renderer.prepare(ctx, Mat4::IDENTITY, Mat4::IDENTITY, Mat4::IDENTITY,
///                          self.ambient, self.directional);
///         ctx.shadow_pass(shadow_map, |pass| renderer.shadow_draw(pass));
///     }
///     fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
///         let (Some(renderer), Some(shadow_map)) = (&self.renderer, &self.shadow_map) else { return };
///         renderer.render(pass, shadow_map);
///     }
///     fn windows() -> Vec<Config> { vec![Config::default()] }
/// }
/// ```
pub struct LitShadowedModel {
    shadow_pipeline: Pipeline,
    main_pipeline: Pipeline,
    vbufs: Vec<VertexBuffer>,
    ibufs: Vec<IndexBuffer>,
    textures: Vec<Texture>,
    mesh_transforms: Vec<glam::Mat4>,
    uniforms: Vec<UniformBuffer>,
}

impl LitShadowedModel {
    /// Upload all mesh data from `model` to the GPU.
    pub fn new(ctx: &mut Context, model: &Model) -> Self {
        let shadow_pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(gen_shadow_wgsl(false), MeshVertex::layout())
                .with_uniform()
                .depth_only(),
        );
        let main_pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(
                gen_main_wgsl(Features {
                    texture: true,
                    ambient: true,
                    directional: true,
                    shadow: true,
                    casts_shadow: true,
                    instanced: false,
                }),
                MeshVertex::layout(),
            )
            .with_uniform()
            .with_texture()
            .with_shadow_map()
            .with_depth()
            .with_alpha_blend(),
        );

        let blank = MaterialUniform::default();

        let mut vbufs = Vec::new();
        let mut ibufs = Vec::new();
        let mut textures = Vec::new();
        let mut mesh_transforms = Vec::new();
        let mut uniforms = Vec::new();

        for mesh in &model.meshes {
            vbufs.push(ctx.create_vertex_buffer(&mesh.vertices));
            ibufs.push(ctx.create_index_buffer(&mesh.indices));
            textures.push(match &mesh.base_color {
                Some(img) => {
                    ctx.create_texture_with(img.width, img.height, &img.data, FilterMode::Linear)
                }
                None => ctx.create_texture_with(1, 1, &[255, 255, 255, 255], FilterMode::Nearest),
            });
            mesh_transforms.push(mesh.transform);
            uniforms.push(ctx.create_uniform_buffer(&blank));
        }

        Self {
            shadow_pipeline,
            main_pipeline,
            vbufs,
            ibufs,
            textures,
            mesh_transforms,
            uniforms,
        }
    }

    /// Upload per-frame uniforms for all meshes.
    ///
    /// `transform` is a global transform applied on top of each mesh's own
    /// node transform (e.g. a rotation or world-space placement).
    pub fn prepare(
        &self,
        ctx: &mut Context,
        view_proj: glam::Mat4,
        transform: glam::Mat4,
        light_vp: glam::Mat4,
        ambient: AmbientLight,
        directional: DirectionalLight,
    ) {
        for i in 0..self.mesh_transforms.len() {
            let model = transform * self.mesh_transforms[i];
            ctx.update_uniform_buffer(
                &self.uniforms[i],
                &MaterialUniform {
                    view_proj,
                    model,
                    light_vp,
                    ambient,
                    directional,
                    color: glam::Vec4::ONE,
                },
            );
        }
    }

    /// Submit draw calls for the depth-only shadow pass.
    ///
    /// Call this inside [`Context::shadow_pass`].
    pub fn shadow_draw(&self, pass: &mut RenderPass) {
        pass.set_pipeline(&self.shadow_pipeline);
        for i in 0..self.vbufs.len() {
            pass.set_uniform(0, &self.uniforms[i]);
            pass.set_vertex_buffer(0, &self.vbufs[i]);
            pass.draw_indexed(&self.ibufs[i]);
        }
    }

    /// Submit draw calls for the main lit + shadowed render pass.
    pub fn render(&self, pass: &mut RenderPass, shadow_map: &ShadowMap) {
        pass.set_pipeline(&self.main_pipeline);
        for i in 0..self.vbufs.len() {
            pass.set_uniform(0, &self.uniforms[i]);
            pass.set_texture(1, &self.textures[i]);
            pass.set_shadow_map(2, shadow_map);
            pass.set_vertex_buffer(0, &self.vbufs[i]);
            pass.draw_indexed(&self.ibufs[i]);
        }
    }
}
