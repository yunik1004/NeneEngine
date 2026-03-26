use bytemuck::{Pod, Zeroable};

use crate::{
    camera::Camera,
    math::Vec2,
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, Texture, UniformBuffer,
        VertexAttribute, VertexBuffer, VertexFormat, VertexLayout,
    },
};

const SHADER: &str = r#"
struct ViewProj { vp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: ViewProj;
@group(1) @binding(0) var tex:  texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VertIn {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
    @location(2) color:    vec4<f32>,
};
struct VertOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertIn) -> VertOut {
    var out: VertOut;
    out.clip  = u.vp * vec4<f32>(in.position, 0.0, 1.0);
    out.uv    = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv) * in.color;
}
"#;

// ── Vertex ────────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

fn vertex_layout() -> VertexLayout {
    use std::mem::offset_of;
    VertexLayout {
        stride: std::mem::size_of::<SpriteVertex>() as u64,
        attributes: vec![
            VertexAttribute {
                location: 0,
                offset: offset_of!(SpriteVertex, position) as u64,
                format: VertexFormat::Float32x2,
            },
            VertexAttribute {
                location: 1,
                offset: offset_of!(SpriteVertex, uv) as u64,
                format: VertexFormat::Float32x2,
            },
            VertexAttribute {
                location: 2,
                offset: offset_of!(SpriteVertex, color) as u64,
                format: VertexFormat::Float32x4,
            },
        ],
    }
}

// ── Public types ──────────────────────────────────────────────────────────────

/// Normalized UV rectangle within a texture (for sprite sheets / atlases).
///
/// `(x, y)` is the top-left corner and `(w, h)` is the extent, all in `[0, 1]`.
#[derive(Debug, Clone, Copy)]
pub struct UvRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UvRect {
    /// The full texture — `[0, 0, 1, 1]`.
    pub const FULL: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };
}

impl Default for UvRect {
    fn default() -> Self {
        Self::FULL
    }
}

/// A single sprite to be rendered by [`SpriteBatch`].
#[derive(Debug, Clone)]
pub struct Sprite {
    /// Center position in world space.
    pub position: Vec2,
    /// Width and height in world units.
    pub size: Vec2,
    /// Rotation in radians (counter-clockwise).
    pub rotation: f32,
    /// RGBA color multiplied with the texture sample.
    pub color: [f32; 4],
    /// UV region within the texture.
    pub uv: UvRect,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            size: Vec2::ONE,
            rotation: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
            uv: UvRect::FULL,
        }
    }
}

// ── SpriteBatch ───────────────────────────────────────────────────────────────

/// Batches many sprites into a single draw call per frame.
///
/// # Usage
/// ```text
/// // update:
/// batch.clear();
/// batch.queue(&sprite);
/// batch.prepare(ctx, &camera, aspect);
///
/// // render:
/// batch.render(pass, &texture);
/// ```
pub struct SpriteBatch {
    pipeline: Pipeline,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    uniform_buffer: UniformBuffer,
    cpu_vertices: Vec<SpriteVertex>,
    max_sprites: usize,
    index_count: u32,
}

impl SpriteBatch {
    /// Create a new batch that can hold up to `max_sprites` sprites per frame.
    pub fn new(ctx: &mut Context, max_sprites: usize) -> Self {
        // Static index buffer: each quad uses indices [b, b+1, b+2, b, b+2, b+3].
        let indices: Vec<u32> = (0..max_sprites as u32)
            .flat_map(|i| {
                let b = i * 4;
                [b, b + 1, b + 2, b, b + 2, b + 3]
            })
            .collect();

        // Pre-allocate vertex buffer (zeroed).
        let vertices = vec![
            SpriteVertex {
                position: [0.0; 2],
                uv: [0.0; 2],
                color: [0.0; 4]
            };
            max_sprites * 4
        ];

        let vertex_buffer = ctx.create_vertex_buffer(&vertices);
        let index_buffer = ctx.create_index_buffer(&indices);
        let uniform_buffer = ctx.create_uniform_buffer(&[[0.0f32; 4]; 4]);

        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(SHADER, vertex_layout())
                .with_uniform()
                .with_texture()
                .with_alpha_blend(),
        );

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            cpu_vertices: Vec::with_capacity(max_sprites * 4),
            max_sprites,
            index_count: 0,
        }
    }

    /// Clear queued sprites. Call at the start of each frame.
    pub fn clear(&mut self) {
        self.cpu_vertices.clear();
        self.index_count = 0;
    }

    /// Queue a sprite for rendering. Silently ignored if over `max_sprites`.
    pub fn queue(&mut self, sprite: &Sprite) {
        if self.cpu_vertices.len() / 4 >= self.max_sprites {
            return;
        }

        let hw = sprite.size.x * 0.5;
        let hh = sprite.size.y * 0.5;

        // Local corners in order: bottom-left, bottom-right, top-right, top-left.
        let corners: [[f32; 2]; 4] = [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]];

        // UV corners matching the same winding order.
        let u = sprite.uv;
        let uvs: [[f32; 2]; 4] = [
            [u.x, u.y + u.h],
            [u.x + u.w, u.y + u.h],
            [u.x + u.w, u.y],
            [u.x, u.y],
        ];

        let (sin, cos) = sprite.rotation.sin_cos();

        for (corner, uv) in corners.iter().zip(uvs.iter()) {
            let rx = cos * corner[0] - sin * corner[1];
            let ry = sin * corner[0] + cos * corner[1];
            self.cpu_vertices.push(SpriteVertex {
                position: [sprite.position.x + rx, sprite.position.y + ry],
                uv: *uv,
                color: sprite.color,
            });
        }

        self.index_count += 6;
    }

    /// Upload sprite data to the GPU. Call in the `update` callback after all [`draw`](Self::draw) calls.
    pub fn prepare(&self, ctx: &mut Context, camera: &Camera, aspect: f32) {
        ctx.update_uniform_buffer(
            &self.uniform_buffer,
            &camera.view_proj(aspect).to_cols_array_2d(),
        );
        if !self.cpu_vertices.is_empty() {
            ctx.update_vertex_buffer(&self.vertex_buffer, &self.cpu_vertices);
        }
    }

    /// Issue the draw call. Call in the `render` callback.
    pub fn render(&self, pass: &mut RenderPass<'_>, texture: &Texture) {
        if self.index_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.uniform_buffer);
        pass.set_texture(1, texture);
        pass.set_vertex_buffer(0, &self.vertex_buffer);
        pass.draw_indexed_count(&self.index_buffer, self.index_count);
    }
}
