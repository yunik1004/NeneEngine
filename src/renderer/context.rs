use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::texture;
use super::{Pipeline, PipelineDescriptor, RenderPass, Texture, VertexBuffer};
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

pub struct HeadlessContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
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

        Some(Self { device, queue })
    }

    pub fn create_vertex_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> VertexBuffer {
        let inner = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX,
            });
        VertexBuffer { inner }
    }

    pub fn submit_empty(&self) {
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.queue.submit([encoder.finish()]);
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

    /// Create a [`TextRenderer`] backed by this headless context.
    /// Uses `Rgba8UnormSrgb` as the target format.
    pub fn create_text_renderer(&self) -> TextRenderer {
        TextRenderer::new_raw(
            &self.device,
            &self.queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }
}

pub struct Context {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
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

        Self {
            surface,
            device,
            queue,
            surface_config,
        }
    }

    pub fn create_vertex_buffer<T: bytemuck::Pod>(&self, data: &[T]) -> VertexBuffer {
        let inner = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX,
            });
        VertexBuffer { inner }
    }

    pub fn create_pipeline(&self, desc: PipelineDescriptor) -> Pipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(desc.shader.into()),
            });

        let texture_layout = desc
            .use_texture
            .then(|| texture::bind_group_layout(&self.device));
        let bind_group_layouts: Vec<Option<&wgpu::BindGroupLayout>> =
            texture_layout.iter().map(Some).collect();
        let layout = self
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

        let inner = self
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
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(desc.fragment_entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        Pipeline { inner }
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

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = RenderPass::new(wgpu_pass);
            draw(&mut pass);
        }

        self.queue.submit([encoder.finish()]);
        frame.present();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}
