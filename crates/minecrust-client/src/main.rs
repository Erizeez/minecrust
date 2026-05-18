use minecrust_engine::world::{Mesher, WorldManager};
use minecrust_engine::{Camera, CameraUniform, EngineApp, EngineRunner, Renderer, Vertex};
use minecrust_shared::AssetPack;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use winit::window::Window;

struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

struct MinecrustClient {
    renderer: Option<Renderer>,
    camera: Camera,
    camera_uniform: CameraUniform,
    world_manager: WorldManager,
    chunk_meshes: HashMap<(i32, i32), Mesh>,
    time: f64,
}

impl MinecrustClient {
    fn new() -> Self {
        Self {
            renderer: None,
            camera: Camera {
                eye: glam::Vec3::new(8.0, 40.0, 8.0),
                target: glam::Vec3::new(8.0, 0.0, 8.0),
                up: glam::Vec3::Y,
                aspect: 16.0 / 9.0,
                fovy: std::f32::consts::FRAC_PI_4,
                znear: 0.1,
                zfar: 1000.0,
            },
            camera_uniform: CameraUniform::new(),
            world_manager: WorldManager::new(12345),
            chunk_meshes: HashMap::new(),
            time: 0.0,
        }
    }
}

impl EngineApp for MinecrustClient {
    fn on_init(&mut self, window: Arc<Window>) {
        env_logger::init();
        log::info!("Initializing Minecrust Client...");
        
        let mut renderer = pollster::block_on(Renderer::new(window));
        
        // Load AssetPack
        let mca_path = "assets/processed/assets.mca";
        log::info!("Loading assets from {}", mca_path);
        if let Ok(mut file) = File::open(mca_path) {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            let pack: AssetPack = bincode::deserialize(&bytes).unwrap();
            log::info!("AssetPack loaded. Atlas size: {} bytes", pack.atlas_png.len());

            renderer.load_atlas_bytes(&pack.atlas_png, 1024, 1024);

            // Generate and mesh 3x3 chunks
            let radius = 1;
            for cx in -radius..=radius {
                for cz in -radius..=radius {
                    let chunk = self.world_manager.chunk_manager.get_or_generate(cx, cz);
                    
                    let chunk_mesh_data = Mesher::mesh_chunk(chunk, |block_id, face_idx| {
                        // Very basic mapping based on hardcoded generator IDs (1: stone, 2: dirt, 3: grass_block)
                        let block_name = match block_id {
                            1 => "minecraft:stone",
                            2 => "minecraft:dirt",
                            3 => "minecraft:grass_block",
                            _ => "minecraft:dirt", // Fallback
                        };
                        if let Some(block_data) = pack.block_dict.get(block_name) {
                            // Grass block needs top/bottom/side logic, but AssetPack usually handles UV faces.
                            // In simplified mesher, 0:North, 1:South, 2:East, 3:West, 4:Up, 5:Down
                            // But Minecraft block models map faces differently. For now we just use face_idx mod len.
                            let face = &block_data.uv_faces[face_idx % block_data.uv_faces.len()];
                            [face[0], face[1], face[2], face[3]]
                        } else {
                            [0.0, 0.0, 0.0, 0.0]
                        }
                    });

                    if chunk_mesh_data.indices.is_empty() { continue; }

                    let mesh = Mesh {
                        vertex_buffer: renderer.create_vertex_buffer(&chunk_mesh_data.vertices),
                        index_buffer: renderer.create_index_buffer(&chunk_mesh_data.indices),
                        index_count: chunk_mesh_data.indices.len() as u32,
                    };

                    self.chunk_meshes.insert((cx, cz), mesh);
                }
            }
        } else {
            log::error!("Failed to load assets.mca! Run asset-cli first.");
        }

        self.renderer = Some(renderer);
    }

    fn on_update(&mut self, dt: f64) {
        self.time += dt;
        
        // Orbit camera around chunk origin
        let radius = 60.0;
        self.camera.eye.x = 8.0 + (self.time.cos() * radius) as f32;
        self.camera.eye.z = 8.0 + (self.time.sin() * radius) as f32;
        self.camera.eye.y = 40.0;
        self.camera.target = glam::Vec3::new(8.0, 10.0, 8.0);
        self.camera_uniform.update_view_proj(&self.camera);
        
        if let Some(renderer) = &mut self.renderer {
            renderer.update_camera(&self.camera_uniform);
        }
    }

    fn on_render(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            let meshes_iter = self.chunk_meshes.values()
                .map(|m| (&m.vertex_buffer, &m.index_buffer, m.index_count));
                
            match renderer.draw_meshes(meshes_iter) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                Err(wgpu::SurfaceError::OutOfMemory) => log::error!("Out of memory!"),
                Err(e) => log::error!("{:?}", e),
            }
        }
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        self.camera.aspect = width as f32 / height as f32;
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(winit::dpi::PhysicalSize::new(width, height));
        }
    }
}

fn main() -> anyhow::Result<()> {
    let app = MinecrustClient::new();
    let runner = EngineRunner::new(app);
    runner.run()
}
