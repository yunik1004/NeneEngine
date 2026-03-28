//! [`FlatObject`] — 2-D colored shape rendered via the built-in Transform2d pipeline.

use super::{
    BuiltinPipeline, Context, IndexBuffer, Pipeline, RenderPass, TransformUniform, UniformBuffer,
    VertexBuffer,
};
use glam::{Mat4, Vec2, Vec4};

enum Draw {
    NonIndexed(u32),
    Indexed(IndexBuffer),
}

/// A colored 2-D shape rendered with the built-in `Transform2d` pipeline.
///
/// Create once; mutate [`color`](FlatObject::color) as needed, call
/// [`set_transform`](FlatObject::set_transform) each frame, then
/// [`render`](FlatObject::render) inside the render pass.
///
/// # Example
/// ```no_run
/// # use nene::renderer::{Context, FlatObject, RenderPass};
/// # use nene::math::{Mat4, Vec2, Vec4};
/// const QUAD: &[Vec2] = &[
///     Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5),
///     Vec2::new(0.5,  0.5),  Vec2::new(-0.5, -0.5),
///     Vec2::new(0.5,  0.5),  Vec2::new(-0.5,  0.5),
/// ];
/// // window_ready:
/// // let square = FlatObject::new(ctx, QUAD, Vec4::new(0.3, 0.6, 1.0, 1.0));
/// // prepare:
/// // square.set_transform(ctx, mvp);
/// // render:
/// // square.render(pass);
/// ```
pub struct FlatObject {
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    draw: Draw,
    ubuf: UniformBuffer,
    /// Tint color. Mutate freely; uploaded on the next [`set_transform`](Self::set_transform) call.
    pub color: Vec4,
}

impl FlatObject {
    /// Create from a pre-triangulated vertex list (no index buffer).
    pub fn new(ctx: &mut Context, vertices: &[Vec2], color: Vec4) -> Self {
        let count = vertices.len() as u32;
        Self {
            pipeline: ctx.create_builtin_pipeline(BuiltinPipeline::Transform2d),
            vbuf: ctx.create_vertex_buffer(vertices),
            draw: Draw::NonIndexed(count),
            ubuf: ctx.create_uniform_buffer(&TransformUniform::new(Mat4::IDENTITY, color)),
            color,
        }
    }

    /// Create from a vertex list and an index buffer.
    pub fn new_indexed(ctx: &mut Context, vertices: &[Vec2], indices: &[u32], color: Vec4) -> Self {
        Self {
            pipeline: ctx.create_builtin_pipeline(BuiltinPipeline::Transform2d),
            vbuf: ctx.create_vertex_buffer(vertices),
            draw: Draw::Indexed(ctx.create_index_buffer(indices)),
            ubuf: ctx.create_uniform_buffer(&TransformUniform::new(Mat4::IDENTITY, color)),
            color,
        }
    }

    /// Upload a new MVP transform. Uses the current [`color`](Self::color) field.
    /// Call once per frame before [`render`](Self::render).
    pub fn set_transform(&self, ctx: &mut Context, mvp: Mat4) {
        ctx.update_uniform_buffer(&self.ubuf, &TransformUniform::new(mvp, self.color));
    }

    pub fn render(&self, pass: &mut RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_vertex_buffer(0, &self.vbuf);
        match &self.draw {
            Draw::NonIndexed(count) => pass.draw(0..*count),
            Draw::Indexed(ibuf) => pass.draw_indexed(ibuf),
        }
    }
}
