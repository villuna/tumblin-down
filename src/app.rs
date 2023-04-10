use anyhow::anyhow;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::Window,
};

use crate::{resources, texture};

const CLEAR_COLOUR: wgpu::Color = wgpu::Color {
    r: 0.5,
    g: 0.82,
    b: 0.98,
    a: 1.0,
};

// This is just for testing my render pipeline works now before i implement
// model loading and rendering
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct Vertex {
    pos: [f32; 3],
    tex_coord: [f32; 2],
}

impl Vertex {
    const ATTRS: &[wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRS,
        }
    }
}

const TEST_VERTICES: &[Vertex] = &[
    Vertex {
        pos: [-0.5, 0.5, 0.0],
        tex_coord: [0.0, 0.0],
    },
    Vertex {
        pos: [0.5, 0.5, 0.0],
        tex_coord: [1.0, 0.0],
    },
    Vertex {
        pos: [0.5, -0.5, 0.0],
        tex_coord: [1.0, 1.0],
    },
    Vertex {
        pos: [-0.5, -0.5, 0.0],
        tex_coord: [0.0, 1.0],
    },
];

const TEST_INDICES: &[u16] = &[0, 3, 1, 3, 2, 1];

pub struct App {
    // WGPU stuff
    // TODO: separate this into its own Renderer struct. It should have a nice
    // rusty way of starting and finishing a render pass.
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: PhysicalSize<u32>,
    window: Window,
    // The rest of the app
    // Since this is so simple there's not really much
    test_texture: texture::Texture,
    test_bind_group: wgpu::BindGroup,
    test_vertex_buffer: wgpu::Buffer,
    test_index_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl App {
    pub async fn new(window: Window) -> anyhow::Result<Self> {
        // --- RENDERER CODE ---
        // A lot of this instantiation boilerplate (as well as a lot of the
        // code, to be fair) was taken from the wgpu tutorial at
        // https://sotrh.github.io/learn-wgpu/
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // SAFETY: surface should live as long as the window as they are both
        // owned by the same struct. I'm pretty sure. That's what they said
        // on the tutorial. But aren't self referential structs generally
        // unsafe?
        let surface = unsafe { instance.create_surface(&window) }?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(anyhow!("Error requesting wgpu adapter."))?;

        log::info!("Backend: {:?}", adapter.get_info().backend);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, /*trace_path*/
            )
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);

        let format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.describe().srgb)
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        // -- OTHER STUFF --
        let test_texture =
            texture::Texture::load_texture(&device, &queue, "assets/dababy.jpg").await?;

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture bind group descriptor"),
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

        let test_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("test bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&test_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&test_texture.sampler),
                },
            ],
        });

        let test_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("test vertex buffer"),
            contents: bytemuck::cast_slice(TEST_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let test_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("test index buffer"),
            contents: bytemuck::cast_slice(TEST_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout descriptor"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("texture shader"),
            source: wgpu::ShaderSource::Wgsl(
                resources::load_string("shaders/texture_shader.wgsl").await?.into(),
            ),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: config.format, 
                    blend: Some(wgpu::BlendState::REPLACE), 
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            }, 
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Ok(Self {
            surface,
            config,
            device,
            queue,
            size,
            window,

            test_texture,
            test_bind_group,
            test_vertex_buffer,
            test_index_buffer,
            pipeline,
        })
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOUR),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.test_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.test_vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.test_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..TEST_INDICES.len() as _, 0, 0..1);

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn process_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::H),
                        ..
                    },
                ..
            } => {
                log::info!("hiii!!!! :3");
                true
            }

            _ => false,
        }
    }

    pub fn update(&mut self) {}

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.size = size;
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn size(&self) -> &PhysicalSize<u32> {
        &self.size
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}
