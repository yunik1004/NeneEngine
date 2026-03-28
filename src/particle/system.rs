use crate::math::{Mat4, Vec3};
use crate::renderer::{
    Context, GpuBatch, InstanceBuffer, PipelineDescriptor, RenderPass, VertexFormat, VertexLayout,
};

use super::emitter::{EmitterConfig, ParticleInstance, QuadVert};
use super::pool::ParticlePool;

// ── WGSL shader ───────────────────────────────────────────────────────────────

const SHADER: &str = r#"
struct Uniforms {
    view_proj : mat4x4<f32>,
    cam_right : vec3<f32>,
    cam_up    : vec3<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertOut {
    @builtin(position) clip  : vec4<f32>,
    @location(0)       color : vec4<f32>,
    @location(1)       uv    : vec2<f32>,
}

@vertex fn vs_main(
    @location(0) corner  : vec2<f32>,
    @location(1) pos_size: vec4<f32>,
    @location(2) color   : vec4<f32>,
) -> VertOut {
    let world = pos_size.xyz
        + u.cam_right * corner.x * pos_size.w
        + u.cam_up   * corner.y * pos_size.w;
    var out: VertOut;
    out.clip  = u.view_proj * vec4<f32>(world, 1.0);
    out.color = color;
    out.uv    = corner + vec2<f32>(0.5);
    return out;
}

@fragment fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    // Soft circle — fade to transparent at edge
    let d = length(in.uv - vec2<f32>(0.5)) * 2.0;
    let alpha = clamp(1.0 - d * d, 0.0, 1.0);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

// ── GPU uniform ───────────────────────────────────────────────────────────────

#[derive(encase::ShaderType)]
struct ParticleUniform {
    view_proj: Mat4,
    cam_right: Vec3,
    cam_up: Vec3,
}

pub const MAX_PARTICLES: usize = 4096;

// ── ParticleSystem (GPU) ──────────────────────────────────────────────────────

/// GPU-backed particle system. Wraps [`ParticlePool`] with a pipeline, quad
/// vertex buffer, and pre-allocated instance buffer.
pub struct ParticleSystem {
    pool: ParticlePool,
    gpu: GpuBatch,
    inst_buf: InstanceBuffer,
    inst_count: u32,
}

impl ParticleSystem {
    pub fn new(ctx: &Context, config: EmitterConfig) -> Self {
        // Vertex layout: slot 0 = corner (Float32x2)
        let vert_layout = VertexLayout {
            stride: std::mem::size_of::<QuadVert>() as u64,
            attributes: vec![crate::renderer::VertexAttribute {
                offset: 0,
                location: 0,
                format: VertexFormat::Float32x2,
            }],
        };
        // Instance layout: slot 1 = pos_size (loc 1, Float32x4), color (loc 2, Float32x4)
        let inst_layout = VertexLayout {
            stride: std::mem::size_of::<ParticleInstance>() as u64,
            attributes: vec![
                crate::renderer::VertexAttribute {
                    offset: 0,
                    location: 1,
                    format: VertexFormat::Float32x4,
                },
                crate::renderer::VertexAttribute {
                    offset: 16,
                    location: 2,
                    format: VertexFormat::Float32x4,
                },
            ],
        };

        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(SHADER, vert_layout)
                .with_instance_layout(inst_layout)
                .with_uniform()
                .with_additive_blend(),
        );

        // 6-vertex unit quad (two triangles)
        let quad_vbuf = ctx.create_vertex_buffer(&[
            QuadVert {
                corner: [-0.5, -0.5],
            },
            QuadVert {
                corner: [0.5, -0.5],
            },
            QuadVert { corner: [0.5, 0.5] },
            QuadVert {
                corner: [-0.5, -0.5],
            },
            QuadVert { corner: [0.5, 0.5] },
            QuadVert {
                corner: [-0.5, 0.5],
            },
        ]);

        let ubuf = ctx.create_uniform_buffer(&ParticleUniform {
            view_proj: Mat4::IDENTITY,
            cam_right: Vec3::X,
            cam_up: Vec3::Y,
        });

        let gpu = GpuBatch::new(pipeline, ubuf, quad_vbuf);

        let dummy = vec![
            ParticleInstance {
                pos_size: [0.0; 4],
                color: [0.0; 4]
            };
            MAX_PARTICLES
        ];
        let inst_buf = ctx.create_instance_buffer(&dummy);

        Self {
            pool: ParticlePool::new(config),
            gpu,
            inst_buf,
            inst_count: 0,
        }
    }

    /// Borrow the underlying CPU pool (e.g. to call `burst`).
    pub fn pool(&self) -> &ParticlePool {
        &self.pool
    }

    pub fn pool_mut(&mut self) -> &mut ParticlePool {
        &mut self.pool
    }

    /// Advance simulation and upload instance data to the GPU.
    ///
    /// `cam_right` / `cam_up` come from the view matrix rows — see example.
    pub fn update(
        &mut self,
        dt: f32,
        emitter_pos: Vec3,
        view_proj: Mat4,
        cam_right: Vec3,
        cam_up: Vec3,
        ctx: &Context,
    ) {
        self.pool.update(dt, emitter_pos.to_array());

        ctx.update_uniform_buffer(
            &self.gpu.ubuf,
            &ParticleUniform {
                view_proj,
                cam_right,
                cam_up,
            },
        );

        let instances = self.pool.instances();
        let count = instances.len().min(MAX_PARTICLES);
        if count > 0 {
            ctx.update_instance_buffer(&self.inst_buf, &instances[..count]);
        }
        self.inst_count = count as u32;
    }

    /// Instantly spawn `count` particles from `pos`.
    pub fn burst(&mut self, count: usize, pos: Vec3) {
        self.pool.burst(count, pos.to_array());
    }

    /// Bind pipeline and draw all live particles.
    pub fn draw(&self, pass: &mut RenderPass) {
        if self.inst_count == 0 {
            return;
        }
        self.gpu
            .draw_instanced(pass, &self.inst_buf, 6, self.inst_count);
    }
}
