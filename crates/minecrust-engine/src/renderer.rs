use crate::camera::CameraUniform;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
}

#[cfg(target_os = "macos")]
pub struct BlasWrapper(pub metal::AccelerationStructure);

#[cfg(target_os = "macos")]
unsafe impl Send for BlasWrapper {}
#[cfg(target_os = "macos")]
unsafe impl Sync for BlasWrapper {}

pub struct RenderMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    #[cfg(target_os = "macos")]
    pub blas: Option<BlasWrapper>,
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,
    
    // Camera
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    // Atlas Texture (Loaded later)
    atlas_bind_group: Option<wgpu::BindGroup>,
    atlas_bind_group_layout: wgpu::BindGroupLayout,
    
    // G-Buffer
    pub gbuffer_albedo_tex: wgpu::Texture,
    pub gbuffer_normal_tex: wgpu::Texture,
    pub gbuffer_mrao_tex: wgpu::Texture,
    pub final_rt_output_tex: wgpu::Texture,
    pub final_rt_output_view: wgpu::TextureView,
    pub gbuffer_albedo_view: wgpu::TextureView,
    pub gbuffer_normal_view: wgpu::TextureView,
    pub gbuffer_mrao_view: wgpu::TextureView,
    
    // Depth buffer
    depth_texture_view: wgpu::TextureView,

    #[cfg(target_os = "macos")]
    pub metal_rt_ctx: Option<crate::metal_rt::MetalRtContext>,

    // Entities
    entity_buffer: wgpu::Buffer,
    entity_bind_group: wgpu::BindGroup,
    pub entity_alignment: u32,

    pub mesh_registry: std::collections::HashMap<String, Arc<RenderMesh>>,

    pub ui: crate::ui::EngineUi,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let mut required_features = wgpu::Features::empty();
        let adapter_features = adapter.features();
        
        if adapter_features.contains(wgpu::Features::RAY_TRACING_ACCELERATION_STRUCTURE | wgpu::Features::RAY_QUERY) {
            log::info!("Hardware Ray Tracing is supported. Enabling...");
            required_features |= wgpu::Features::RAY_TRACING_ACCELERATION_STRUCTURE | wgpu::Features::RAY_QUERY;
        } else {
            log::error!("Hardware Ray Tracing is NOT supported by this adapter! Disabling RT features.");
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        println!("Surface formats: {:?}", surface_caps.formats);
        println!("Surface usages: {:?}", surface_caps.usages);

        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|&f| f == wgpu::TextureFormat::Rgba16Float)
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Depth Texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Depth Texture"),
            view_formats: &[],
        });
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let create_gbuffer = |label: &str, format: wgpu::TextureFormat| -> (wgpu::Texture, wgpu::TextureView) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
                mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                label: Some(label), view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (texture, view)
        };

        let (gbuffer_albedo_tex, gbuffer_albedo_view) = create_gbuffer("GBuffer Albedo", wgpu::TextureFormat::Bgra8UnormSrgb);
        let (gbuffer_normal_tex, gbuffer_normal_view) = create_gbuffer("GBuffer Normal", wgpu::TextureFormat::Rgba16Float);
        let (gbuffer_mrao_tex, gbuffer_mrao_view) = create_gbuffer("GBuffer MRAO", wgpu::TextureFormat::Rgba8Unorm);
        
        let final_rt_output_tex = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            label: Some("Final RT Output"),
            view_formats: &[],
        });
        let final_rt_output_view = final_rt_output_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Camera Uniform
        let mut camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Atlas Layout
        let atlas_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("atlas_bind_group_layout"),
        });

        // Entity Layout (Dynamic)
        let entity_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
            label: Some("entity_bind_group_layout"),
        });

        let entity_alignment = adapter.limits().min_uniform_buffer_offset_alignment as u32;
        let max_entities = 1024;
        let entity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Entity Uniform Buffer"),
            size: (entity_alignment * max_entities) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let entity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &entity_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &entity_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(64),
                }),
            }],
            label: Some("entity_bind_group"),
        });

        // Pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &atlas_bind_group_layout, &entity_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // Cull back faces
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let ui = crate::ui::EngineUi::new(
            &device,
            config.format,
            None, // No depth testing for UI
            1,
            window.clone(),
        );

        #[cfg(target_os = "macos")]
        let metal_rt_ctx = Some(crate::metal_rt::MetalRtContext::new(&device, &queue));
        #[cfg(not(target_os = "macos"))]
        let metal_rt_ctx = None;

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            atlas_bind_group: None,
            atlas_bind_group_layout,
            gbuffer_albedo_tex,
            gbuffer_normal_tex,
            gbuffer_mrao_tex,
            final_rt_output_tex,
            final_rt_output_view,
            gbuffer_albedo_view,
            gbuffer_normal_view,
            gbuffer_mrao_view,
            depth_texture_view,
            metal_rt_ctx,
            entity_buffer,
            entity_bind_group,
            entity_alignment,
            mesh_registry: std::collections::HashMap::new(),
            ui,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Recreate depth texture
            let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("Depth Texture"),
                view_formats: &[],
            });
            self.depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let create_gbuffer = |label: &str, format: wgpu::TextureFormat| -> (wgpu::Texture, wgpu::TextureView) {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    size: wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
                    mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                    format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                    label: Some(label), view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                (texture, view)
            };

            let (gbuffer_albedo_tex, gbuffer_albedo_view) = create_gbuffer("GBuffer Albedo", wgpu::TextureFormat::Bgra8UnormSrgb);
            self.gbuffer_albedo_tex = gbuffer_albedo_tex;
            self.gbuffer_albedo_view = gbuffer_albedo_view;

            let (gbuffer_normal_tex, gbuffer_normal_view) = create_gbuffer("GBuffer Normal", wgpu::TextureFormat::Rgba16Float);
            self.gbuffer_normal_tex = gbuffer_normal_tex;
            self.gbuffer_normal_view = gbuffer_normal_view;

            let (gbuffer_mrao_tex, gbuffer_mrao_view) = create_gbuffer("GBuffer MRAO", wgpu::TextureFormat::Rgba8Unorm);
            self.gbuffer_mrao_tex = gbuffer_mrao_tex;
            self.gbuffer_mrao_view = gbuffer_mrao_view;

            self.final_rt_output_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
                mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
                label: Some("Final RT Output"),
                view_formats: &[],
            });
            self.final_rt_output_view = self.final_rt_output_tex.create_view(&wgpu::TextureViewDescriptor::default());
        }
    }

    pub fn update_camera(&mut self, camera_uniform: &CameraUniform) {
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*camera_uniform]));
    }

    pub fn load_atlas_bytes(&mut self, albedo_bytes: &[u8], normal_bytes: &[u8], specular_bytes: &[u8], width: u32, height: u32) {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        
        let create_texture = |label: &str, format: wgpu::TextureFormat, bytes: &[u8]| -> wgpu::TextureView {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::ImageCopyTexture { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                bytes,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: Some(height) },
                size,
            );
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let view_albedo = create_texture("Albedo Atlas", wgpu::TextureFormat::Rgba8UnormSrgb, albedo_bytes);
        let view_normal = create_texture("Normal Atlas", wgpu::TextureFormat::Rgba8Unorm, normal_bytes);
        let view_specular = create_texture("Specular Atlas", wgpu::TextureFormat::Rgba8Unorm, specular_bytes);

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        self.atlas_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view_albedo) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view_normal) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&view_specular) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
            label: Some("atlas_bind_group"),
        }));
    }

    pub fn create_vertex_buffer(&self, vertices: &[Vertex]) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    pub fn create_index_buffer(&self, indices: &[u32]) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        })
    }

    pub fn create_render_mesh(&self, vertices: &[Vertex], indices: &[u32]) -> RenderMesh {
        let vertex_buffer = self.create_vertex_buffer(vertices);
        let index_buffer = self.create_index_buffer(indices);
        
        #[cfg(target_os = "macos")]
        let blas = {
            if let Some(metal_rt) = &self.metal_rt_ctx {
                let mtl_vb = unsafe { crate::metal_rt::MetalRtContext::extract_buffer(&vertex_buffer) };
                let mtl_ib = unsafe { crate::metal_rt::MetalRtContext::extract_buffer(&index_buffer) };
                Some(BlasWrapper(metal_rt.build_blas(
                    &mtl_vb,
                    std::mem::size_of::<Vertex>() as u64,
                    &mtl_ib,
                    indices.len() as u32,
                )))
            } else {
                None
            }
        };

        RenderMesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            #[cfg(target_os = "macos")]
            blas,
        }
    }

    pub fn draw_world<'a>(
        &mut self,
        window: &winit::window::Window,
        world: &hecs::World,
        chunk_meshes: impl Iterator<Item = &'a RenderMesh>,
        extra_entity_meshes: impl Iterator<Item = (&'a RenderMesh, &'a glam::Mat4)>,
        ui_builder: impl FnOnce(&egui::Context),
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let raw_input = self.ui.state.take_egui_input(window);
        self.ui.context.begin_pass(raw_input);

        ui_builder(&self.ui.context);

        let full_output = self.ui.context.end_pass();
        self.ui.state.handle_platform_output(window, full_output.platform_output);

        let tris = self.ui.context.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, delta) in &full_output.textures_delta.set {
            self.ui.renderer.update_texture(&self.device, &self.queue, *id, delta);
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        #[cfg(target_os = "macos")]
        let mut tlas_instances = Vec::new();
        #[cfg(target_os = "macos")]
        let mut tlas_blas_owned = Vec::new();

        // 1. Draw 3D world
        if self.atlas_bind_group.is_some() {
            let clear_op = wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store };
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment { view: &self.gbuffer_albedo_view, resolve_target: None, ops: clear_op }),
                    Some(wgpu::RenderPassColorAttachment { view: &self.gbuffer_normal_view, resolve_target: None, ops: clear_op }),
                    Some(wgpu::RenderPassColorAttachment { view: &self.gbuffer_mrao_view, resolve_target: None, ops: clear_op }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, self.atlas_bind_group.as_ref().unwrap(), &[]);
            
            // Collect entity meshes and their matrices to write them to the buffer
            let mut entities_drawn = 0;
            let mut entity_draw_calls = Vec::new();
            
            // Write identity matrix at offset 0 for chunks
            self.queue.write_buffer(&self.entity_buffer, 0, bytemuck::cast_slice(&[glam::Mat4::IDENTITY.to_cols_array_2d()]));
            render_pass.set_bind_group(2, &self.entity_bind_group, &[0]);

            for render_mesh in chunk_meshes {
                render_pass.set_vertex_buffer(0, render_mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(render_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..render_mesh.index_count, 0, 0..1);
                
                #[cfg(target_os = "macos")]
                if let Some(blas) = &render_mesh.blas {
                    let cols = glam::Mat4::IDENTITY.to_cols_array_2d();
                    let transform = [
                        [cols[0][0], cols[0][1], cols[0][2]],
                        [cols[1][0], cols[1][1], cols[1][2]],
                        [cols[2][0], cols[2][1], cols[2][2]],
                        [cols[3][0], cols[3][1], cols[3][2]],
                    ];
                    tlas_instances.push(metal::MTLAccelerationStructureInstanceDescriptor {
                        transformation_matrix: transform,
                        options: metal::MTLAccelerationStructureInstanceOptions::Opaque,
                        mask: 0xFF,
                        intersection_function_table_offset: 0,
                        acceleration_structure_index: tlas_blas_owned.len() as u32,
                    });
                    tlas_blas_owned.push(blas.0.clone());
                }
            }

            // Extra Entities (e.g., remote players without ECS mesh components)
            for (render_mesh, model_mat) in extra_entity_meshes {
                let entity_index = entities_drawn + 1; // 0 is identity
                if entity_index >= 1024 {
                    break;
                }
                
                let offset = entity_index * self.entity_alignment;
                self.queue.write_buffer(&self.entity_buffer, offset as wgpu::BufferAddress, bytemuck::cast_slice(&[model_mat.to_cols_array_2d()]));
                
                entity_draw_calls.push((&render_mesh.vertex_buffer, &render_mesh.index_buffer, render_mesh.index_count, offset));
                entities_drawn += 1;

                #[cfg(target_os = "macos")]
                if let Some(blas) = &render_mesh.blas {
                    let cols = model_mat.to_cols_array_2d();
                    let transform = [
                        [cols[0][0], cols[0][1], cols[0][2]],
                        [cols[1][0], cols[1][1], cols[1][2]],
                        [cols[2][0], cols[2][1], cols[2][2]],
                        [cols[3][0], cols[3][1], cols[3][2]],
                    ];
                    tlas_instances.push(metal::MTLAccelerationStructureInstanceDescriptor {
                        transformation_matrix: transform,
                        options: metal::MTLAccelerationStructureInstanceOptions::Opaque,
                        mask: 0xFF,
                        intersection_function_table_offset: 0,
                        acceleration_structure_index: tlas_blas_owned.len() as u32,
                    });
                    tlas_blas_owned.push(blas.0.clone());
                }
            }

            // ECS Entities
            let mut ecs_entities_to_draw = Vec::new();
            for (mesh_comp, global_transform) in world.query::<(&minecrust_shared::ecs::mesh::Mesh, &minecrust_shared::ecs::transform::GlobalTransform)>().iter() {
                if !mesh_comp.visible { continue; }
                if let Some(render_mesh) = self.mesh_registry.get(&mesh_comp.mesh_id) {
                    ecs_entities_to_draw.push((Arc::clone(render_mesh), global_transform.0));
                }
            }

            for (render_mesh, model_mat) in ecs_entities_to_draw {
                let entity_index = entities_drawn + 1;
                if entity_index >= 1024 { break; }
                
                let offset = entity_index * self.entity_alignment;
                self.queue.write_buffer(&self.entity_buffer, offset as wgpu::BufferAddress, bytemuck::cast_slice(&[model_mat.to_cols_array_2d()]));
                
                render_pass.set_bind_group(2, &self.entity_bind_group, &[offset]);
                render_pass.set_vertex_buffer(0, render_mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(render_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..render_mesh.index_count, 0, 0..1);
                
                entities_drawn += 1;

                #[cfg(target_os = "macos")]
                if let Some(blas) = &render_mesh.blas {
                    let cols = model_mat.to_cols_array_2d();
                    let transform = [
                        [cols[0][0], cols[0][1], cols[0][2]],
                        [cols[1][0], cols[1][1], cols[1][2]],
                        [cols[2][0], cols[2][1], cols[2][2]],
                        [cols[3][0], cols[3][1], cols[3][2]],
                    ];
                    tlas_instances.push(metal::MTLAccelerationStructureInstanceDescriptor {
                        transformation_matrix: transform,
                        options: metal::MTLAccelerationStructureInstanceOptions::Opaque,
                        mask: 0xFF,
                        intersection_function_table_offset: 0,
                        acceleration_structure_index: tlas_blas_owned.len() as u32,
                    });
                    tlas_blas_owned.push(blas.0.clone());
                }
            }

            for (vertex_buffer, index_buffer, index_count, offset) in entity_draw_calls {
                render_pass.set_bind_group(2, &self.entity_bind_group, &[offset as u32]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        #[cfg(target_os = "macos")]
        let tlas = if let Some(metal_rt) = &self.metal_rt_ctx {
            if !tlas_instances.is_empty() {
                let tlas_blas_refs: Vec<&metal::AccelerationStructureRef> = tlas_blas_owned.iter().map(|b| b.as_ref()).collect();
                Some(metal_rt.build_tlas(&tlas_instances, &tlas_blas_refs))
            } else {
                None
            }
        } else {
            None
        };

        // Dispatch Metal Compute Shader!
        #[cfg(target_os = "macos")]
        if let Some(metal_rt) = &self.metal_rt_ctx {
            metal_rt.dispatch(
                &self.final_rt_output_view,
                &self.gbuffer_albedo_view,
                &self.gbuffer_normal_view,
                &self.gbuffer_mrao_view,
                &self.depth_texture_view,
                &self.camera_buffer,
                tlas.as_ref(), // TLAS built from the current frame!
            );
        }

        let mut ui_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("UI Encoder"),
        });

        ui_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.final_rt_output_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &output.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );

        // 2. Draw UI
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        
        self.ui.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut ui_encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let mut ui_pass = ui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Load since Metal compute just wrote the 3D scene to it
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None, // No depth testing for UI
                timestamp_writes: None,
                occlusion_query_set: None,
            }).forget_lifetime();

            self.ui.renderer.render(&mut ui_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.ui.renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(ui_encoder.finish()));
        output.present();

        Ok(())
    }
}
