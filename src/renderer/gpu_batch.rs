/// Shared GPU draw state: pipeline + uniform buffer + vertex buffer.
///
/// Subsystems embed this to avoid duplicating the three fields and the
/// identical `set_pipeline → set_uniform → set_vertex_buffer` preamble.
pub(crate) struct GpuBatch {
    pub pipeline: Pipeline,
    pub ubuf: UniformBuffer,
    pub vbuf: VertexBuffer,
}

impl GpuBatch {
    pub fn new(pipeline: Pipeline, ubuf: UniformBuffer, vbuf: VertexBuffer) -> Self {
        Self {
            pipeline,
            ubuf,
            vbuf,
        }
    }

    fn setup(&self, pass: &mut RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_vertex_buffer(0, &self.vbuf);
    }

    /// Non-indexed draw — `count` vertices starting at 0.
    pub fn draw(&self, pass: &mut RenderPass, count: u32) {
        self.setup(pass);
        pass.draw(0..count);
    }

    /// Indexed draw using the full index buffer.
    pub fn draw_indexed(&self, pass: &mut RenderPass, ibuf: &IndexBuffer) {
        self.setup(pass);
        pass.draw_indexed(ibuf);
    }

    /// Indexed draw with a texture at group 1 and an explicit index count.
    pub fn draw_textured(
        &self,
        pass: &mut RenderPass,
        texture: &Texture,
        ibuf: &IndexBuffer,
        count: u32,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_texture(1, texture);
        pass.set_vertex_buffer(0, &self.vbuf);
        pass.draw_indexed_count(ibuf, count);
    }

    /// Instanced draw — `inst_buf` at vertex slot 1, `vert_count` vertices per instance.
    pub fn draw_instanced(
        &self,
        pass: &mut RenderPass,
        inst_buf: &InstanceBuffer,
        vert_count: u32,
        inst_count: u32,
    ) {
        self.setup(pass);
        pass.set_instance_buffer(1, inst_buf);
        pass.draw_instanced(0..vert_count, inst_count);
    }
}

use super::{
    IndexBuffer, InstanceBuffer, Pipeline, RenderPass, Texture, UniformBuffer, VertexBuffer,
};
