//! [`SkinnedMaterial`] — GPU material for skeletal meshes.

use crate::math::Mat4;
use crate::mesh::Vertex;
use crate::renderer::{
    AmbientLight, Context, DirectionalLight, GpuMesh, Pipeline, PipelineDescriptor, RenderPass,
    StorageBuffer, Texture, UniformBuffer,
    light::{AMBIENT_LIGHT_WGSL, DIRECTIONAL_LIGHT_WGSL},
};

use super::animator::skinning_wgsl;

// ── Uniform ───────────────────────────────────────────────────────────────────

/// Fat uniform for [`SkinnedMaterial`]. Set the relevant fields and call
/// [`SkinnedMaterial::flush`] once per frame.
#[derive(Clone, Copy, encase::ShaderType)]
pub struct SkinnedMaterialUniform {
    pub view_proj: glam::Mat4,
    pub model: glam::Mat4,
    /// Base tint color. Multiplied with the texture sample when `.texture()` is active.
    pub color: glam::Vec4,
    /// Rim light tint color.
    pub rim_color: glam::Vec4,
    /// World-space camera position. Required for rim lighting.
    pub camera_pos: glam::Vec4,
    pub ambient: AmbientLight,
    pub directional: DirectionalLight,
}

impl Default for SkinnedMaterialUniform {
    fn default() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY,
            model: glam::Mat4::IDENTITY,
            color: glam::Vec4::ONE,
            rim_color: glam::Vec4::ONE,
            camera_pos: glam::Vec4::ZERO,
            ambient: AmbientLight::default(),
            directional: DirectionalLight::default(),
        }
    }
}

// ── Features ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
struct Features {
    texture: bool,
    ambient: bool,
    directional: bool,
    rim: bool,
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Builder for a [`SkinnedMaterial`].
///
/// Pass the joint count from your loaded skeleton so the GPU buffer is sized
/// correctly at build time:
/// ```ignore
/// let mat = SkinnedMaterialBuilder::new(skeleton.joints.len()).ambient().build(ctx);
/// ```
#[derive(Default)]
pub struct SkinnedMaterialBuilder {
    feat: Features,
    init: SkinnedMaterialUniform,
}

impl SkinnedMaterialBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply ambient lighting from [`SkinnedMaterialUniform::ambient`].
    pub fn ambient(mut self) -> Self {
        self.feat.ambient = true;
        self
    }

    /// Apply directional lighting from [`SkinnedMaterialUniform::directional`].
    pub fn directional(mut self) -> Self {
        self.feat.directional = true;
        self
    }

    /// Add rim lighting. Set [`SkinnedMaterialUniform::camera_pos`] and
    /// [`SkinnedMaterialUniform::rim_color`] each frame.
    pub fn rim(mut self) -> Self {
        self.feat.rim = true;
        self
    }

    /// Sample a diffuse texture at group 2. Pass `Some(&texture)` to [`SkinnedMaterial::render`].
    pub fn texture(mut self) -> Self {
        self.feat.texture = true;
        self
    }

    /// Consume the builder and create a [`SkinnedMaterial`] on the GPU.
    ///
    /// `joint_count` must match the number of joints in the skeleton that will
    /// be animated with this material.
    pub fn build(self, ctx: &mut Context, joint_count: usize) -> SkinnedMaterial {
        let shader = gen_skinned_wgsl(self.feat);
        let mut desc = PipelineDescriptor::new(shader, Vertex::layout())
            .with_uniform() // group 0: scene
            .with_storage() // group 1: joint matrices
            .with_depth();
        if self.feat.texture {
            desc = desc.with_texture().with_alpha_blend();
        }
        let pipeline = ctx.create_pipeline(desc);
        let ubuf = ctx.create_uniform_buffer(&self.init);

        // Initialise joint buffer with identity matrices.
        let identity_mats: Vec<Mat4> = vec![Mat4::IDENTITY; joint_count];
        let joint_buf = ctx.create_storage_buffer(bytemuck::cast_slice(&identity_mats));

        SkinnedMaterial {
            pipeline,
            ubuf,
            joint_buf,
            uniform: self.init,
        }
    }
}

// ── Material ──────────────────────────────────────────────────────────────────

/// A GPU material for skeletal meshes.
///
/// Mutate [`uniform`](SkinnedMaterial::uniform) each frame, call
/// [`flush`](SkinnedMaterial::flush), then call
/// [`update_joints`](SkinnedMaterial::update_joints) with the current pose.
pub struct SkinnedMaterial {
    pipeline: Pipeline,
    ubuf: UniformBuffer,
    joint_buf: StorageBuffer,
    /// CPU-side copy of the scene uniform. Mutate freely; call
    /// [`flush`](SkinnedMaterial::flush) to upload to the GPU.
    pub uniform: SkinnedMaterialUniform,
}

impl SkinnedMaterial {
    /// Upload [`uniform`](SkinnedMaterial::uniform) to the GPU.
    pub fn flush(&self, ctx: &mut Context) {
        ctx.update_uniform_buffer(&self.ubuf, &self.uniform);
    }

    /// Upload the current joint matrices to the GPU.
    ///
    /// `joints` is typically the return value of [`Animator::joint_matrices`].
    pub fn update_joints(&self, ctx: &mut Context, joints: &[Mat4]) {
        ctx.update_storage_buffer(&self.joint_buf, bytemuck::cast_slice(joints));
    }

    /// Draw the mesh.
    ///
    /// Pass `Some(&texture)` if the material was built with [`.texture()`](SkinnedMaterialBuilder::texture).
    pub fn render(&self, pass: &mut RenderPass, mesh: &GpuMesh, texture: Option<&Texture>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_storage(1, &self.joint_buf);
        if let Some(t) = texture {
            pass.set_texture(2, t);
        }
        mesh.draw(pass);
    }
}

// ── WGSL generation ───────────────────────────────────────────────────────────

const SKINNED_U_WGSL: &str = "
struct SkinnedU {
    view_proj:   mat4x4<f32>,
    model:       mat4x4<f32>,
    color:       vec4<f32>,
    rim_color:   vec4<f32>,
    camera_pos:  vec4<f32>,
    ambient:     AmbientLight,
    directional: DirectionalLight,
}
@group(0) @binding(0) var<uniform> u: SkinnedU;
";

fn gen_skinned_wgsl(feat: Features) -> String {
    let needs_normal = feat.ambient || feat.directional || feat.rim;
    let needs_uv = feat.texture;
    let needs_world_pos = feat.rim;

    let mut loc = 0u32;
    let mut next = || {
        let l = loc;
        loc += 1;
        l
    };
    let normal_loc = needs_normal.then(&mut next);
    let uv_loc = needs_uv.then(&mut next);
    let world_pos_loc = needs_world_pos.then(next);

    let mut s = String::new();

    // Type declarations
    s.push_str(skinning_wgsl());
    s.push_str(AMBIENT_LIGHT_WGSL);
    s.push_str(DIRECTIONAL_LIGHT_WGSL);
    s.push_str(SKINNED_U_WGSL);

    if feat.texture {
        s.push_str(
            "@group(2) @binding(0) var t_diffuse: texture_2d<f32>;\n\
             @group(2) @binding(1) var s_diffuse: sampler;\n",
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
    if let Some(l) = world_pos_loc {
        s.push_str(&format!("    @location({l}) world_pos: vec3<f32>,\n"));
    }
    s.push_str("}\n");

    // Vertex shader — matches Vertex layout: loc 4 = joints, loc 5 = weights.
    s.push_str(
        "@vertex\nfn vs_main(\n\
         \t@location(0) position: vec3<f32>,\n\
         \t@location(1) normal:   vec3<f32>,\n\
         \t@location(2) uv:       vec2<f32>,\n\
         \t@location(4) joints:   vec4<u32>,\n\
         \t@location(5) weights:  vec4<f32>,\n\
         ) -> VOut {\n\
         \tlet skin =\n\
         \t\t  weights.x * joint_mats[joints.x]\n\
         \t\t+ weights.y * joint_mats[joints.y]\n\
         \t\t+ weights.z * joint_mats[joints.z]\n\
         \t\t+ weights.w * joint_mats[joints.w];\n\
         \tlet world = u.model * skin * vec4<f32>(position, 1.0);\n\
         \tvar o: VOut;\n\
         \to.clip = u.view_proj * world;\n",
    );
    if normal_loc.is_some() {
        s.push_str("\to.normal = normalize((u.model * skin * vec4<f32>(normal, 0.0)).xyz);\n");
    }
    if uv_loc.is_some() {
        s.push_str("\to.uv = uv;\n");
    }
    if world_pos_loc.is_some() {
        s.push_str("\to.world_pos = world.xyz;\n");
    }
    s.push_str("\treturn o;\n}\n");

    // Fragment shader
    s.push_str("@fragment\nfn fs_main(v: VOut) -> @location(0) vec4<f32> {\n");

    // Albedo
    if feat.texture {
        s.push_str("\tlet albedo = (textureSample(t_diffuse, s_diffuse, v.uv) * u.color).rgb;\n");
    } else {
        s.push_str("\tlet albedo = u.color.rgb;\n");
    }

    // Lighting
    if feat.ambient || feat.directional {
        s.push_str("\tvar light = vec3<f32>(0.0);\n");
        if feat.ambient {
            s.push_str("\tlight += ambient_light(u.ambient);\n");
        }
        if feat.directional {
            s.push_str("\tlight += directional_light(u.directional, v.normal);\n");
        }
        s.push_str("\tvar color = albedo * light;\n");
    } else {
        s.push_str("\tvar color = albedo;\n");
    }

    // Rim lighting
    if feat.rim {
        s.push_str(
            "\tlet view_dir = normalize(u.camera_pos.xyz - v.world_pos);\n\
             \tlet rim = pow(1.0 - max(dot(v.normal, view_dir), 0.0), 5.0) * 0.85;\n\
             \tcolor += mix(albedo, u.rim_color.rgb, 0.5) * rim;\n",
        );
    }

    s.push_str("\treturn vec4<f32>(color, 1.0);\n}\n");
    s
}
