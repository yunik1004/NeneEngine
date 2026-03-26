pub struct Pipeline {
    pub(crate) inner: wgpu::RenderPipeline,
}

pub struct PipelineDescriptor<'a> {
    pub shader: &'a str,
    pub vertex_layout: VertexLayout,
    pub vertex_entry: &'a str,
    pub fragment_entry: &'a str,
    pub use_texture: bool,
    pub use_uniform: bool,
    pub alpha_blend: bool,
    pub depth_write: bool,
    pub use_shadow_map: bool,
    pub depth_only: bool,
    /// Fullscreen-triangle pass: no vertex buffers, no depth stencil.
    pub fullscreen: bool,
}

impl<'a> PipelineDescriptor<'a> {
    pub fn new(shader: &'a str, vertex_layout: VertexLayout) -> Self {
        Self {
            shader,
            vertex_layout,
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
            use_texture: false,
            use_uniform: false,
            alpha_blend: false,
            depth_write: false,
            use_shadow_map: false,
            depth_only: false,
            fullscreen: false,
        }
    }

    /// Fullscreen-triangle pass (no vertex buffers, no depth stencil).
    /// The vertex shader uses `@builtin(vertex_index)` to cover NDC space.
    /// Call [`RenderPass::draw`]`(0..3)` to issue the draw.
    pub fn fullscreen_pass(shader: &'a str) -> Self {
        Self {
            shader,
            vertex_layout: VertexLayout {
                stride: 0,
                attributes: vec![],
            },
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
            use_texture: false,
            use_uniform: false,
            alpha_blend: false,
            depth_write: false,
            use_shadow_map: false,
            depth_only: false,
            fullscreen: true,
        }
    }

    pub fn with_alpha_blend(mut self) -> Self {
        self.alpha_blend = true;
        self
    }

    /// Enable depth testing and writing (needed for 3-D geometry).
    pub fn with_depth(mut self) -> Self {
        self.depth_write = true;
        self
    }

    pub fn with_texture(mut self) -> Self {
        self.use_texture = true;
        self
    }

    /// Add a uniform buffer bind group.
    /// When enabled, uniform is at group 0; texture (if also enabled) moves to group 1.
    pub fn with_uniform(mut self) -> Self {
        self.use_uniform = true;
        self
    }

    pub fn with_shadow_map(mut self) -> Self {
        self.use_shadow_map = true;
        self
    }

    pub fn depth_only(mut self) -> Self {
        self.depth_only = true;
        self.depth_write = true;
        self
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
    /// Four unsigned bytes — maps to `vec4<u32>` in WGSL. Use for joint indices.
    Uint8x4,
}

impl From<VertexFormat> for wgpu::VertexFormat {
    fn from(f: VertexFormat) -> Self {
        match f {
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
            VertexFormat::Uint8x4 => wgpu::VertexFormat::Uint8x4,
        }
    }
}
