use minecrust_engine::input::InputManager;
use minecrust_engine::world::{player::PlayerController, Mesher, WorldManager};
use minecrust_engine::Renderer;
use minecrust_shared::AssetPack;
use std::collections::{HashMap, HashSet};
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
    pub player: PlayerController,
    pub input_manager: InputManager,
    pub chunk_meshes: HashMap<(i32, i32), Mesh>,
    pub asset_pack: Option<AssetPack>,
    pub other_players: HashMap<u32, RemotePlayer>,
    
    // C/S Channels
    server_tx: Sender<ClientMessage>,
    server_rx: Receiver<ServerMessage>,
    sent_requests: HashSet<(i32, i32)>,
    joined: bool,
}

impl GameSession {
    pub fn new(server_tx: Sender<ClientMessage>, server_rx: Receiver<ServerMessage>) -> Self {
        let mut session = Self {
            world_manager: WorldManager::new(12345),
            player: PlayerController::new(glam::Vec3::new(8.0, 60.0, 8.0)),
            input_manager: InputManager::new(),
            chunk_meshes: HashMap::new(),
            asset_pack: None,
            other_players: HashMap::new(),
            server_tx,
            server_rx,
            sent_requests: HashSet::new(),
            joined: false,
        };

        // Send a join request immediately on session startup
        if let Err(e) = session.server_tx.send(ClientMessage::Join { username: "Player".to_string() }) {
            warn!("Failed to send Join packet to integrated server: {:?}", e);
        }

        session
    }

    pub fn update(&mut self, dt: f64, time: f64, render_distance: i32, renderer: Option<&Renderer>) {
        // 1. Process all incoming Server Messages (non-blocking)
        while let Ok(msg) = self.server_rx.try_recv() {
            self.handle_server_message(msg);
        }

        // 2. Perform local client player movement
        self.player.update(&mut self.input_manager, &mut self.world_manager, dt, time);

        // Send movement update to server (could rate limit this, but local channel is fine)
        let _ = self.server_tx.send(ClientMessage::PlayerMove {
            x: self.player.position.x,
            y: self.player.position.y,
            z: self.player.position.z,
            yaw: 0.0,
            pitch: 0.0,
        });

        // 3. Update chunk load subscriptions
        let player_cx = (self.player.position.x / minecrust_engine::world::CHUNK_WIDTH as f32).floor() as i32;
        let player_cz = (self.player.position.z / minecrust_engine::world::CHUNK_DEPTH as f32).floor() as i32;

        let mut expected_chunks = HashSet::new();
        for cx in (player_cx - render_distance)..=(player_cx + render_distance) {
            for cz in (player_cz - render_distance)..=(player_cz + render_distance) {
                expected_chunks.insert((cx, cz));
            }
        }

        // Unload old chunk meshes and local chunks
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

        // 4. Mesh newly loaded chunks that have been sent by the server
        if let (Some(renderer), Some(pack)) = (renderer, &self.asset_pack) {
            for pos in expected_chunks {
                if !self.chunk_meshes.contains_key(&pos) {
                    // Check if chunk is loaded locally (sent from server)
                    if let Some(chunk) = self.world_manager.chunk_manager.chunks.get(&pos) {
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

            // 4b. Mesh other players' Steve models
            for player in self.other_players.values_mut() {
                if player.mesh.is_none() {
                    let (vertices, indices) = crate::steve::build_steve_vertices(player.position, pack);
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
        }
    }

    fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::JoinAck { spawn_pos } => {
                info!("Server acknowledged join. Teleporting to spawn: {:?}", spawn_pos);
                self.player.position = spawn_pos;
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
