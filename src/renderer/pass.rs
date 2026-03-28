use super::{
    IndexBuffer, InstanceBuffer, Pipeline, ShadowMap, Texture, UniformBuffer, VertexBuffer,
};

pub struct RenderPass<'a> {
    pub(crate) inner: wgpu::RenderPass<'a>,
}

impl<'a> RenderPass<'a> {
    pub(crate) fn new(inner: wgpu::RenderPass<'a>) -> Self {
        Self { inner }
    }

    pub(crate) fn set_pipeline(&mut self, pipeline: &Pipeline) {
        self.inner.set_pipeline(&pipeline.inner);
    }

    pub(crate) fn set_vertex_buffer(&mut self, slot: u32, buffer: &VertexBuffer) {
        // SAFETY: buffer is stored in user state that outlives this render pass.
        let slice = unsafe {
            std::mem::transmute::<wgpu::BufferSlice<'_>, wgpu::BufferSlice<'a>>(
                buffer.inner.slice(..),
            )
        };
        self.inner.set_vertex_buffer(slot, slice);
    }

    pub(crate) fn set_texture(&mut self, group: u32, texture: &Texture) {
        self.inner.set_bind_group(group, &texture.bind_group, &[]);
    }

    pub(crate) fn set_uniform(&mut self, group: u32, buffer: &UniformBuffer) {
        self.inner.set_bind_group(group, &buffer.bind_group, &[]);
    }

    pub(crate) fn draw(&mut self, vertices: std::ops::Range<u32>) {
        self.inner.draw(vertices, 0..1);
    }

    pub(crate) fn draw_instanced(&mut self, vertices: std::ops::Range<u32>, instance_count: u32) {
        self.inner.draw(vertices, 0..instance_count);
    }

    pub(crate) fn draw_indexed(&mut self, indices: &IndexBuffer) {
        // SAFETY: buffer outlives this render pass (stored in user state).
        let slice = unsafe {
            std::mem::transmute::<wgpu::BufferSlice<'_>, wgpu::BufferSlice<'a>>(
                indices.inner.slice(..),
            )
        };
        self.inner
            .set_index_buffer(slice, wgpu::IndexFormat::Uint32);
        self.inner.draw_indexed(0..indices.count, 0, 0..1);
    }

    /// Draw the first `count` indices from the index buffer.
    pub(crate) fn draw_indexed_count(&mut self, indices: &IndexBuffer, count: u32) {
        let slice = unsafe {
            std::mem::transmute::<wgpu::BufferSlice<'_>, wgpu::BufferSlice<'a>>(
                indices.inner.slice(..),
            )
        };
        self.inner
            .set_index_buffer(slice, wgpu::IndexFormat::Uint32);
        self.inner.draw_indexed(0..count, 0, 0..1);
    }

    pub(crate) fn set_instance_buffer(&mut self, slot: u32, buffer: &InstanceBuffer) {
        let slice = unsafe {
            std::mem::transmute::<wgpu::BufferSlice<'_>, wgpu::BufferSlice<'a>>(
                buffer.inner.slice(..),
            )
        };
        self.inner.set_vertex_buffer(slot, slice);
    }

    pub(crate) fn draw_indexed_instanced(&mut self, indices: &IndexBuffer, instance_count: u32) {
        let slice = unsafe {
            std::mem::transmute::<wgpu::BufferSlice<'_>, wgpu::BufferSlice<'a>>(
                indices.inner.slice(..),
            )
        };
        self.inner
            .set_index_buffer(slice, wgpu::IndexFormat::Uint32);
        self.inner
            .draw_indexed(0..indices.count, 0, 0..instance_count);
    }

    pub(crate) fn set_shadow_map(&mut self, group: u32, shadow_map: &ShadowMap) {
        self.inner
            .set_bind_group(group, &shadow_map.bind_group, &[]);
    }
}
