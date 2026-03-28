mod atlas;

pub use atlas::ATLAS_SIZE;

use std::collections::HashMap;

use cosmic_text::{Attrs, Buffer, CacheKey, FontSystem, Metrics, Shaping, SwashCache};
use wgpu::util::DeviceExt;

use crate::renderer::{Context, RenderPass, Texture};
use atlas::{CachedGlyph, RowPacker, TextEntry, TextVertex};

/// Renders text using cosmic-text for shaping and a GPU glyph atlas.
pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    glyph_cache: HashMap<CacheKey, Option<CachedGlyph>>,
    packer: RowPacker,
    atlas_data: Vec<u8>,
    atlas_dirty: bool,
    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    texture_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    pending: Vec<TextEntry>,
    vertex_count: u32,
    vertex_buffer: Option<wgpu::Buffer>,
}

impl TextRenderer {
    pub fn new(ctx: &Context) -> Self {
        Self::new_raw(ctx.device(), ctx.queue(), ctx.surface_config().format)
    }

    pub(crate) fn new_raw(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        // --- Atlas texture ---
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("text_atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text_atlas_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text_atlas_bind_group"),
            layout: &atlas_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        // --- Uniform buffer (screen size) ---
        let uniform_data = [0.0f32, 0.0f32];
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_uniform"),
            contents: bytemuck::cast_slice(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text_uniform_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text_uniform_bind_group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // --- Pipeline ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text_pipeline_layout"),
            bind_group_layouts: &[Some(&atlas_layout), Some(&uniform_layout)],
            immediate_size: 0,
        });

        let make_pipeline =
            |fmt: wgpu::TextureFormat, depth_stencil: Option<wgpu::DepthStencilState>| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("text_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<TextVertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![
                                0 => Float32x2,
                                1 => Float32x2,
                                2 => Float32x4
                            ],
                        }],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: fmt,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                })
            };

        // Surface pipeline: declare depth format but always pass (no write, no test).
        let depth_disabled = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        let pipeline = make_pipeline(format, depth_disabled);
        let texture_pipeline = make_pipeline(wgpu::TextureFormat::Rgba8UnormSrgb, None);

        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            glyph_cache: HashMap::new(),
            packer: RowPacker::new(),
            atlas_data: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            atlas_dirty: false,
            atlas_texture,
            atlas_bind_group,
            pipeline,
            texture_pipeline,
            uniform_buffer,
            uniform_bind_group,
            pending: Vec::new(),
            vertex_count: 0,
            vertex_buffer: None,
        }
    }

    /// Queue a string to be drawn at the given position.
    /// Call `prepare` before rendering.
    pub fn queue(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        self.pending.push(TextEntry {
            text: text.to_owned(),
            x,
            y,
            size,
            color,
        });
    }

    /// Clear all queued text.
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Number of text entries waiting to be rendered.
    pub fn queued_count(&self) -> usize {
        self.pending.len()
    }

    /// Upload queued text glyphs and vertices to the GPU.
    /// Must be called before the render pass that uses `draw_text`.
    pub fn prepare(&mut self, ctx: &Context) {
        let device = ctx.device();
        let queue = ctx.queue();
        let cfg = ctx.surface_config();
        let width = cfg.width as f32;
        let height = cfg.height as f32;

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[width, height]),
        );

        let mut vertices: Vec<TextVertex> = Vec::new();
        let entries: Vec<TextEntry> = std::mem::take(&mut self.pending);

        for entry in &entries {
            let metrics = Metrics::new(entry.size, entry.size * 1.2);
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            buffer.set_size(&mut self.font_system, None, None);
            buffer.set_text(
                &mut self.font_system,
                &entry.text,
                &Attrs::new(),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);

            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical = glyph.physical((entry.x, entry.y + run.line_y), 1.0);
                    let cached = self.ensure_glyph(physical.cache_key);
                    let Some(g) = cached else { continue };
                    if g.width == 0 || g.height == 0 {
                        continue;
                    }

                    let gx = (physical.x + g.offset_x) as f32;
                    let gy = (physical.y - g.offset_y) as f32;
                    let gw = g.width as f32;
                    let gh = g.height as f32;
                    let u0 = g.atlas_x as f32 / ATLAS_SIZE as f32;
                    let v0 = g.atlas_y as f32 / ATLAS_SIZE as f32;
                    let u1 = (g.atlas_x + g.width) as f32 / ATLAS_SIZE as f32;
                    let v1 = (g.atlas_y + g.height) as f32 / ATLAS_SIZE as f32;

                    let c = entry.color;
                    let tl = TextVertex {
                        pos: [gx, gy],
                        uv: [u0, v0],
                        color: c,
                    };
                    let tr = TextVertex {
                        pos: [gx + gw, gy],
                        uv: [u1, v0],
                        color: c,
                    };
                    let bl = TextVertex {
                        pos: [gx, gy + gh],
                        uv: [u0, v1],
                        color: c,
                    };
                    let br = TextVertex {
                        pos: [gx + gw, gy + gh],
                        uv: [u1, v1],
                        color: c,
                    };
                    vertices.extend_from_slice(&[tl, tr, bl, tr, br, bl]);
                }
            }
        }

        if self.atlas_dirty {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.atlas_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ATLAS_SIZE),
                    rows_per_image: Some(ATLAS_SIZE),
                },
                wgpu::Extent3d {
                    width: ATLAS_SIZE,
                    height: ATLAS_SIZE,
                    depth_or_array_layers: 1,
                },
            );
            self.atlas_dirty = false;
        }

        self.vertex_count = vertices.len() as u32;
        if !vertices.is_empty() {
            self.vertex_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("text_vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        } else {
            self.vertex_buffer = None;
        }
    }

    fn ensure_glyph(&mut self, key: CacheKey) -> Option<&CachedGlyph> {
        if !self.glyph_cache.contains_key(&key) {
            let image = self
                .swash_cache
                .get_image_uncached(&mut self.font_system, key);

            let cached = image.and_then(|img| {
                let placement = img.placement;
                let w = placement.width;
                let h = placement.height;

                if w == 0 || h == 0 {
                    return Some(CachedGlyph {
                        atlas_x: 0,
                        atlas_y: 0,
                        width: 0,
                        height: 0,
                        offset_x: placement.left,
                        offset_y: placement.top,
                    });
                }

                let (ax, ay) = self.packer.alloc(w, h)?;

                for row in 0..h {
                    for col in 0..w {
                        let src_i = (row * w + col) as usize;
                        let dst_i = ((ay + row) * ATLAS_SIZE + (ax + col)) as usize;
                        let alpha = img.data.get(src_i).copied().unwrap_or(0);
                        if let Some(byte) = self.atlas_data.get_mut(dst_i) {
                            *byte = alpha;
                        }
                    }
                }
                self.atlas_dirty = true;

                Some(CachedGlyph {
                    atlas_x: ax,
                    atlas_y: ay,
                    width: w,
                    height: h,
                    offset_x: placement.left,
                    offset_y: placement.top,
                })
            });

            self.glyph_cache.insert(key, cached);
        }

        self.glyph_cache.get(&key)?.as_ref()
    }

    /// Render all queued text into a new `width × height` texture and return it.
    ///
    /// Coordinates are in pixels relative to the texture's top-left corner.
    /// The returned [`Texture`] can be passed to [`RenderPass::set_texture`].
    pub fn render_to_texture(&mut self, ctx: &Context, width: u32, height: u32) -> Texture {
        let device = ctx.device();
        let queue = ctx.queue();

        let uniform_data = [width as f32, height as f32];
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_texture_uniform"),
            contents: bytemuck::cast_slice(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let entries: Vec<TextEntry> = std::mem::take(&mut self.pending);
        let mut vertices: Vec<TextVertex> = Vec::new();

        for entry in &entries {
            let metrics = cosmic_text::Metrics::new(entry.size, entry.size * 1.2);
            let mut buffer = cosmic_text::Buffer::new(&mut self.font_system, metrics);
            buffer.set_size(&mut self.font_system, None, None);
            buffer.set_text(
                &mut self.font_system,
                &entry.text,
                &cosmic_text::Attrs::new(),
                cosmic_text::Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);

            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical = glyph.physical((entry.x, entry.y + run.line_y), 1.0);
                    let cached = self.ensure_glyph(physical.cache_key);
                    let Some(g) = cached else { continue };
                    if g.width == 0 || g.height == 0 {
                        continue;
                    }

                    let gx = (physical.x + g.offset_x) as f32;
                    let gy = (physical.y - g.offset_y) as f32;
                    let gw = g.width as f32;
                    let gh = g.height as f32;
                    let u0 = g.atlas_x as f32 / ATLAS_SIZE as f32;
                    let v0 = g.atlas_y as f32 / ATLAS_SIZE as f32;
                    let u1 = (g.atlas_x + g.width) as f32 / ATLAS_SIZE as f32;
                    let v1 = (g.atlas_y + g.height) as f32 / ATLAS_SIZE as f32;
                    let c = entry.color;
                    let tl = TextVertex {
                        pos: [gx, gy],
                        uv: [u0, v0],
                        color: c,
                    };
                    let tr = TextVertex {
                        pos: [gx + gw, gy],
                        uv: [u1, v0],
                        color: c,
                    };
                    let bl = TextVertex {
                        pos: [gx, gy + gh],
                        uv: [u0, v1],
                        color: c,
                    };
                    let br = TextVertex {
                        pos: [gx + gw, gy + gh],
                        uv: [u1, v1],
                        color: c,
                    };
                    vertices.extend_from_slice(&[tl, tr, bl, tr, br, bl]);
                }
            }
        }

        if self.atlas_dirty {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.atlas_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ATLAS_SIZE),
                    rows_per_image: Some(ATLAS_SIZE),
                },
                wgpu::Extent3d {
                    width: ATLAS_SIZE,
                    height: ATLAS_SIZE,
                    depth_or_array_layers: 1,
                },
            );
            self.atlas_dirty = false;
        }

        let target = crate::renderer::texture::create_render_target(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            false,
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text_to_texture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !vertices.is_empty() {
                let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("text_texture_vb"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                pass.set_pipeline(&self.texture_pipeline);
                pass.set_bind_group(0, &self.atlas_bind_group, &[]);
                pass.set_bind_group(1, &uniform_bg, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
        }
        queue.submit([encoder.finish()]);

        target.into_texture()
    }

    /// Render prepared text into a render pass.
    /// Call [`prepare`](Self::prepare) before the render pass that uses this.
    pub fn render(&self, pass: &mut RenderPass<'_>) {
        let Some(ref vb) = self.vertex_buffer else {
            return;
        };
        if self.vertex_count == 0 {
            return;
        }
        let inner = &mut pass.inner;
        inner.set_pipeline(&self.pipeline);
        inner.set_bind_group(0, &self.atlas_bind_group, &[]);
        inner.set_bind_group(1, &self.uniform_bind_group, &[]);
        inner.set_vertex_buffer(0, vb.slice(..));
        inner.draw(0..self.vertex_count, 0..1);
    }
}
