pub struct VertexBuffer {
    pub(crate) inner: wgpu::Buffer,
}

impl VertexBuffer {
    pub fn size(&self) -> u64 {
        self.inner.size()
    }
}
