use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod world;
pub mod protocol;
pub mod ecs;

/// The final cooked asset pack loaded by the wgpu client.
#[derive(Serialize, Deserialize, Debug)]
pub struct AssetPack {
    /// Version string or identifier
    pub version: String,
    /// A single large RGBA PNG image bytes representing the texture atlas
    pub atlas_png: Vec<u8>,
    /// The normal map texture atlas
    pub atlas_normal_png: Vec<u8>,
    /// The specular/LabPBR texture atlas (Smoothness, Metallic, Emissive)
    pub atlas_specular_png: Vec<u8>,
    /// The dictionary mapping block namespace names (e.g. "minecraft:stone") to rendering data
    pub block_dict: HashMap<String, BlockRenderData>,
    /// The dictionary mapping raw texture paths to their atlas UV boundaries
    pub texture_dict: HashMap<String, [f32; 4]>,
}

/// The rendering data for a specific block (MVP only stores basic full cube blocks)
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockRenderData {
    /// UV coordinates for all 6 faces. Order: [North, South, East, West, Up, Down]
    /// Each UV is (u0, v0, u1, v1) in 0.0-1.0 space.
    pub uv_faces: [[f32; 4]; 6],
}
