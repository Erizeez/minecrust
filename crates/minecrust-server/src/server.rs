use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::{Receiver, Sender};
use log::{info, error};
use laminar::{Socket, SocketEvent, Packet};

use minecrust_shared::protocol::{ClientMessage, ServerMessage};
use minecrust_shared::world::generator::WorldGenerator;
use minecrust_engine::world::WorldManager;

pub struct IntegratedServer {
    world_manager: WorldManager,
    generator: Arc<WorldGenerator>,
    
    // Singleplayer Mode
    rx: Option<Receiver<ClientMessage>>,
    tx: Option<Sender<ServerMessage>>,
    
    // Multiplayer (laminar) Mode
    laminar_sender: Option<Sender<Packet>>,
    laminar_receiver: Option<Receiver<SocketEvent>>,
    clients: HashMap<SocketAddr, u32>,          // maps address to player Entity ID
    client_usernames: HashMap<SocketAddr, String>, // maps address to username
    next_entity_id: u32,
    
    requested_chunks: HashSet<(i32, i32)>,
    
    world_time: u32,
}

impl IntegratedServer {
    /// Start the integrated server in a background thread.
    /// Returns the MPSC channels (used in Singleplayer mode).
    pub fn start(
        seed: u32,
        bind_addr: Option<SocketAddr>,
        registry: Arc<minecrust_shared::world::block::BlockRegistry>,
    ) -> (Sender<ClientMessage>, Receiver<ServerMessage>) {
        let (client_tx, server_rx) = crossbeam_channel::unbounded::<ClientMessage>();
        let (server_tx, client_rx) = crossbeam_channel::unbounded::<ServerMessage>();

        thread::Builder::new()
            .name("IntegratedServer".to_string())
            .spawn(move || {
                let mut laminar_sender = None;
                let mut laminar_receiver = None;
                if let Some(addr) = bind_addr {
                    info!("Binding laminar server to {:?}", addr);
                    match Socket::bind(addr) {
                        Ok(mut socket) => {
                            let bound_addr = socket.local_addr().unwrap_or(addr);
                            let bound_port = bound_addr.port();
                            
                            laminar_sender = Some(socket.get_packet_sender());
                            laminar_receiver = Some(socket.get_event_receiver());
                            
                            thread::spawn(move || {
                                socket.start_polling();
                            });
                            
                            // Start LAN Discovery broadcaster thread
                            thread::spawn(move || {
                                let udp = match std::net::UdpSocket::bind("0.0.0.0:0") {
                                    Ok(s) => s,
                                    Err(e) => {
                                        error!("Failed to bind LAN UDP broadcast socket: {:?}", e);
                                        return;
                                    }
                                };
                                let _ = udp.set_broadcast(true);
                                
                                // Fetch local IP dynamically
                                let local_ip = if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
                                    if s.connect("8.8.8.8:80").is_ok() {
                                        if let Ok(addr) = s.local_addr() {
                                            addr.ip().to_string()
                                        } else {
                                            "127.0.0.1".to_string()
                                        }
                                    } else {
                                        "127.0.0.1".to_string()
                                    }
                                } else {
                                    "127.0.0.1".to_string()
                                };
                                
                                let message = format!("[MOTD]Minecrust LAN World[/MOTD][AD]{}:{}[/AD]", local_ip, bound_port);
                                info!("LAN Discovery Broadcaster active! Announcing {}:{}", local_ip, bound_port);
                                loop {
                                    info!("Broadcasting LAN Discovery to 255.255.255.255:44452 and 224.0.2.60:44452");
                                    let _ = udp.send_to(message.as_bytes(), "255.255.255.255:44452");
                                    let _ = udp.send_to(message.as_bytes(), "224.0.2.60:44452");
                                    thread::sleep(Duration::from_millis(1500));
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to bind laminar server socket: {:?}", e);
                        }
                    }
                }

                let mut server = Self {
                    world_manager: WorldManager::new(seed, Arc::clone(&registry)),
                    generator: Arc::new(WorldGenerator::new(seed, registry)),
                    rx: if laminar_sender.is_none() { Some(server_rx) } else { None },
                    tx: if laminar_sender.is_none() { Some(server_tx) } else { None },
                    laminar_sender,
                    laminar_receiver,
                    clients: HashMap::new(),
                    client_usernames: HashMap::new(),
                    next_entity_id: 1,
                    requested_chunks: HashSet::new(),
                    world_time: 0,
                };
                server.run_loop();
            })
            .expect("Failed to spawn IntegratedServer thread");

        (client_tx, client_rx)
    }

    fn run_loop(&mut self) {
        info!("Integrated Server event loop running.");
        
        let tick_duration = Duration::from_millis(50); // 20 Ticks/sec (50ms per tick)
        let mut last_tick = Instant::now();

        loop {
            let now = Instant::now();
            let elapsed = now.duration_since(last_tick);

            // 1. Process Singleplayer MPSC events
            let mut msgs = Vec::new();
            if let Some(ref rx) = self.rx {
                while let Ok(msg) = rx.try_recv() {
                    msgs.push(msg);
                }
            }
            for msg in msgs {
                self.handle_message_singleplayer(msg);
            }

            // 2. Process Multiplayer laminar socket events
            if let Some(event_receiver) = self.laminar_receiver.clone() {
                while let Ok(event) = event_receiver.try_recv() {
                    match event {
                        SocketEvent::Packet(packet) => {
                            let addr = packet.addr();
                            if let Ok(msg) = bincode::deserialize::<ClientMessage>(packet.payload()) {
                                self.handle_message_multiplayer(addr, msg);
                            }
                        }
                        SocketEvent::Connect(addr) => {
                            info!("Laminar client connected: {:?}", addr);
                        }
                        SocketEvent::Timeout(addr) => {
                            info!("Laminar client timed out: {:?}", addr);
                            self.handle_disconnect(addr);
                        }
                        _ => {}
                    }
                }
            }

            // 3. Server Tick update (20 Ticks/sec)
            if elapsed >= tick_duration {
                last_tick = now;
                self.world_time = (self.world_time + 1) % 24000;
                
                // Broadcast time update every 20 ticks (1 second)
                if self.world_time % 20 == 0 {
                    let msg = ServerMessage::TimeUpdate { time: self.world_time };
                    if let Some(ref tx) = self.tx {
                        let _ = tx.send(msg.clone());
                    }
                    self.broadcast(msg, 0, None);
                }
            }

            // Yield CPU
            thread::sleep(Duration::from_millis(5));
        }
    }

    // --- Singleplayer MPSC Handlers ---
    fn handle_message_singleplayer(&mut self, msg: ClientMessage) {
        match msg {
            ClientMessage::Join { username } => {
                info!("Player {} joined the singleplayer server.", username);
                let spawn_pos = glam::Vec3::new(8.0, 60.0, 8.0);
                if let Some(ref tx) = self.tx {
                    let _ = tx.send(ServerMessage::JoinAck { spawn_pos });
                }
            }
            ClientMessage::RequestChunk { cx, cz } => {
                if self.requested_chunks.insert((cx, cz)) {
                    let generator = Arc::clone(&self.generator);
                    if let Some(ref tx) = self.tx {
                        let tx = tx.clone();
                        thread::spawn(move || {
                            let chunk = generator.generate_chunk(cx, cz);
                            let _ = tx.send(ServerMessage::ChunkData { cx, cz, chunk });
                        });
                    }
                }
            }
            ClientMessage::PlayerMove { x, y, z, yaw: _, pitch: _ } => {
                let player_pos = glam::Vec3::new(x, y, z);
                if let Some(ref tx) = self.tx {
                    let _ = tx.send(ServerMessage::PlayerPosAck { position: player_pos });
                }
            }
        }
    }

    // --- Multiplayer laminar Handlers ---
    fn handle_message_multiplayer(&mut self, addr: SocketAddr, msg: ClientMessage) {
        match msg {
            ClientMessage::Join { username } => {
                let player_id = self.next_entity_id;
                self.next_entity_id += 1;
                
                info!("Player {} joined the server from {:?} with ID {}.", username, addr, player_id);
                self.clients.insert(addr, player_id);
                self.client_usernames.insert(addr, username.clone());
                
                let spawn_pos = glam::Vec3::new(8.0, 60.0, 8.0);
                
                // 1. Send JoinAck to the joined player
                self.send_to_client(addr, ServerMessage::JoinAck { spawn_pos }, 0);
                
                // 2. Broadcast PlayerJoined to other players
                self.broadcast(
                    ServerMessage::PlayerJoined {
                        id: player_id,
                        username: username.clone(),
                        position: spawn_pos,
                    },
                    0,
                    Some(addr),
                );
                
                // 3. Notify the newly joined player about existing players
                let other_players_info: Vec<(u32, String)> = self.clients.iter()
                    .filter(|(other_addr, _)| **other_addr != addr)
                    .filter_map(|(other_addr, &other_id)| {
                        self.client_usernames.get(other_addr).map(|name| (other_id, name.clone()))
                    })
                    .collect();
                
                for (other_id, other_name) in other_players_info {
                    self.send_to_client(
                        addr,
                        ServerMessage::PlayerJoined {
                            id: other_id,
                            username: other_name,
                            position: spawn_pos,
                        },
                        0,
                    );
                }
            }
            ClientMessage::RequestChunk { cx, cz } => {
                // For safety, per-connection chunk requesting
                let generator = Arc::clone(&self.generator);
                let socket_sender = self.laminar_sender.as_ref().unwrap().clone();
                
                thread::spawn(move || {
                    let chunk = generator.generate_chunk(cx, cz);
                    let msg = ServerMessage::ChunkData { cx, cz, chunk };
                    let payload = bincode::serialize(&msg).unwrap();
                    let packet = Packet::reliable_ordered(addr, payload, Some(0));
                    let _ = socket_sender.send(packet);
                });
            }
            ClientMessage::PlayerMove { x, y, z, yaw: _, pitch: _ } => {
                if let Some(&player_id) = self.clients.get(&addr) {
                    let player_pos = glam::Vec3::new(x, y, z);
                    
                    // Acknowledge position to the sender (optional, but good for heartbeat)
                    self.send_to_client(addr, ServerMessage::PlayerPosAck { position: player_pos }, 1);
                    
                    // Broadcast movement to all other players over channel 1 (unreliable-sequenced)
                    self.broadcast(
                        ServerMessage::PlayerMoved {
                            id: player_id,
                            position: player_pos,
                        },
                        1,
                        Some(addr),
                    );
                }
            }
        }
    }

    fn handle_disconnect(&mut self, addr: SocketAddr) {
        if let Some(player_id) = self.clients.remove(&addr) {
            let username = self.client_usernames.remove(&addr).unwrap_or_default();
            info!("Player {} (ID {}) disconnected.", username, player_id);
            
            // Broadcast player departure
            self.broadcast(
                ServerMessage::PlayerLeft { id: player_id },
                0,
                None,
            );
        }
    }

    // --- Helper Network Senders ---
    fn send_to_client(&mut self, addr: SocketAddr, msg: ServerMessage, channel_id: u8) {
        if let Some(ref sender) = self.laminar_sender {
            let payload = bincode::serialize(&msg).unwrap();
            let packet = if channel_id == 0 {
                Packet::reliable_ordered(addr, payload, Some(0))
            } else {
                Packet::unreliable_sequenced(addr, payload, Some(1))
            };
            let _ = sender.send(packet);
        }
    }

    fn broadcast(&mut self, msg: ServerMessage, channel_id: u8, exclude: Option<SocketAddr>) {
        let targets: Vec<SocketAddr> = self.clients.keys().cloned().collect();
        for addr in targets {
            if Some(addr) != exclude {
                self.send_to_client(addr, msg.clone(), channel_id);
            }
        }
    }
}
