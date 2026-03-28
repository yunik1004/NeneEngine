//! Immediate-mode 2-D shape renderer.
//!
//! [`Draw2d`] is a retained-upload, immediate-style batch that draws colored
//! rects, circles, lines, and triangles without requiring you to touch
//! pipelines, vertex buffers, or WGSL.
//!
//! # Usage
//!
//! ```no_run
//! use nene::app::{App, WindowId, run};
//! use nene::draw2d::Draw2d;
//! use nene::input::Input;
//! use nene::renderer::{Context, RenderPass};
//! use nene::time::Time;
//! use nene::window::Config;
//!
//! const W: f32 = 800.0;
//! const H: f32 = 600.0;
//!
//! struct Game { draw: Option<Draw2d> }
//!
//! impl App for Game {
//!     fn new() -> Self { Game { draw: None } }
//!
//!     fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
//!         self.draw = Some(Draw2d::new(ctx, W, H));
//!     }
//!
//!     fn update(&mut self, _input: &Input, _time: &Time) {
//!         if let Some(d) = &mut self.draw {
//!             d.clear();
//!             d.rect(100.0, 100.0, 200.0, 50.0, [0.2, 0.6, 1.0, 1.0]);
//!             d.circle(400.0, 300.0, 60.0, [1.0, 0.4, 0.2, 1.0]);
//!             d.line(0.0, 0.0, W, H, 2.0, [1.0, 1.0, 0.0, 1.0]);
//!         }
//!     }
//!
//!     fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
//!         if let Some(d) = &mut self.draw { d.flush(ctx); }
//!     }
//!
//!     fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
//!         if let Some(d) = &self.draw { d.render(pass); }
//!     }
//!
//!     fn windows() -> Vec<Config> {
//!         vec![Config { title: "Draw2d", width: W as u32, height: H as u32, ..Default::default() }]
//!     }
//! }
//!
//! fn main() { run::<Game>(); }
//! ```

use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};

use crate::renderer::{
    Context, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexAttribute,
    VertexBuffer, VertexFormat, VertexLayout,
};

// ── Internal vertex ───────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vtx {
    pos: [f32; 2],
    color: [f32; 4],
}

fn vtx_layout() -> VertexLayout {
    VertexLayout {
        stride: std::mem::size_of::<Vtx>() as u64,
        attributes: vec![
            VertexAttribute {
                offset: 0,
                location: 0,
                format: VertexFormat::Float32x2,
            },
            VertexAttribute {
                offset: 8,
                location: 1,
                format: VertexFormat::Float32x4,
            },
        ],
    }
}

// ── WGSL ──────────────────────────────────────────────────────────────────────

const DRAW2D_WGSL: &str = "
struct Scene { proj: mat4x4<f32> }
@group(0) @binding(0) var<uniform> u: Scene;

struct VIn  { @location(0) pos: vec2<f32>, @location(1) color: vec4<f32> }
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) color: vec4<f32> }

@vertex fn vs_main(v: VIn) -> VOut {
    return VOut(u.proj * vec4(v.pos, 0.0, 1.0), v.color);
}

@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    return v.color;
}
";

// ── Uniform ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, encase::ShaderType)]
struct SceneUniform {
    proj: glam::Mat4,
}

// ── Draw2d ────────────────────────────────────────────────────────────────────

const DEFAULT_CAPACITY: usize = 8192;

/// Immediate-mode 2-D shape batch.
///
/// Coordinates are in pixel space: `(0, 0)` is the top-left of the window,
/// positive X goes right, positive Y goes down.
///
/// Call [`clear`](Self::clear) at the start of each frame (typically in
/// `update`), issue draw commands, call [`flush`](Self::flush) in `prepare`,
/// and call [`render`](Self::render) in `render`.
pub struct Draw2d {
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    ubuf: UniformBuffer,
    verts: Vec<Vtx>,
    capacity: usize,
}

impl Draw2d {
    /// Create a new `Draw2d` renderer for a window of size `(width × height)`.
    pub fn new(ctx: &mut Context, width: f32, height: f32) -> Self {
        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(DRAW2D_WGSL, vtx_layout())
                .with_uniform()
                .with_alpha_blend(),
        );
        let vbuf = ctx.create_vertex_buffer(&vec![
            Vtx {
                pos: [0.0; 2],
                color: [0.0; 4]
            };
            DEFAULT_CAPACITY
        ]);
        let ubuf = ctx.create_uniform_buffer(&SceneUniform {
            proj: glam::Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0),
        });
        Self {
            pipeline,
            vbuf,
            ubuf,
            verts: Vec::with_capacity(DEFAULT_CAPACITY),
            capacity: DEFAULT_CAPACITY,
        }
    }

    /// Discard all queued geometry. Call at the start of each frame.
    pub fn clear(&mut self) {
        self.verts.clear();
    }

    // ── Primitives ────────────────────────────────────────────────────────────

    /// Filled axis-aligned rectangle. `(x, y)` is the top-left corner.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let (x1, y1, x2, y2) = (x, y, x + w, y + h);
        self.push_quad([x1, y1], [x2, y1], [x2, y2], [x1, y2], color);
    }

    /// Filled circle approximated with `segments` triangles.
    ///
    /// Defaults to 32 segments when `segments == 0`.
    pub fn circle(&mut self, cx: f32, cy: f32, radius: f32, color: [f32; 4]) {
        self.circle_segments(cx, cy, radius, color, 32);
    }

    /// Filled circle with an explicit segment count.
    pub fn circle_segments(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        color: [f32; 4],
        segments: u32,
    ) {
        let n = segments.max(3) as usize;
        for i in 0..n {
            let a0 = TAU * i as f32 / n as f32;
            let a1 = TAU * (i + 1) as f32 / n as f32;
            self.push_tri(
                [cx, cy],
                [cx + a0.cos() * radius, cy + a0.sin() * radius],
                [cx + a1.cos() * radius, cy + a1.sin() * radius],
                color,
            );
        }
    }

    /// Filled triangle. Vertices are `[x, y]` pairs.
    pub fn triangle(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
        self.push_tri(a, b, c, color);
    }

    /// Thick line from `(x1, y1)` to `(x2, y2)`.
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: [f32; 4]) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt().max(f32::EPSILON);
        let nx = -dy / len * thickness * 0.5;
        let ny = dx / len * thickness * 0.5;
        self.push_quad(
            [x1 + nx, y1 + ny],
            [x2 + nx, y2 + ny],
            [x2 - nx, y2 - ny],
            [x1 - nx, y1 - ny],
            color,
        );
    }

    /// Rectangle outline (four lines) of the given `thickness`.
    pub fn rect_outline(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: [f32; 4],
    ) {
        self.line(x, y, x + w, y, thickness, color); // top
        self.line(x + w, y, x + w, y + h, thickness, color); // right
        self.line(x + w, y + h, x, y + h, thickness, color); // bottom
        self.line(x, y + h, x, y, thickness, color); // left
    }

    // ── GPU ───────────────────────────────────────────────────────────────────

    /// Upload accumulated geometry to the GPU. Call in `prepare`.
    ///
    /// Grows the internal vertex buffer automatically if needed.
    pub fn flush(&mut self, ctx: &mut Context) {
        if self.verts.is_empty() {
            return;
        }
        if self.verts.len() > self.capacity {
            self.capacity = self.verts.len().next_power_of_two();
            self.vbuf = ctx.create_vertex_buffer(&vec![
                Vtx {
                    pos: [0.0; 2],
                    color: [0.0; 4]
                };
                self.capacity
            ]);
        }
        ctx.update_vertex_buffer(&self.vbuf, &self.verts);
    }

    /// Issue draw calls. Call in `render`.
    pub fn render(&self, pass: &mut RenderPass) {
        if self.verts.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_vertex_buffer(0, &self.vbuf);
        pass.draw(0..self.verts.len() as u32);
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn push_tri(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
        self.verts.push(Vtx { pos: a, color });
        self.verts.push(Vtx { pos: b, color });
        self.verts.push(Vtx { pos: c, color });
    }

    fn push_quad(
        &mut self,
        tl: [f32; 2],
        tr: [f32; 2],
        br: [f32; 2],
        bl: [f32; 2],
        color: [f32; 4],
    ) {
        self.push_tri(tl, tr, br, color);
        self.push_tri(tl, br, bl, color);
    }
}
