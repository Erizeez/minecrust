use minecrust_engine::{Camera, CameraUniform, EngineApp, EngineRunner, Renderer, Vertex};
use minecrust_shared::AssetPack;
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
    mesh: Option<Mesh>,
    time: f64,
}

impl MinecrustClient {
    fn new() -> Self {
        Self {
            renderer: None,
            camera: Camera {
                eye: glam::Vec3::new(3.0, 3.0, 3.0),
                target: glam::Vec3::ZERO,
                up: glam::Vec3::Y,
                aspect: 16.0 / 9.0,
                fovy: std::f32::consts::FRAC_PI_4,
                znear: 0.1,
                zfar: 100.0,
            },
            camera_uniform: CameraUniform::new(),
            mesh: None,
            time: 0.0,
        }
    }

    fn build_voxel_mesh(renderer: &Renderer, pack: &AssetPack, block_name: &str) -> Mesh {
        let block_data = pack.block_dict.get(block_name).expect("Block not found in MCA pack!");
        
        // Cube vertices (24 vertices, 4 per face: North, South, East, West, Up, Down)
        // Order matches UV faces: [North, South, East, West, Up, Down]
        // Minecraft Y-Up coordinate system:
        // North: -Z, South: +Z, East: +X, West: -X, Up: +Y, Down: -Y

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let mut add_face = |normal: glam::Vec3, uv_face_idx: usize| {
            let uvs = block_data.uv_faces[uv_face_idx];
            let (u0, v0, u1, v1) = (uvs[0], uvs[1], uvs[2], uvs[3]);
            
            let base_idx = vertices.len() as u16;
            
            // Build generic face perpendicular to normal
            let (tangent, bitangent) = if normal.y.abs() > 0.5 {
                (glam::Vec3::X, glam::Vec3::Z * -normal.y) // Up/Down
            } else {
                (glam::Vec3::Y.cross(normal), glam::Vec3::Y) // Sides
            };

            // 4 corners
            let v0_pos = normal * 0.5 - tangent * 0.5 - bitangent * 0.5;
            let v1_pos = normal * 0.5 + tangent * 0.5 - bitangent * 0.5;
            let v2_pos = normal * 0.5 + tangent * 0.5 + bitangent * 0.5;
            let v3_pos = normal * 0.5 - tangent * 0.5 + bitangent * 0.5;

            vertices.push(Vertex { position: v0_pos.into(), uv: [u0, v1] });
            vertices.push(Vertex { position: v1_pos.into(), uv: [u1, v1] });
            vertices.push(Vertex { position: v2_pos.into(), uv: [u1, v0] });
            vertices.push(Vertex { position: v3_pos.into(), uv: [u0, v0] });

            indices.extend_from_slice(&[
                base_idx, base_idx + 1, base_idx + 2,
                base_idx, base_idx + 2, base_idx + 3,
            ]);
        };

        add_face(glam::Vec3::NEG_Z, 0); // North
        add_face(glam::Vec3::Z, 1);     // South
        add_face(glam::Vec3::X, 2);     // East
        add_face(glam::Vec3::NEG_X, 3); // West
        add_face(glam::Vec3::Y, 4);     // Up
        add_face(glam::Vec3::NEG_Y, 5); // Down

        Mesh {
            vertex_buffer: renderer.create_vertex_buffer(&vertices),
            index_buffer: renderer.create_index_buffer(&indices),
            index_count: indices.len() as u32,
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
            
            // Build dirt mesh
            self.mesh = Some(Self::build_voxel_mesh(&renderer, &pack, "minecraft:dirt"));
        } else {
            log::error!("Failed to load assets.mca! Run asset-cli first.");
        }

        self.renderer = Some(renderer);
    }

    fn on_update(&mut self, dt: f64) {
        self.time += dt;
        
        // Orbit camera
        let radius = 3.0;
        self.camera.eye.x = (self.time.cos() * radius) as f32;
        self.camera.eye.z = (self.time.sin() * radius) as f32;
        self.camera_uniform.update_view_proj(&self.camera);
        
        if let Some(renderer) = &mut self.renderer {
            renderer.update_camera(&self.camera_uniform);
        }
    }

    fn on_render(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            if let Some(mesh) = &self.mesh {
                match renderer.draw_mesh(&mesh.vertex_buffer, &mesh.index_buffer, mesh.index_count) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => log::error!("Out of memory!"),
                    Err(e) => log::error!("{:?}", e),
                }
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
