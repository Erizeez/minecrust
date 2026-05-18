use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use log::{info, error};
use crossbeam_channel::{Sender, Receiver};
use minecrust_shared::protocol::{ClientMessage, ServerMessage};

#[derive(Debug, Clone)]
pub struct LanServer {
    pub motd: String,
    pub address: String,
    pub last_seen: Instant,
}

#[derive(Clone)]
pub struct LanServerDiscoverer {
    servers: Arc<Mutex<Vec<LanServer>>>,
}

impl LanServerDiscoverer {
    pub fn new() -> Self {
        let servers = Arc::new(Mutex::new(Vec::<LanServer>::new()));
        let servers_clone = Arc::clone(&servers);

        thread::Builder::new()
            .name("LanDiscoverer".to_string())
            .spawn(move || {
                let socket = match UdpSocket::bind("0.0.0.0:44452") {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to bind LAN Discoverer UDP socket to 44452: {:?}", e);
                        return;
                    }
                };
                
                // Join multicast group
                if let Err(e) = socket.join_multicast_v4(
                    &"224.0.2.60".parse().unwrap(),
                    &"0.0.0.0".parse().unwrap(),
                ) {
                    info!("Could not join UDP multicast group (expected if no multirouting): {:?}", e);
                }
                
                socket.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
                let mut buf = [0u8; 1024];

                info!("LAN Server Discoverer listening on port 44452...");

                loop {
                    // Cleanup servers older than 5 seconds
                    {
                        if let Ok(mut list) = servers_clone.lock() {
                            list.retain(|s| s.last_seen.elapsed() < Duration::from_secs(5));
                        }
                    }

                    if let Ok((amt, _src)) = socket.recv_from(&mut buf) {
                        let payload = String::from_utf8_lossy(&buf[..amt]);
                        if payload.contains("[MOTD]") && payload.contains("[/MOTD]") && payload.contains("[AD]") && payload.contains("[/AD]") {
                            // Extract MOTD
                            let motd_start = payload.find("[MOTD]").unwrap() + 6;
                            let motd_end = payload.find("[/MOTD]").unwrap();
                            let motd = payload[motd_start..motd_end].to_string();

                            // Extract Address
                            let addr_start = payload.find("[AD]").unwrap() + 4;
                            let addr_end = payload.find("[/AD]").unwrap();
                            let address = payload[addr_start..addr_end].to_string();

                            let mut list = servers_clone.lock().unwrap();
                            if let Some(srv) = list.iter_mut().find(|s| s.address == address) {
                                srv.motd = motd;
                                srv.last_seen = Instant::now();
                            } else {
                                info!("Discovered LAN Server: {} ({})", motd, address);
                                list.push(LanServer {
                                    motd,
                                    address,
                                    last_seen: Instant::now(),
                                });
                            }
                        }
                    }
                    
                    thread::sleep(Duration::from_millis(100));
                }
            })
            .expect("Failed to spawn LanDiscoverer thread");

        Self { servers }
    }

    pub fn get_servers(&self) -> Vec<LanServer> {
        if let Ok(list) = self.servers.lock() {
            list.clone()
        } else {
            Vec::new()
        }
    }
}

pub fn connect_multiplayer(
    server_addr: SocketAddr,
    username: String,
) -> Result<(Sender<ClientMessage>, Receiver<ServerMessage>), anyhow::Error> {
    use laminar::{Socket, SocketEvent, Packet};
    
    info!("Connecting to multiplayer server at {:?}", server_addr);
    let mut socket = Socket::bind("0.0.0.0:0")?;
    let packet_sender = socket.get_packet_sender();
    let event_receiver = socket.get_event_receiver();
    let _polling_handle = socket.start_polling();

    let (client_tx, server_rx) = crossbeam_channel::unbounded::<ClientMessage>();
    let (server_tx, client_rx) = crossbeam_channel::unbounded::<ServerMessage>();

    // Background sender thread
    let packet_sender_clone = packet_sender.clone();
    thread::Builder::new()
        .name("LaminarSender".to_string())
        .spawn(move || {
            while let Ok(msg) = server_rx.recv() {
                let payload = bincode::serialize(&msg).unwrap();
                let packet = match msg {
                    ClientMessage::PlayerMove { .. } => {
                        Packet::unreliable_sequenced(server_addr, payload, Some(1))
                    }
                    _ => {
                        Packet::reliable_ordered(server_addr, payload, Some(0))
                    }
                };
                let _ = packet_sender_clone.send(packet);
            }
        })
        .unwrap();

    // Background receiver thread
    thread::Builder::new()
        .name("LaminarReceiver".to_string())
        .spawn(move || {
            while let Ok(event) = event_receiver.recv() {
                match event {
                    SocketEvent::Packet(packet) => {
                        if let Ok(msg) = bincode::deserialize::<ServerMessage>(packet.payload()) {
                            let _ = server_tx.send(msg);
                        }
                    }
                    SocketEvent::Timeout(_) => {
                        info!("Laminar connection timed out.");
                        break;
                    }
                    _ => {}
                }
            }
        })
        .unwrap();

    Ok((client_tx, client_rx))
}
