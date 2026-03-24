pub struct Pipeline {
    pub(crate) inner: wgpu::RenderPipeline,
}

pub struct PipelineDescriptor<'a> {
    pub shader: &'a str,
    pub vertex_layout: VertexLayout,
    pub vertex_entry: &'a str,
    pub fragment_entry: &'a str,
}

impl<'a> PipelineDescriptor<'a> {
    pub fn new(shader: &'a str, vertex_layout: VertexLayout) -> Self {
        Self {
            shader,
            vertex_layout,
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
        }
    }
}

pub struct VertexLayout {
    pub stride: u64,
    pub attributes: Vec<VertexAttribute>,
}

pub struct VertexAttribute {
    pub offset: u64,
    pub location: u32,
    pub format: VertexFormat,
}

#[derive(Clone, Copy)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
}

impl From<VertexFormat> for wgpu::VertexFormat {
    fn from(f: VertexFormat) -> Self {
        match f {
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
        }
    }
}
