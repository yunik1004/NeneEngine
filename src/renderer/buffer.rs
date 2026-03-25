pub struct VertexBuffer {
    pub(crate) inner: wgpu::Buffer,
}

impl VertexBuffer {
    pub fn size(&self) -> u64 {
        self.inner.size()
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
