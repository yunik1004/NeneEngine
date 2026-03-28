pub(crate) struct Pipeline {
    pub(crate) inner: wgpu::RenderPipeline,
}

pub(crate) struct PipelineDescriptor {
    pub shader: String,
    pub vertex_layout: VertexLayout,
    pub vertex_entry: &'static str,
    pub fragment_entry: &'static str,
    pub use_texture: bool,
    /// Number of uniform buffer bind groups (each occupies one group slot).
    pub uniform_count: u32,
    /// Number of read-only storage buffer bind groups, placed after uniforms.
    pub storage_count: u32,
    pub alpha_blend: bool,
    pub additive_blend: bool,
    pub depth_write: bool,
    pub use_shadow_map: bool,
    pub depth_only: bool,
    /// Fullscreen-triangle pass: no vertex buffers, no depth stencil.
    pub fullscreen: bool,
    /// Render lines instead of triangles (`LineList` topology).
    pub line_topology: bool,
    /// Optional per-instance vertex buffer layout (slot 1).
    ///
    /// Use [`VertexLayout::at_locations`] to shift instance attribute
    /// locations past the per-vertex attributes.
    pub instance_layout: Option<VertexLayout>,
}

impl PipelineDescriptor {
    pub fn new(shader: impl Into<String>, vertex_layout: VertexLayout) -> Self {
        Self {
            shader: shader.into(),
            vertex_layout,
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
            use_texture: false,
            uniform_count: 0,
            storage_count: 0,
            alpha_blend: false,
            additive_blend: false,
            depth_write: false,
            use_shadow_map: false,
            depth_only: false,
            fullscreen: false,
            line_topology: false,
            instance_layout: None,
        }
    }

    /// Fullscreen-triangle pass (no vertex buffers, no depth stencil).
    /// The vertex shader uses `@builtin(vertex_index)` to cover NDC space.
    /// Call [`RenderPass::draw`]`(0..3)` to issue the draw.
    pub fn fullscreen_pass(shader: impl Into<String>) -> Self {
        Self {
            shader: shader.into(),
            vertex_layout: VertexLayout {
                stride: 0,
                attributes: vec![],
            },
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
            use_texture: false,
            uniform_count: 0,
            storage_count: 0,
            alpha_blend: false,
            additive_blend: false,
            depth_write: false,
            use_shadow_map: false,
            depth_only: false,
            fullscreen: true,
            line_topology: false,
            instance_layout: None,
        }
    }

    pub fn with_instance_layout(mut self, layout: VertexLayout) -> Self {
        self.instance_layout = Some(layout);
        self
    }

    pub fn with_alpha_blend(mut self) -> Self {
        self.alpha_blend = true;
        self
    }

    /// Additive blending: `src * src_alpha + dst` — ideal for fire, sparks, glows.
    pub fn with_additive_blend(mut self) -> Self {
        self.additive_blend = true;
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

    /// Add one uniform buffer bind group.
    ///
    /// Each call reserves the next group slot. Call twice for two separate
    /// uniform bindings (e.g. scene + joint matrices).
    pub fn with_uniform(mut self) -> Self {
        self.uniform_count += 1;
        self
    }

    /// Add one read-only storage buffer bind group, placed after all uniforms.
    pub fn with_storage(mut self) -> Self {
        self.storage_count += 1;
        self
    }

    /// Use `LineList` topology — draw lines instead of triangles.
    pub fn with_lines(mut self) -> Self {
        self.line_topology = true;
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

pub(crate) struct VertexLayout {
    pub(crate) stride: u64,
    pub(crate) attributes: Vec<VertexAttribute>,
}

pub(crate) struct VertexAttribute {
    pub(crate) offset: u64,
    pub(crate) location: u32,
    pub(crate) format: VertexFormat,
}

#[derive(Clone, Copy)]
pub(crate) enum VertexFormat {
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
