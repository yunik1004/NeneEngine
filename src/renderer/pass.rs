use super::{Pipeline, Texture, VertexBuffer};

pub struct RenderPass<'a> {
    pub(crate) inner: wgpu::RenderPass<'a>,
}

impl<'a> RenderPass<'a> {
    pub(crate) fn new(inner: wgpu::RenderPass<'a>) -> Self {
        Self { inner }
    }

    pub fn set_pipeline(&mut self, pipeline: &Pipeline) {
        self.inner.set_pipeline(&pipeline.inner);
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &VertexBuffer) {
        // SAFETY: buffer is stored in user state that outlives this render pass.
        let slice = unsafe {
            std::mem::transmute::<wgpu::BufferSlice<'_>, wgpu::BufferSlice<'a>>(
                buffer.inner.slice(..),
            )
        };
        self.inner.set_vertex_buffer(slot, slice);
    }

    pub fn set_texture(&mut self, group: u32, texture: &Texture) {
        self.inner.set_bind_group(group, &texture.bind_group, &[]);
    }

    pub fn draw(&mut self, vertices: std::ops::Range<u32>) {
        self.inner.draw(vertices, 0..1);
    }
}
