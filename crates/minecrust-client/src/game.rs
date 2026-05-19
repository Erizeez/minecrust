use minecrust_engine::input::InputManager;
use minecrust_engine::world::{Mesher, WorldManager};
use minecrust_engine::Renderer;
use minecrust_shared::ecs::player::{Player, CameraMode};
use minecrust_shared::ecs::transform::{LocalTransform, GlobalTransform, Children, Parent};
use minecrust_shared::ecs::animation::{Animator, Bone, BoneType};
use minecrust_shared::ecs::mesh::Mesh as EcsMesh;
use minecrust_engine::systems::player::player_movement_system;
use minecrust_engine::systems::transform::transform_update_system;
use minecrust_engine::systems::animation::procedural_animation_system;
use minecrust_engine::world::lod::LodGenerator;
use minecrust_engine::world::lod_mesher::LodMesher;
use minecrust_shared::AssetPack;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use log::{info, warn};

use minecrust_shared::protocol::{ClientMessage, ServerMessage};

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

pub struct RemotePlayer {
    pub username: String,
    pub position: glam::Vec3,
    pub mesh: Option<Mesh>,
}

pub struct GameSession {
    pub world_manager: WorldManager,
    pub local_player_entity: hecs::Entity,
    pub input_manager: InputManager,
    pub chunk_meshes: HashMap<(i32, i32), Mesh>,
    pub asset_pack: Option<Arc<AssetPack>>,
    pub other_players: HashMap<u32, RemotePlayer>,
    pub local_player_mesh: Option<Mesh>,
    
    // C/S Channels
    server_tx: Sender<ClientMessage>,
    server_rx: Receiver<ServerMessage>,
    sent_requests: HashSet<(i32, i32)>,
    joined: bool,
    
    // Async Meshing
    mesh_tx: Sender<((i32, i32), minecrust_engine::world::ChunkMesh)>,
    mesh_rx: Receiver<((i32, i32), minecrust_engine::world::ChunkMesh)>,
    pub meshing_chunks: HashSet<(i32, i32)>,
    
    // LOD Meshing
    pub lod_meshes: HashMap<(u8, i32, i32), Mesh>,
    pub meshing_lods: HashSet<(u8, i32, i32)>,
    mesh_lod_tx: Sender<((u8, i32, i32), minecrust_engine::world::ChunkMesh)>,
    mesh_lod_rx: Receiver<((u8, i32, i32), minecrust_engine::world::ChunkMesh)>,
}

impl GameSession {
    pub fn new(server_tx: Sender<ClientMessage>, server_rx: Receiver<ServerMessage>) -> Self {
        let mut world_manager = WorldManager::new(12345);
        
        let local_player_entity = minecrust_engine::world::prefab::spawn_steve(&mut world_manager.ecs, glam::Vec3::new(8.0, 60.0, 8.0));

        let (mesh_tx, mesh_rx) = crossbeam_channel::unbounded();
        let (mesh_lod_tx, mesh_lod_rx) = crossbeam_channel::unbounded();

        let mut session = Self {
            world_manager,
            local_player_entity,
            input_manager: InputManager::new(),
            chunk_meshes: HashMap::new(),
            asset_pack: None,
            other_players: HashMap::new(),
            local_player_mesh: None,
            server_tx,
            server_rx,
            sent_requests: HashSet::new(),
            joined: false,
            mesh_tx,
            mesh_rx,
            meshing_chunks: HashSet::new(),
            lod_meshes: HashMap::new(),
            meshing_lods: HashSet::new(),
            mesh_lod_tx,
            mesh_lod_rx,
        };

        // Send a join request immediately on session startup
        if let Err(e) = session.server_tx.send(ClientMessage::Join { username: "Player".to_string() }) {
            warn!("Failed to send Join packet to integrated server: {:?}", e);
        }

        session
    }

    pub fn update(&mut self, dt: f64, time: f64, render_distance: i32, local_model: crate::steve::PlayerModelType, renderer: Option<&Renderer>) {
        // 1. Process all incoming Server Messages (non-blocking)
        while let Ok(msg) = self.server_rx.try_recv() {
            self.handle_server_message(msg);
        }

        // 2. Perform local client player movement
        player_movement_system(&mut self.world_manager.ecs, &mut self.input_manager, &self.world_manager.chunk_manager, dt, time);
        
        // 2.5 Run ECS Animations and Transforms
        procedural_animation_system(&mut self.world_manager.ecs, dt as f32);
        transform_update_system(&mut self.world_manager.ecs);

        let (player_pos, player_yaw, player_pitch) = {
            if let Ok(transform) = self.world_manager.ecs.get::<&LocalTransform>(self.local_player_entity) {
                if let Ok(player) = self.world_manager.ecs.get::<&Player>(self.local_player_entity) {
                    (transform.translation, player.yaw, player.pitch)
                } else {
                    (glam::Vec3::ZERO, 0.0, 0.0)
                }
            } else {
                (glam::Vec3::ZERO, 0.0, 0.0)
            }
        };

        // Send movement update to server (could rate limit this, but local channel is fine)
        let _ = self.server_tx.send(ClientMessage::PlayerMove {
            x: player_pos.x,
            y: player_pos.y,
            z: player_pos.z,
            yaw: player_yaw,
            pitch: player_pitch,
        });

        // 3. Update chunk load subscriptions
        let player_cx = (player_pos.x / minecrust_engine::world::CHUNK_WIDTH as f32).floor() as i32;
        let player_cz = (player_pos.z / minecrust_engine::world::CHUNK_DEPTH as f32).floor() as i32;

        let mut expected_chunks = HashSet::new();
        for cx in (player_cx - render_distance)..=(player_cx + render_distance) {
            for cz in (player_cz - render_distance)..=(player_cz + render_distance) {
                expected_chunks.insert((cx, cz));
            }
        }

        let mut expected_lods = HashSet::new();
        for level in 1..4 {
            let range = render_distance * (level as i32 + 1);
            for cx in (player_cx - range)..=(player_cx + range) {
                for cz in (player_cz - range)..=(player_cz + range) {
                    if !expected_chunks.contains(&(cx, cz)) {
                        expected_lods.insert((level as u8, cx, cz));
                    }
                }
            }
        }

        // Receive LOD meshes
        if let Some(renderer) = renderer {
            while let Ok((pos, chunk_mesh_data)) = self.mesh_lod_rx.try_recv() {
                self.meshing_lods.remove(&pos);
                if !chunk_mesh_data.indices.is_empty() {
                    let mesh = Mesh {
                        vertex_buffer: renderer.create_vertex_buffer(&chunk_mesh_data.vertices),
                        index_buffer: renderer.create_index_buffer(&chunk_mesh_data.indices),
                        index_count: chunk_mesh_data.indices.len() as u32,
                    };
                    self.lod_meshes.insert(pos, mesh);
                }
            }
        }

        // 3. Unload out-of-range chunks meshes and local chunks
        self.chunk_meshes.retain(|pos, _| expected_chunks.contains(pos));
        self.world_manager.chunk_manager.chunks.retain(|pos, _| expected_chunks.contains(pos));
        self.sent_requests.retain(|pos| expected_chunks.contains(pos));

        // Request missing chunks asynchronously from the server
        for pos in &expected_chunks {
            if !self.world_manager.chunk_manager.chunks.contains_key(pos) && !self.sent_requests.contains(pos) {
                self.sent_requests.insert(*pos);
                let _ = self.server_tx.send(ClientMessage::RequestChunk { cx: pos.0, cz: pos.1 });
            }
        }

        // Process completed async meshes
        if let Some(renderer) = renderer {
            while let Ok((pos, chunk_mesh_data)) = self.mesh_rx.try_recv() {
                self.meshing_chunks.remove(&pos);
                if expected_chunks.contains(&pos) {
                    if !chunk_mesh_data.indices.is_empty() {
                        let mesh = Mesh {
                            vertex_buffer: renderer.create_vertex_buffer(&chunk_mesh_data.vertices),
                            index_buffer: renderer.create_index_buffer(&chunk_mesh_data.indices),
                            index_count: chunk_mesh_data.indices.len() as u32,
                        };
                        self.chunk_meshes.insert(pos, mesh);
                    } else {
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
        
        let current_lods: Vec<_> = self.lod_meshes.keys().cloned().collect();
        for pos in current_lods {
            if !expected_lods.contains(&pos) {
                self.lod_meshes.remove(&pos);
            }
        }

        // 4. Queue unmeshed chunks for background processing
        if let (Some(renderer), Some(pack)) = (renderer, &self.asset_pack) {
            for &pos in &expected_chunks {
                if !self.chunk_meshes.contains_key(&pos) && !self.meshing_chunks.contains(&pos) {
                    if let Some(chunk) = self.world_manager.chunk_manager.chunks.get(&pos) {
                        self.meshing_chunks.insert(pos);
                        
                        let chunk_clone = chunk.clone();
                        let pack_clone = Arc::clone(pack);
                        let mesh_tx_clone = self.mesh_tx.clone();
                        
                        self.world_manager.task_pool.spawn(move || {
                            let chunk_mesh_data = Mesher::mesh_chunk(&chunk_clone, |block_id, face_idx| {
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

                                if let Some(block_data) = pack_clone.block_dict.get(block_name) {
                                    let face = &block_data.uv_faces[face_idx % block_data.uv_faces.len()];
                                    ([face[0], face[1], face[2], face[3]], color)
                                } else {
                                    ([0.0, 0.0, 0.0, 0.0], color)
                                }
                            });
                            
                            let _ = mesh_tx_clone.send((pos, chunk_mesh_data));
                        });
                    }
                }
            }

            // 4b. Mesh other players' Steve models
            for (id, player) in self.other_players.iter_mut() {
                if player.mesh.is_none() {
                    let model_type = if id % 2 == 0 { crate::steve::PlayerModelType::Steve } else { crate::steve::PlayerModelType::Alex };
                    let (vertices, indices) = crate::steve::build_steve_vertices(player.position, pack, model_type);
                    if !indices.is_empty() {
                        let mesh = Mesh {
                            vertex_buffer: renderer.create_vertex_buffer(&vertices),
                            index_buffer: renderer.create_index_buffer(&indices),
                            index_count: indices.len() as u32,
                        };
                        player.mesh = Some(mesh);
                    }
                }
            }
            
            // 4c. Generate LOD meshes
            let generator = self.world_manager.chunk_manager.generator.clone();
            
            // Get center of stone texture to avoid atlas sampling bleed on LODs
            let mut stone_uv_center = [0.0, 0.0];
            if let Some(pack) = &self.asset_pack {
                if let Some(block_data) = pack.block_dict.get("minecraft:stone") {
                    if !block_data.uv_faces.is_empty() {
                        let face = &block_data.uv_faces[0];
                        // center point: u0 + (u1 - u0)/2, v0 + (v1 - v0)/2
                        stone_uv_center = [(face[0] + face[2]) / 2.0, (face[1] + face[3]) / 2.0];
                    }
                }
            }

            for &pos in &expected_lods {
                if !self.lod_meshes.contains_key(&pos) && !self.meshing_lods.contains(&pos) {
                    self.meshing_lods.insert(pos);
                    let mesh_lod_tx_clone = self.mesh_lod_tx.clone();
                    let gen_clone = generator.clone();
                    self.world_manager.task_pool.spawn(move || {
                        let (level, tx, tz) = pos;
                        let tile_data = LodGenerator::generate_procedural(level, tx, tz, &gen_clone);
                        let mesh_data = LodMesher::mesh_tile(&tile_data, stone_uv_center);
                        let _ = mesh_lod_tx_clone.send((pos, mesh_data));
                    });
                }
            }
        }
    }

    fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::JoinAck { spawn_pos } => {
                info!("Server acknowledged join. Teleporting to spawn: {:?}", spawn_pos);
                if let Ok(mut transform) = self.world_manager.ecs.get::<&mut LocalTransform>(self.local_player_entity) {
                    transform.translation = spawn_pos;
                }
                self.joined = true;
            }
            ServerMessage::ChunkData { cx, cz, chunk } => {
                // Insert the chunk authoritatively sent by the server into local mirror
                self.world_manager.chunk_manager.chunks.insert((cx, cz), chunk);
            }
            ServerMessage::PlayerPosAck { position: _ } => {
                // Pos ack from server - could perform client-side correction here if out of sync
            }
            ServerMessage::UnloadChunk { cx: _, cz: _ } => {
                // Server requested to unload chunk (unused in singleplayer since client dictates render distance)
            }
            ServerMessage::PlayerJoined { id, username, position } => {
                info!("Remote player {} (ID {}) joined at {:?}", username, id, position);
                self.other_players.insert(id, RemotePlayer {
                    username,
                    position,
                    mesh: None,
                });
            }
            ServerMessage::PlayerMoved { id, position } => {
                if let Some(player) = self.other_players.get_mut(&id) {
                    player.position = position;
                    // Reset mesh to force regeneration at the new position
                    player.mesh = None;
                }
            }
            ServerMessage::PlayerLeft { id } => {
                info!("Remote player (ID {}) left.", id);
                self.other_players.remove(&id);
            }
        }
    }
}
