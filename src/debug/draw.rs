use std::f32::consts::TAU;

use encase::ShaderType;

use crate::math::{Mat4, Vec3};
use crate::renderer::{
    Context, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexAttribute,
    VertexBuffer, VertexFormat, VertexLayout,
};

// ── Color constants ───────────────────────────────────────────────────────────

/// Predefined RGB colours for debug drawing.
pub mod color {
    use crate::math::Vec3;

    pub const RED: Vec3 = Vec3::new(1.0, 0.0, 0.0);
    pub const GREEN: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    pub const BLUE: Vec3 = Vec3::new(0.0, 0.5, 1.0);
    pub const YELLOW: Vec3 = Vec3::new(1.0, 1.0, 0.0);
    pub const CYAN: Vec3 = Vec3::new(0.0, 1.0, 1.0);
    pub const MAGENTA: Vec3 = Vec3::new(1.0, 0.0, 1.0);
    pub const WHITE: Vec3 = Vec3::new(1.0, 1.0, 1.0);
    pub const ORANGE: Vec3 = Vec3::new(1.0, 0.5, 0.0);
    pub const GRAY: Vec3 = Vec3::new(0.5, 0.5, 0.5);
}

// ── Vertex type ───────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DebugVertex {
    pub pos: [f32; 3],
    pub col: [f32; 3],
}

// ── DebugBuffer ───────────────────────────────────────────────────────────────

/// CPU-side vertex accumulator. GPU-free — useful for unit testing.
///
/// [`DebugDraw`] wraps this and adds GPU upload/draw.
pub struct DebugBuffer {
    pub verts: Vec<DebugVertex>,
}

impl DebugBuffer {
    pub fn new() -> Self {
        Self {
            verts: Vec::with_capacity(1024),
        }
    }

    /// Number of line endpoints accumulated since the last clear.
    pub fn vertex_count(&self) -> usize {
        self.verts.len()
    }

    /// Draw a line segment from `a` to `b`.
    pub fn line(&mut self, a: Vec3, b: Vec3, color: Vec3) {
        let col = color.to_array();
        self.verts.push(DebugVertex {
            pos: a.to_array(),
            col,
        });
        self.verts.push(DebugVertex {
            pos: b.to_array(),
            col,
        });
    }

    /// Draw a ray: `origin` in direction `dir` (normalised) for `length` units.
    pub fn ray(&mut self, origin: Vec3, dir: Vec3, length: f32, color: Vec3) {
        self.line(origin, origin + dir * length, color);
    }

    /// Draw three axis lines (X=red, Y=green, Z=blue) at `origin`.
    pub fn axes(&mut self, origin: Vec3, size: f32) {
        self.line(origin, origin + Vec3::X * size, color::RED);
        self.line(origin, origin + Vec3::Y * size, color::GREEN);
        self.line(origin, origin + Vec3::Z * size, color::BLUE);
    }

    /// Draw a wireframe axis-aligned bounding box.
    pub fn aabb(&mut self, min: Vec3, max: Vec3, color: Vec3) {
        let c = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ];
        self.line(c[0], c[1], color);
        self.line(c[1], c[2], color);
        self.line(c[2], c[3], color);
        self.line(c[3], c[0], color);
        self.line(c[4], c[5], color);
        self.line(c[5], c[6], color);
        self.line(c[6], c[7], color);
        self.line(c[7], c[4], color);
        self.line(c[0], c[4], color);
        self.line(c[1], c[5], color);
        self.line(c[2], c[6], color);
        self.line(c[3], c[7], color);
    }

    /// Draw a wireframe sphere (3 great circles) centred at `center`.
    pub fn sphere(&mut self, center: Vec3, radius: f32, color: Vec3) {
        self.circle(center, Vec3::X, radius, color);
        self.circle(center, Vec3::Y, radius, color);
        self.circle(center, Vec3::Z, radius, color);
    }

    /// Draw a circle in the plane whose normal is `normal`.
    pub fn circle(&mut self, center: Vec3, normal: Vec3, radius: f32, color: Vec3) {
        const SEGS: usize = 24;
        let t = if normal.x.abs() < 0.9 {
            normal.cross(Vec3::X).normalize()
        } else {
            normal.cross(Vec3::Y).normalize()
        };
        let b = normal.cross(t);
        for i in 0..SEGS {
            let a0 = i as f32 * TAU / SEGS as f32;
            let a1 = (i + 1) as f32 * TAU / SEGS as f32;
            let p0 = center + radius * (a0.cos() * t + a0.sin() * b);
            let p1 = center + radius * (a1.cos() * t + a1.sin() * b);
            self.line(p0, p1, color);
        }
    }

    /// Draw a wireframe cylinder between `a` and `b` with the given radius.
    pub fn cylinder(&mut self, a: Vec3, b: Vec3, radius: f32, color: Vec3) {
        const SEGS: usize = 16;
        let axis = (b - a).normalize();
        let t = if axis.x.abs() < 0.9 {
            axis.cross(Vec3::X).normalize()
        } else {
            axis.cross(Vec3::Y).normalize()
        };
        let bt = axis.cross(t);
        let mut prev_a = a + radius * t;
        let mut prev_b = b + radius * t;
        for i in 1..=SEGS {
            let angle = i as f32 * TAU / SEGS as f32;
            let off = radius * (angle.cos() * t + angle.sin() * bt);
            let cur_a = a + off;
            let cur_b = b + off;
            self.line(prev_a, cur_a, color);
            self.line(prev_b, cur_b, color);
            self.line(cur_a, cur_b, color);
            prev_a = cur_a;
            prev_b = cur_b;
        }
        self.circle(a, axis, radius, color);
        self.circle(b, axis, radius, color);
    }

    pub(crate) fn clear(&mut self) {
        self.verts.clear();
    }
}

impl Default for DebugBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ── GPU internals ─────────────────────────────────────────────────────────────

/// Maximum number of line endpoints (vertices) per frame.
pub const MAX_DEBUG_VERTS: usize = 65536;

const SHADER: &str = r#"
struct Uniform { view_proj: mat4x4<f32> }
@group(0) @binding(0) var<uniform> u: Uniform;

struct VIn  { @location(0) pos: vec3<f32>, @location(1) col: vec3<f32> }
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) col: vec3<f32> }

@vertex fn vs_main(v: VIn) -> VOut {
    var out: VOut;
    out.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    out.col  = v.col;
    return out;
}

@fragment fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.col, 1.0);
}
"#;

fn vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: std::mem::size_of::<DebugVertex>() as u64,
        attributes: vec![
            VertexAttribute {
                location: 0,
                offset: 0,
                format: VertexFormat::Float32x3,
            },
            VertexAttribute {
                location: 1,
                offset: 12,
                format: VertexFormat::Float32x3,
            },
        ],
    }
}

#[derive(ShaderType)]
struct DebugUniform {
    view_proj: Mat4,
}

// ── DebugDraw ─────────────────────────────────────────────────────────────────

/// GPU-backed immediate-mode wireframe renderer.
///
/// Create once with [`DebugDraw::new`], call primitive methods each frame,
/// then [`flush`](Self::flush) at end of `update` and [`draw`](Self::draw)
/// inside the render callback.
pub struct DebugDraw {
    buf: DebugBuffer,
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    uniform_buf: UniformBuffer,
    draw_count: u32,
}

impl DebugDraw {
    pub fn new(ctx: &Context) -> Self {
        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(SHADER, vertex_layout())
                .with_uniform()
                .with_lines()
                .with_depth(),
        );
        let placeholder = vec![
            DebugVertex {
                pos: [0.0; 3],
                col: [0.0; 3]
            };
            MAX_DEBUG_VERTS
        ];
        let vbuf = ctx.create_vertex_buffer(&placeholder);
        let uniform_buf = ctx.create_uniform_buffer(&DebugUniform {
            view_proj: Mat4::IDENTITY,
        });

        Self {
            buf: DebugBuffer::new(),
            pipeline,
            vbuf,
            uniform_buf,
            draw_count: 0,
        }
    }

    // ── Primitive forwarding ──────────────────────────────────────────────────

    pub fn line(&mut self, a: Vec3, b: Vec3, color: Vec3) {
        self.buf.line(a, b, color);
    }

    pub fn ray(&mut self, origin: Vec3, dir: Vec3, length: f32, color: Vec3) {
        self.buf.ray(origin, dir, length, color);
    }

    pub fn axes(&mut self, origin: Vec3, size: f32) {
        self.buf.axes(origin, size);
    }

    pub fn aabb(&mut self, min: Vec3, max: Vec3, color: Vec3) {
        self.buf.aabb(min, max, color);
    }

    pub fn sphere(&mut self, center: Vec3, radius: f32, color: Vec3) {
        self.buf.sphere(center, radius, color);
    }

    pub fn circle(&mut self, center: Vec3, normal: Vec3, radius: f32, color: Vec3) {
        self.buf.circle(center, normal, radius, color);
    }

    pub fn cylinder(&mut self, a: Vec3, b: Vec3, radius: f32, color: Vec3) {
        self.buf.cylinder(a, b, radius, color);
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    /// Upload accumulated lines to the GPU, then clear the CPU buffer.
    ///
    /// Call once per frame in `update` after adding all primitives.
    pub fn flush(&mut self, ctx: &Context, view_proj: Mat4) {
        ctx.update_uniform_buffer(&self.uniform_buf, &DebugUniform { view_proj });
        let count = self.buf.verts.len().min(MAX_DEBUG_VERTS);
        if count > 0 {
            ctx.update_vertex_buffer(&self.vbuf, &self.buf.verts[..count]);
        }
        self.draw_count = count as u32;
        self.buf.clear();
    }

    /// Issue a single draw call for all lines from the last [`flush`](Self::flush).
    ///
    /// Call inside the render callback.
    pub fn draw(&self, pass: &mut RenderPass) {
        if self.draw_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.uniform_buf);
        pass.set_vertex_buffer(0, &self.vbuf);
        pass.draw(0..self.draw_count);
    }
}
