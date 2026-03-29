use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::builtin::BuiltinPipeline;
use super::light::{Light, SceneLightsUniform};
use super::shadow::{self, ShadowMap};
use super::texture::{self, FilterMode, RenderTarget, Texture};
use super::uniform;
use super::{
    IndexBuffer, InstanceBuffer, Pipeline, PipelineDescriptor, RenderPass, StorageBuffer,
    UniformBuffer, VertexBuffer,
};
use crate::text::TextRenderer;

fn create_instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        backend_options: wgpu::BackendOptions::default(),
        display: None,
    })
}

/// Shared GPU device + queue with all buffer/texture creation helpers.
pub(crate) struct GpuDevice {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

impl GpuDevice {
    pub(crate) fn create_vertex_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> VertexBuffer {
        let inner = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        VertexBuffer { inner }
    }

    pub(crate) fn update_vertex_buffer<T: bytemuck::Pod>(&self, buf: &VertexBuffer, data: &[T]) {
        self.queue
            .write_buffer(&buf.inner, 0, bytemuck::cast_slice(data));
    }

    pub(crate) fn create_instance_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> InstanceBuffer {
        let inner = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        InstanceBuffer { inner }
    }

    pub(crate) fn update_instance_buffer<T: bytemuck::Pod>(
        &self,
        buf: &InstanceBuffer,
        data: &[T],
    ) {
        self.queue
            .write_buffer(&buf.inner, 0, bytemuck::cast_slice(data));
    }

    pub(crate) fn create_index_buffer(&self, indices: &[u32]) -> IndexBuffer {
        let inner = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        IndexBuffer {
            inner,
            count: indices.len() as u32,
        }
    }

    pub(crate) fn create_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        data: &T,
    ) -> UniformBuffer {
        let mut buf = encase::UniformBuffer::new(Vec::new());
        buf.write(data).unwrap();
        uniform::create(&self.device, buf.into_inner().as_slice())
    }

    pub(crate) fn update_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        buf: &UniformBuffer,
        data: &T,
    ) {
        let mut storage = encase::UniformBuffer::new(Vec::new());
        storage.write(data).unwrap();
        self.queue
            .write_buffer(&buf.inner, 0, storage.into_inner().as_slice());
    }

    pub(crate) fn create_storage_buffer(&self, data: &[u8]) -> StorageBuffer {
        uniform::create_storage(&self.device, data)
    }

    pub(crate) fn update_storage_buffer(&self, buf: &StorageBuffer, data: &[u8]) {
        self.queue.write_buffer(&buf.inner, 0, data);
    }

    pub fn load_texture(&self, path: impl AsRef<std::path::Path>) -> Option<Texture> {
        self.load_texture_with(path, FilterMode::Linear)
    }

    pub fn load_texture_with(
        &self,
        path: impl AsRef<std::path::Path>,
        filter: FilterMode,
    ) -> Option<Texture> {
        let rgba = image::open(path).ok()?.to_rgba8();
        let (w, h) = rgba.dimensions();
        Some(texture::create(
            &self.device,
            &self.queue,
            w,
            h,
            &rgba,
            filter,
        ))
    }

    pub fn load_texture_from_memory(&self, bytes: &[u8]) -> Option<Texture> {
        self.load_texture_from_memory_with(bytes, FilterMode::Linear)
    }

    pub fn load_texture_from_memory_with(
        &self,
        bytes: &[u8],
        filter: FilterMode,
    ) -> Option<Texture> {
        let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
        let (w, h) = rgba.dimensions();
        Some(texture::create(
            &self.device,
            &self.queue,
            w,
            h,
            &rgba,
            filter,
        ))
    }

    pub fn create_texture(&self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        self.create_texture_with(width, height, rgba, FilterMode::Linear)
    }

    pub fn create_texture_with(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        filter: FilterMode,
    ) -> Texture {
        texture::create(&self.device, &self.queue, width, height, rgba, filter)
    }

    pub fn create_shadow_map(&self, size: u32) -> ShadowMap {
        shadow::create(&self.device, size)
    }

    pub fn create_render_target(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> RenderTarget {
        texture::create_render_target(&self.device, width, height, format, false)
    }

    pub fn create_scene_target(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> RenderTarget {
        texture::create_render_target(&self.device, width, height, format, true)
    }

    pub fn create_text_renderer(&self) -> TextRenderer {
        TextRenderer::new_raw(
            &self.device,
            &self.queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    pub(crate) fn create_pipeline(
        &self,
        desc: PipelineDescriptor,
        color_format: wgpu::TextureFormat,
    ) -> Pipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(desc.shader.into()),
            });

        let uniform_layout =
            (desc.uniform_count > 0).then(|| uniform::bind_group_layout(&self.device));
        let storage_layout =
            (desc.storage_count > 0).then(|| uniform::storage_bind_group_layout(&self.device));
        let texture_layout = desc
            .use_texture
            .then(|| texture::bind_group_layout(&self.device));
        let shadow_layout = desc
            .use_shadow_map
            .then(|| shadow::bind_group_layout(&self.device));

        let mut bgl: Vec<Option<&wgpu::BindGroupLayout>> = Vec::new();
        if let Some(u) = &uniform_layout {
            for _ in 0..desc.uniform_count {
                bgl.push(Some(u));
            }
        }
        if let Some(t) = &texture_layout {
            bgl.push(Some(t));
        }
        if let Some(sh) = &shadow_layout {
            bgl.push(Some(sh));
        }
        if let Some(s) = &storage_layout {
            for _ in 0..desc.storage_count {
                bgl.push(Some(s));
            }
        }

        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &bgl,
                immediate_size: 0,
            });

        let vert_attrs: Vec<wgpu::VertexAttribute> = desc
            .vertex_layout
            .attributes
            .into_iter()
            .map(|a| wgpu::VertexAttribute {
                offset: a.offset,
                shader_location: a.location,
                format: a.format.into(),
            })
            .collect();

        // Per-instance attributes (empty when no instance layout is specified).
        let inst_attrs: Vec<wgpu::VertexAttribute> = desc
            .instance_layout
            .as_ref()
            .map(|il| {
                il.attributes
                    .iter()
                    .map(|a| wgpu::VertexAttribute {
                        offset: a.offset,
                        shader_location: a.location,
                        format: a.format.into(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let vert_buf_layout = wgpu::VertexBufferLayout {
            array_stride: desc.vertex_layout.stride,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vert_attrs,
        };

        let mut vb_layouts: Vec<wgpu::VertexBufferLayout> = vec![];
        if !desc.fullscreen {
            vb_layouts.push(vert_buf_layout);
            if let Some(ref il) = desc.instance_layout {
                vb_layouts.push(wgpu::VertexBufferLayout {
                    array_stride: il.stride,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &inst_attrs,
                });
            }
        }

        let blend = if desc.additive_blend {
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            })
        } else if desc.alpha_blend {
            Some(wgpu::BlendState::ALPHA_BLENDING)
        } else {
            Some(wgpu::BlendState::REPLACE)
        };
        let color_target = Some(wgpu::ColorTargetState {
            format: color_format,
            blend,
            write_mask: wgpu::ColorWrites::ALL,
        });
        let color_targets = if desc.depth_only {
            vec![]
        } else {
            vec![color_target]
        };

        let depth_stencil = if desc.fullscreen {
            // Fullscreen passes draw to the swapchain which has a depth attachment.
            // Declare depth format but disable write/test so the pass is no-op for depth.
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(desc.depth_write || desc.depth_only),
                depth_compare: Some(if desc.depth_write || desc.depth_only {
                    wgpu::CompareFunction::LessEqual
                } else {
                    wgpu::CompareFunction::Always
                }),
                stencil: wgpu::StencilState::default(),
                bias: if desc.depth_only {
                    wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 4.0,
                        clamp: 0.0,
                    }
                } else {
                    wgpu::DepthBiasState::default()
                },
            })
        };

        let inner = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(desc.vertex_entry),
                    buffers: &vb_layouts,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: if desc.depth_only {
                    None
                } else {
                    Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some(desc.fragment_entry),
                        targets: &color_targets,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    })
                },
                primitive: wgpu::PrimitiveState {
                    topology: if desc.line_topology {
                        wgpu::PrimitiveTopology::LineList
                    } else {
                        wgpu::PrimitiveTopology::TriangleList
                    },
                    ..Default::default()
                },
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        Pipeline { inner }
    }
}

pub struct HeadlessContext {
    gpu: GpuDevice,
}

impl HeadlessContext {
    pub fn new() -> Option<Self> {
        let adapter = pollster::block_on(create_instance().request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: None,
                force_fallback_adapter: false,
            },
        ))
        .ok()?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

        Some(Self {
            gpu: GpuDevice { device, queue },
        })
    }

    pub(crate) fn create_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        data: &T,
    ) -> UniformBuffer {
        self.gpu.create_uniform_buffer(data)
    }

    pub(crate) fn update_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        buf: &UniformBuffer,
        data: &T,
    ) {
        self.gpu.update_uniform_buffer(buf, data)
    }

    pub fn load_texture(&self, path: impl AsRef<std::path::Path>) -> Option<Texture> {
        self.gpu.load_texture(path)
    }

    pub fn load_texture_with(
        &self,
        path: impl AsRef<std::path::Path>,
        filter: FilterMode,
    ) -> Option<Texture> {
        self.gpu.load_texture_with(path, filter)
    }

    pub fn load_texture_from_memory(&self, bytes: &[u8]) -> Option<Texture> {
        self.gpu.load_texture_from_memory(bytes)
    }

    pub fn load_texture_from_memory_with(
        &self,
        bytes: &[u8],
        filter: FilterMode,
    ) -> Option<Texture> {
        self.gpu.load_texture_from_memory_with(bytes, filter)
    }

    pub fn create_texture(&self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        self.gpu.create_texture(width, height, rgba)
    }

    pub fn create_texture_with(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        filter: FilterMode,
    ) -> Texture {
        self.gpu.create_texture_with(width, height, rgba, filter)
    }

    pub fn submit_empty(&self) {
        let encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.gpu.queue.submit([encoder.finish()]);
    }

    pub fn create_shadow_map(&self, size: u32) -> ShadowMap {
        self.gpu.create_shadow_map(size)
    }

    /// Create a color-only render target (no depth).
    pub fn create_render_target(&self, width: u32, height: u32) -> RenderTarget {
        self.gpu
            .create_render_target(width, height, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    /// Create a render target with a depth buffer.
    pub fn create_scene_target(&self, width: u32, height: u32) -> RenderTarget {
        self.gpu
            .create_scene_target(width, height, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    /// Compile a render pipeline targeting `Rgba8UnormSrgb`.
    pub(crate) fn create_pipeline(&self, desc: PipelineDescriptor) -> Pipeline {
        self.gpu
            .create_pipeline(desc, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    /// Render into an off-screen [`RenderTarget`].
    pub fn render_to_target<F: FnOnce(&mut RenderPass<'_>)>(
        &mut self,
        target: &RenderTarget,
        draw: F,
    ) {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let wgpu_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_to_target"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: target.depth_view.as_ref().map(|dv| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: dv,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = RenderPass::new(wgpu_pass, None);
            draw(&mut pass);
        }
        self.gpu.queue.submit([encoder.finish()]);
    }

    pub fn create_text_renderer(&self) -> TextRenderer {
        self.gpu.create_text_renderer()
    }
}

/// Abstraction over [`Context`] and [`HeadlessContext`] for code that works in both environments.
#[allow(private_interfaces)]
pub(crate) trait RenderContext {
    fn create_scene_target(&self, width: u32, height: u32) -> RenderTarget;
    fn create_pipeline(&self, desc: PipelineDescriptor) -> Pipeline;
    fn create_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        data: &T,
    ) -> UniformBuffer;
    fn update_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        buf: &UniformBuffer,
        data: &T,
    );
}

#[allow(private_interfaces)]
impl RenderContext for Context {
    fn create_scene_target(&self, width: u32, height: u32) -> RenderTarget {
        self.create_scene_target(width, height)
    }
    fn create_pipeline(&self, desc: PipelineDescriptor) -> Pipeline {
        self.create_pipeline(desc)
    }
    fn create_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        data: &T,
    ) -> UniformBuffer {
        self.create_uniform_buffer(data)
    }
    fn update_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        buf: &UniformBuffer,
        data: &T,
    ) {
        self.update_uniform_buffer(buf, data)
    }
}

#[allow(private_interfaces)]
impl RenderContext for HeadlessContext {
    fn create_scene_target(&self, width: u32, height: u32) -> RenderTarget {
        self.create_scene_target(width, height)
    }
    fn create_pipeline(&self, desc: PipelineDescriptor) -> Pipeline {
        self.create_pipeline(desc)
    }
    fn create_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        data: &T,
    ) -> UniformBuffer {
        self.create_uniform_buffer(data)
    }
    fn update_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        buf: &UniformBuffer,
        data: &T,
    ) {
        self.update_uniform_buffer(buf, data)
    }
}

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

pub struct Context {
    gpu: GpuDevice,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    window: Arc<Window>,
    lights_buf: UniformBuffer,
}

impl Context {
    pub fn new(window: Arc<Window>) -> Self {
        let instance = create_instance();

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .or_else(|_| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: true,
            }))
        })
        .expect("No GPU adapter found — update your graphics drivers");

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("Failed to create device");

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let depth_view = create_depth_texture(&device, size.width, size.height);

        let gpu = GpuDevice { device, queue };
        let lights_buf = gpu.create_uniform_buffer(&SceneLightsUniform::default());

        Self {
            gpu,
            surface,
            surface_config,
            depth_view,
            window,
            lights_buf,
        }
    }

    /// Replace the scene-wide light list. Up to [`MAX_LIGHTS`] entries are used.
    ///
    /// Affects all [`Material`]s built with `.lights()`. Call once per frame
    /// before rendering. The default is a soft directional + dim ambient.
    pub fn set_lights(&mut self, lights: &[Light]) {
        let mut u = SceneLightsUniform::default();
        u.set(lights);
        self.gpu.update_uniform_buffer(&self.lights_buf, &u);
    }

    /// Toggle fullscreen. Pass `true` for borderless fullscreen, `false` for windowed.
    pub fn set_fullscreen(&self, fullscreen: bool) {
        use winit::window::Fullscreen;
        self.window.set_fullscreen(if fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        });
    }

    pub(crate) fn create_vertex_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> VertexBuffer {
        self.gpu.create_vertex_buffer(data)
    }

    pub(crate) fn update_vertex_buffer<T: bytemuck::Pod>(&self, buf: &VertexBuffer, data: &[T]) {
        self.gpu.update_vertex_buffer(buf, data)
    }

    pub(crate) fn create_instance_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> InstanceBuffer {
        self.gpu.create_instance_buffer(data)
    }

    pub(crate) fn update_instance_buffer<T: bytemuck::Pod>(
        &self,
        buf: &InstanceBuffer,
        data: &[T],
    ) {
        self.gpu.update_instance_buffer(buf, data)
    }

    pub(crate) fn create_index_buffer(&self, indices: &[u32]) -> IndexBuffer {
        self.gpu.create_index_buffer(indices)
    }

    pub(crate) fn create_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        data: &T,
    ) -> UniformBuffer {
        self.gpu.create_uniform_buffer(data)
    }

    pub(crate) fn update_uniform_buffer<T: encase::ShaderType + encase::internal::WriteInto>(
        &self,
        buf: &UniformBuffer,
        data: &T,
    ) {
        self.gpu.update_uniform_buffer(buf, data)
    }

    pub(crate) fn create_storage_buffer(&self, data: &[u8]) -> StorageBuffer {
        self.gpu.create_storage_buffer(data)
    }

    pub(crate) fn update_storage_buffer(&self, buf: &StorageBuffer, data: &[u8]) {
        self.gpu.update_storage_buffer(buf, data)
    }

    /// Compile a render pipeline targeting the swapchain surface format.
    pub(crate) fn create_pipeline(&self, desc: PipelineDescriptor) -> Pipeline {
        self.gpu.create_pipeline(desc, self.surface_config.format)
    }

    /// Compile one of the engine's pre-built pipelines.
    ///
    /// See [`BuiltinPipeline`] for available variants and their required vertex /
    /// uniform types.
    pub(crate) fn create_builtin_pipeline(&self, pipeline: BuiltinPipeline) -> Pipeline {
        self.gpu
            .create_pipeline(pipeline.descriptor(), self.surface_config.format)
    }

    pub fn load_texture(&self, path: impl AsRef<std::path::Path>) -> Option<Texture> {
        self.gpu.load_texture(path)
    }

    pub fn load_texture_with(
        &self,
        path: impl AsRef<std::path::Path>,
        filter: FilterMode,
    ) -> Option<Texture> {
        self.gpu.load_texture_with(path, filter)
    }

    pub fn load_texture_from_memory(&self, bytes: &[u8]) -> Option<Texture> {
        self.gpu.load_texture_from_memory(bytes)
    }

    pub fn load_texture_from_memory_with(
        &self,
        bytes: &[u8],
        filter: FilterMode,
    ) -> Option<Texture> {
        self.gpu.load_texture_from_memory_with(bytes, filter)
    }

    pub fn create_texture(&self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        self.gpu.create_texture(width, height, rgba)
    }

    pub fn create_texture_with(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        filter: FilterMode,
    ) -> Texture {
        self.gpu.create_texture_with(width, height, rgba, filter)
    }

    pub fn create_shadow_map(&self, size: u32) -> ShadowMap {
        self.gpu.create_shadow_map(size)
    }

    /// Create a color-only render target (no depth). Use for fullscreen effect passes.
    pub fn create_render_target(&self, width: u32, height: u32) -> RenderTarget {
        self.gpu
            .create_render_target(width, height, self.surface_config.format)
    }

    /// Create a render target with a depth buffer. Use for off-screen 3-D scene rendering.
    pub fn create_scene_target(&self, width: u32, height: u32) -> RenderTarget {
        self.gpu
            .create_scene_target(width, height, self.surface_config.format)
    }

    /// Render into an off-screen [`RenderTarget`].
    /// If the target was created with [`create_scene_target`], depth testing is active.
    pub fn render_to_target<F: FnOnce(&mut RenderPass<'_>)>(
        &mut self,
        target: &RenderTarget,
        draw: F,
    ) {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let wgpu_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_to_target"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: target.depth_view.as_ref().map(|dv| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: dv,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = RenderPass::new(wgpu_pass, Some(&self.lights_buf));
            draw(&mut pass);
        }
        self.gpu.queue.submit([encoder.finish()]);
    }

    pub fn shadow_pass<F: FnOnce(&mut RenderPass<'_>)>(&mut self, shadow_map: &ShadowMap, draw: F) {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let wgpu_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_map.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = RenderPass::new(wgpu_pass, None);
            draw(&mut pass);
        }
        self.gpu.queue.submit([encoder.finish()]);
    }

    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.gpu.device
    }

    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.gpu.queue
    }

    pub(crate) fn window(&self) -> &Arc<Window> {
        &self.window
    }

    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }

    pub(crate) fn render_with<F: FnOnce(&mut RenderPass<'_>)>(&mut self, draw: F) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => return,
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let wgpu_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = RenderPass::new(wgpu_pass, Some(&self.lights_buf));
            draw(&mut pass);
        }

        self.gpu.queue.submit([encoder.finish()]);
        frame.present();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface
            .configure(&self.gpu.device, &self.surface_config);
        self.depth_view = create_depth_texture(&self.gpu.device, width, height);
    }
}
