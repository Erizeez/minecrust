use minecrust_engine::input::InputManager;
use minecrust_engine::world::{Mesher, WorldManager, player::PlayerController};
use minecrust_engine::{Camera, CameraUniform, EngineApp, EngineRunner, Renderer, Vertex};
use minecrust_shared::AssetPack;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use winit::window::Window;
use winit::keyboard::{Key, NamedKey};
use winit::event::ElementState;

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
    input_manager: InputManager,
    time: f64,
    asset_pack: Option<AssetPack>,
    
    // Player State
    player: PlayerController,
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
            input_manager: InputManager::new(),
            time: 0.0,
            asset_pack: None,
            player: PlayerController::new(glam::Vec3::new(8.0, 60.0, 8.0)),
        }
    }
}

impl EngineApp for MinecrustClient {
    fn on_init(&mut self, window: Arc<Window>) {
        env_logger::init();
        log::info!("Initializing Minecrust Client...");
        
        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
        window.set_cursor_visible(false);

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

            // Save pack for dynamic loading
            self.asset_pack = Some(pack);
        } else {
            log::error!("Failed to load assets.mca! Run asset-cli first.");
        }

        self.renderer = Some(renderer);
    }

    fn on_update(&mut self, dt: f64) {
        self.time += dt;

        // Update player logic (physics, input, flying)
        self.player.update(&mut self.input_manager, &mut self.world_manager, dt, self.time);

        // Dynamic Chunk Loading
        let player_cx = (self.player.position.x / minecrust_engine::world::chunk::CHUNK_WIDTH as f32).floor() as i32;
        let player_cz = (self.player.position.z / minecrust_engine::world::chunk::CHUNK_DEPTH as f32).floor() as i32;
        
        let render_distance = 4;
        let mut expected_chunks = std::collections::HashSet::new();
        
        for cx in (player_cx - render_distance)..=(player_cx + render_distance) {
            for cz in (player_cz - render_distance)..=(player_cz + render_distance) {
                expected_chunks.insert((cx, cz));
            }
        }

        // Unload old chunks
        self.chunk_meshes.retain(|pos, _| expected_chunks.contains(pos));

        // Load and mesh new chunks
        if let (Some(renderer), Some(pack)) = (&self.renderer, &self.asset_pack) {
            for pos in expected_chunks {
                if !self.chunk_meshes.contains_key(&pos) {
                    let chunk = self.world_manager.chunk_manager.get_or_generate(pos.0, pos.1);
                    
                    let chunk_mesh_data = Mesher::mesh_chunk(chunk, |block_id, face_idx| {
                        let block_name = match block_id {
                            1 => "minecraft:stone",
                            2 => "minecraft:dirt",
                            3 => "minecraft:grass_block",
                            _ => "minecraft:dirt",
                        };
                        let color = if block_id == 3 && face_idx == 4 { // Grass block Top
                            [0.44, 0.70, 0.33] // Plains green tint
                        } else {
                            [1.0, 1.0, 1.0]
                        };

                        if let Some(block_data) = pack.block_dict.get(block_name) {
                            let face = &block_data.uv_faces[face_idx % block_data.uv_faces.len()];
                            ([face[0], face[1], face[2], face[3]], color)
                        } else {
                            ([0.0, 0.0, 0.0, 0.0], color)
                        }
                    });

                    if !chunk_mesh_data.indices.is_empty() {
                        let mesh = Mesh {
                            vertex_buffer: renderer.create_vertex_buffer(&chunk_mesh_data.vertices),
                            index_buffer: renderer.create_index_buffer(&chunk_mesh_data.indices),
                            index_count: chunk_mesh_data.indices.len() as u32,
                        };
                        self.chunk_meshes.insert(pos, mesh);
                    } else {
                        // Insert an empty mesh to mark it as loaded so we don't try again
                        let mesh = Mesh {
                            vertex_buffer: renderer.create_vertex_buffer(&[]),
                            index_buffer: renderer.create_index_buffer(&[]),
                            index_count: 0,
                        };
                        self.chunk_meshes.insert(pos, mesh);
                    }
                }
            }
        }

        // Update Camera Eye and Target
        let (eye, target) = self.player.get_camera_vectors();
        self.camera.eye = eye;
        self.camera.target = target;
        
        self.camera_uniform.update_view_proj(&self.camera);
        
        if let Some(renderer) = &mut self.renderer {
            renderer.update_camera(&self.camera_uniform);
        }
    }

    fn on_keyboard(&mut self, key: Key, state: ElementState) {
        self.input_manager.set_key(key, state == ElementState::Pressed);
    }

    fn on_mouse_move(&mut self, dx: f64, dy: f64) {
        self.input_manager.add_mouse_delta(dx, dy);
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
