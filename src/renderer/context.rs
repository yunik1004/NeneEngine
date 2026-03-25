use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::shadow;
use super::shadow::ShadowMap;
use super::texture;
use super::uniform;
use super::{
    IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, Texture, UniformBuffer, VertexBuffer,
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
    pub fn create_vertex_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> VertexBuffer {
        let inner = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        VertexBuffer { inner }
    }

    pub fn update_vertex_buffer<T: bytemuck::Pod>(&self, buf: &VertexBuffer, data: &[T]) {
        self.queue
            .write_buffer(&buf.inner, 0, bytemuck::cast_slice(data));
    }

    pub fn create_index_buffer(&self, indices: &[u32]) -> IndexBuffer {
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

    pub fn create_uniform_buffer<T: bytemuck::Pod>(&self, data: &T) -> UniformBuffer {
        uniform::create(&self.device, bytemuck::bytes_of(data))
    }

    pub fn update_uniform_buffer<T: bytemuck::Pod>(&self, buf: &UniformBuffer, data: &T) {
        self.queue
            .write_buffer(&buf.inner, 0, bytemuck::bytes_of(data));
    }

    pub fn load_texture(&self, path: impl AsRef<std::path::Path>) -> Texture {
        self.load_texture_with(path, texture::FilterMode::Linear)
    }

    pub fn load_texture_with(
        &self,
        path: impl AsRef<std::path::Path>,
        filter: texture::FilterMode,
    ) -> Texture {
        let rgba = image::open(path).expect("Failed to open image").to_rgba8();
        let (w, h) = rgba.dimensions();
        texture::create(&self.device, &self.queue, w, h, &rgba, filter)
    }

    pub fn load_texture_from_memory(&self, bytes: &[u8]) -> Texture {
        self.load_texture_from_memory_with(bytes, texture::FilterMode::Linear)
    }

    pub fn load_texture_from_memory_with(
        &self,
        bytes: &[u8],
        filter: texture::FilterMode,
    ) -> Texture {
        let rgba = image::load_from_memory(bytes)
            .expect("Failed to decode image")
            .to_rgba8();
        let (w, h) = rgba.dimensions();
        texture::create(&self.device, &self.queue, w, h, &rgba, filter)
    }

    pub fn create_texture(&self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        self.create_texture_with(width, height, rgba, texture::FilterMode::Linear)
    }

    pub fn create_texture_with(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        filter: texture::FilterMode,
    ) -> Texture {
        texture::create(&self.device, &self.queue, width, height, rgba, filter)
    }

    pub fn create_shadow_map(&self, size: u32) -> ShadowMap {
        shadow::create(&self.device, size)
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

    pub fn create_vertex_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> VertexBuffer {
        self.gpu.create_vertex_buffer(data)
    }

    pub fn update_vertex_buffer<T: bytemuck::Pod>(&self, buf: &VertexBuffer, data: &[T]) {
        self.gpu.update_vertex_buffer(buf, data)
    }

    pub fn create_index_buffer(&self, indices: &[u32]) -> IndexBuffer {
        self.gpu.create_index_buffer(indices)
    }

    pub fn create_uniform_buffer<T: bytemuck::Pod>(&self, data: &T) -> UniformBuffer {
        self.gpu.create_uniform_buffer(data)
    }

    pub fn update_uniform_buffer<T: bytemuck::Pod>(&self, buf: &UniformBuffer, data: &T) {
        self.gpu.update_uniform_buffer(buf, data)
    }

    pub fn load_texture(&self, path: impl AsRef<std::path::Path>) -> Texture {
        self.gpu.load_texture(path)
    }

    pub fn load_texture_with(
        &self,
        path: impl AsRef<std::path::Path>,
        filter: texture::FilterMode,
    ) -> Texture {
        self.gpu.load_texture_with(path, filter)
    }

    pub fn load_texture_from_memory(&self, bytes: &[u8]) -> Texture {
        self.gpu.load_texture_from_memory(bytes)
    }

    pub fn load_texture_from_memory_with(
        &self,
        bytes: &[u8],
        filter: texture::FilterMode,
    ) -> Texture {
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
        filter: texture::FilterMode,
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

    pub fn create_text_renderer(&self) -> TextRenderer {
        TextRenderer::new_raw(
            &self.gpu.device,
            &self.gpu.queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
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
        .expect("Failed to find adapter");

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

        Self {
            gpu: GpuDevice { device, queue },
            surface,
            surface_config,
            depth_view,
        }
    }

    pub fn create_vertex_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> VertexBuffer {
        self.gpu.create_vertex_buffer(data)
    }

    pub fn update_vertex_buffer<T: bytemuck::Pod>(&self, buf: &VertexBuffer, data: &[T]) {
        self.gpu.update_vertex_buffer(buf, data)
    }

    pub fn create_index_buffer(&self, indices: &[u32]) -> IndexBuffer {
        self.gpu.create_index_buffer(indices)
    }

    pub fn create_uniform_buffer<T: bytemuck::Pod>(&self, data: &T) -> UniformBuffer {
        self.gpu.create_uniform_buffer(data)
    }

    pub fn update_uniform_buffer<T: bytemuck::Pod>(&self, buf: &UniformBuffer, data: &T) {
        self.gpu.update_uniform_buffer(buf, data)
    }

    pub fn create_pipeline(&self, desc: PipelineDescriptor) -> Pipeline {
        let shader = self
            .gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(desc.shader.into()),
            });

        let uniform_layout = desc
            .use_uniform
            .then(|| uniform::bind_group_layout(&self.gpu.device));
        let texture_layout = desc
            .use_texture
            .then(|| texture::bind_group_layout(&self.gpu.device));
        let shadow_layout = desc
            .use_shadow_map
            .then(|| shadow::bind_group_layout(&self.gpu.device));

        let mut bind_group_layouts: Vec<Option<&wgpu::BindGroupLayout>> = Vec::new();
        if let Some(u) = &uniform_layout { bind_group_layouts.push(Some(u)); }
        if let Some(t) = &texture_layout { bind_group_layouts.push(Some(t)); }
        if let Some(s) = &shadow_layout  { bind_group_layouts.push(Some(s)); }

        let layout = self
            .gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &bind_group_layouts,
                immediate_size: 0,
            });

        let attributes: Vec<wgpu::VertexAttribute> = desc
            .vertex_layout
            .attributes
            .into_iter()
            .map(|a| wgpu::VertexAttribute {
                offset: a.offset,
                shader_location: a.location,
                format: a.format.into(),
            })
            .collect();

        let color_target = Some(wgpu::ColorTargetState {
            format: self.surface_config.format,
            blend: Some(if desc.alpha_blend {
                wgpu::BlendState::ALPHA_BLENDING
            } else {
                wgpu::BlendState::REPLACE
            }),
            write_mask: wgpu::ColorWrites::ALL,
        });
        let color_targets = [color_target];

        let inner = self
            .gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(desc.vertex_entry),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: desc.vertex_layout.stride,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attributes,
                    }],
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
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(desc.depth_write || desc.depth_only),
                    depth_compare: Some(if desc.depth_write || desc.depth_only {
                        wgpu::CompareFunction::LessEqual
                    } else {
                        wgpu::CompareFunction::Always
                    }),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        Pipeline { inner }
    }

    pub fn load_texture(&self, path: impl AsRef<std::path::Path>) -> Texture {
        self.gpu.load_texture(path)
    }

    pub fn load_texture_with(
        &self,
        path: impl AsRef<std::path::Path>,
        filter: texture::FilterMode,
    ) -> Texture {
        self.gpu.load_texture_with(path, filter)
    }

    pub fn load_texture_from_memory(&self, bytes: &[u8]) -> Texture {
        self.gpu.load_texture_from_memory(bytes)
    }

    pub fn load_texture_from_memory_with(
        &self,
        bytes: &[u8],
        filter: texture::FilterMode,
    ) -> Texture {
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
        filter: texture::FilterMode,
    ) -> Texture {
        self.gpu.create_texture_with(width, height, rgba, filter)
    }

    pub fn create_shadow_map(&self, size: u32) -> ShadowMap {
        self.gpu.create_shadow_map(size)
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
            let mut pass = RenderPass::new(wgpu_pass);
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
            let mut pass = RenderPass::new(wgpu_pass);
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
