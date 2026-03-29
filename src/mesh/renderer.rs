use crate::renderer::material::{Features, gen_main_wgsl, gen_shadow_wgsl};
use crate::renderer::{
    AmbientLight, Context, DirectionalLight, FilterMode, IndexBuffer, MaterialUniform, Pipeline,
    PipelineDescriptor, RenderPass, ShadowMap, Texture, UniformBuffer, VertexBuffer,
};

use super::Model;
use super::vertex::Vertex;

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
/// # use nene::app::Config;
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
            PipelineDescriptor::new(gen_shadow_wgsl(false), Vertex::layout())
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
                    vertex_color: false,
                }),
                Vertex::layout(),
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

        for mesh in model.meshes.iter().filter(|m| !m.skinned) {
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
