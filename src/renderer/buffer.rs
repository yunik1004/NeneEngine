pub struct VertexBuffer {
    pub(crate) inner: wgpu::Buffer,
}

impl VertexBuffer {
    pub fn size(&self) -> u64 {
        self.inner.size()
    }
}

/// A GPU buffer holding per-instance data.
///
/// Create with [`Context::create_instance_buffer`], bind with
/// [`RenderPass::set_instance_buffer`], and draw with
/// [`RenderPass::draw_indexed_instanced`].
pub struct InstanceBuffer {
    pub(crate) inner: wgpu::Buffer,
    /// Number of instances stored in the buffer.
    pub(crate) count: u32,
}

impl InstanceBuffer {
    /// Number of instances currently stored.
    pub fn count(&self) -> u32 {
        self.count
    }
}

pub struct IndexBuffer {
    pub(crate) inner: wgpu::Buffer,
    pub(crate) count: u32,
}

impl IndexBuffer {
    pub fn count(&self) -> u32 {
        self.count
    }
}
