use anyhow::anyhow;
use kira::{
    manager::{AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::Window,
};

use crate::camera::Camera;
use crate::input;
use crate::light;
use crate::{
    camera::CameraUniform,
    model::{self, ModelVertex, Vertex},
    resources::{self, load_bytes},
    texture,
};

const CLEAR_COLOUR: wgpu::Color = wgpu::Color {
    r: 0.5,
    g: 0.82,
    b: 0.98,
    a: 1.0,
};

pub const SAMPLE_COUNT: u32 = 4;
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
    pipeline: wgpu::RenderPipeline,
    depth_texture: texture::Texture,
    msaa_texture: wgpu::Texture,
    msaa_view: wgpu::TextureView,
    // The rest of the app
    // Since this is so simple there's not really much
    rei_model: model::Model,
    light_model: model::Model,
    camera: Camera,
    // TODO: Put this into the camera struct
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,

    light_uniform: light::LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    light_pipeline: wgpu::RenderPipeline,

    keyboard: input::KeyboardWatcher,
    // Audio
    song: StaticSoundData,
    audio_manager: AudioManager,
}

fn create_render_pipeline(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::PipelineLayout,
    colour_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: &wgpu::ShaderModule,
    samples: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: colour_format,
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
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview: None,
    })
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture bind group layout descriptor"),
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

        let camera = Camera {
            eye: (0.0, 2.0, 6.0).into(),
            h_angle: 0.0,
            v_angle: 0.0,
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera uniform buffer"),
            contents: bytemuck::cast_slice(&[camera.to_uniform()]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<CameraUniform>() as _,
                        ),
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let light_uniform = light::LightUniform::new([2.0, 3.0, 2.0], [0.96, 0.68, 1.0]);

        let light_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Light buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("light bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light bind group"),
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout descriptor"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &texture_bind_group_layout,
                &light_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model shader"),
            source: wgpu::ShaderSource::Wgsl(
                resources::load_string("shaders/model_shader.wgsl")
                    .await?
                    .into(),
            ),
        });

        log::info!("Creating depth texture...");
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth texture");
        log::info!("Created!");

        let pipeline = create_render_pipeline(
            &device,
            "render pipeline",
            &pipeline_layout,
            config.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[ModelVertex::desc()],
            &shader,
            SAMPLE_COUNT,
        );

        let light_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Light shader"), 
            source: wgpu::ShaderSource::Wgsl(resources::load_string("shaders/light_shader.wgsl").await?.into()),
        });

        let light_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[], 
            });

        let light_pipeline = create_render_pipeline(
            &device,
            "light pipeline",
            &light_pipeline_layout,
            config.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[ModelVertex::desc()],
            &light_shader,
            SAMPLE_COUNT,
        );

        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            view_formats: &[],
        });

        let msaa_view = msaa_texture.create_view(&TextureViewDescriptor::default());

        // -- OTHER STUFF --

        let rei_model = model::Model::load(
            &device,
            &queue,
            "assets/rei/rei.obj",
            Some(&texture_bind_group_layout),
        )
        .await?;
        let light_model = model::Model::load(&device, &queue, "assets/ike.obj", None).await?;

        let song = StaticSoundData::from_cursor(
            std::io::Cursor::new(load_bytes("assets/komm-susser-tod.ogg").await?),
            StaticSoundSettings::default(),
        )?;

        let audio_manager = AudioManager::new(AudioManagerSettings::default())?;

        Ok(Self {
            surface,
            config,
            device,
            queue,
            size,
            window,
            pipeline,
            depth_texture,
            rei_model,
            light_model,
            camera,
            camera_bind_group,
            camera_buffer,
            msaa_texture,
            msaa_view,

            keyboard: input::KeyboardWatcher::new(),
            song,
            audio_manager,
            light_uniform,
            light_buffer,
            light_bind_group,
            light_pipeline,
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
                view: &self.msaa_view,
                resolve_target: Some(&view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOUR),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        // Light Model
        render_pass.set_pipeline(&self.light_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.light_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.light_model.meshes[0].vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.light_model.meshes[0].index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.light_model.meshes[0].num_indices as _, 0, 0..1);

        // Rei
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(2, &self.light_bind_group, &[]);

        for mesh in self.rei_model.meshes.iter() {
            let material = &self.rei_model.materials[mesh.material.unwrap()];

            render_pass.set_bind_group(1, material.diffuse_bind_group.as_ref().unwrap(), &[]);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
        }

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn process_input(&mut self, event: &WindowEvent) -> bool {
        self.keyboard.process_input(event);
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

    pub fn update(&mut self) {
        self.light_uniform.update();
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );

        if self.camera.update(&self.keyboard) {
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera.to_uniform()]),
            );
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.size = size;
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth texture");

            self.msaa_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("msaa texture"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                sample_count: SAMPLE_COUNT,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                view_formats: &[],
            });

            self.msaa_view = self
                .msaa_texture
                .create_view(&TextureViewDescriptor::default());
        }
    }

    pub fn size(&self) -> &PhysicalSize<u32> {
        &self.size
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn play_music(&mut self) {
        self.audio_manager.play(self.song.clone()).unwrap();
    }
}
