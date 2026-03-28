pub(crate) struct VertexBuffer {
    pub(crate) inner: wgpu::Buffer,
}

pub(crate) struct InstanceBuffer {
    pub(crate) inner: wgpu::Buffer,
}

pub(crate) struct IndexBuffer {
    pub(crate) inner: wgpu::Buffer,
    pub(crate) count: u32,
}
