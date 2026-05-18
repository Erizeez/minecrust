use minecrust_engine::input::InputManager;
use minecrust_engine::world::{player::PlayerController, Mesher, WorldManager};
use minecrust_engine::Renderer;
use minecrust_shared::AssetPack;
use std::collections::{HashMap, HashSet};

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

pub struct GameSession {
    pub world_manager: WorldManager,
    pub player: PlayerController,
    pub input_manager: InputManager,
    pub chunk_meshes: HashMap<(i32, i32), Mesh>,
    pub asset_pack: Option<AssetPack>,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            world_manager: WorldManager::new(12345),
            player: PlayerController::new(glam::Vec3::new(8.0, 60.0, 8.0)),
            input_manager: InputManager::new(),
            chunk_meshes: HashMap::new(),
            asset_pack: None,
        }
    }

    pub fn update(&mut self, dt: f64, time: f64, render_distance: i32, renderer: Option<&Renderer>) {
        self.player.update(&mut self.input_manager, &mut self.world_manager, dt, time);

        let player_cx = (self.player.position.x / minecrust_engine::world::chunk::CHUNK_WIDTH as f32).floor() as i32;
        let player_cz = (self.player.position.z / minecrust_engine::world::chunk::CHUNK_DEPTH as f32).floor() as i32;

        let mut expected_chunks = HashSet::new();
        for cx in (player_cx - render_distance)..=(player_cx + render_distance) {
            for cz in (player_cz - render_distance)..=(player_cz + render_distance) {
                expected_chunks.insert((cx, cz));
            }
        }

        // Unload old chunks
        self.chunk_meshes.retain(|pos, _| expected_chunks.contains(pos));

        // Load and mesh new chunks
        if let (Some(renderer), Some(pack)) = (renderer, &self.asset_pack) {
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
                        let color = if block_id == 3 && face_idx == 4 {
                            // Grass block Top
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
    }
}
