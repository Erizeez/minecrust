use serde::{Deserialize, Serialize};
use crate::world::chunk::Chunk;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Join {
        username: String,
    },
    RequestChunk {
        cx: i32,
        cz: i32,
    },
    PlayerMove {
        x: f32,
        y: f32,
        z: f32,
        yaw: f32,
        pitch: f32,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    JoinAck {
        spawn_pos: glam::Vec3,
    },
    ChunkData {
        cx: i32,
        cz: i32,
        chunk: std::sync::Arc<Chunk>,
    },
    PlayerPosAck {
        position: glam::Vec3,
    },
    UnloadChunk {
        cx: i32,
        cz: i32,
    },
    PlayerJoined {
        id: u32,
        username: String,
        position: glam::Vec3,
    },
    PlayerMoved {
        id: u32,
        position: glam::Vec3,
    },
    PlayerLeft {
        id: u32,
    },
    TimeUpdate {
        time: u32,
    },
}
