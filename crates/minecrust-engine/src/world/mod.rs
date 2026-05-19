pub mod mesher;
pub mod prefab;
pub mod lod;
pub mod lod_mesher;

pub use minecrust_shared::world::chunk::{Chunk, CHUNK_WIDTH, CHUNK_DEPTH, CHUNK_HEIGHT, CHUNK_VOLUME, MIN_Y, MAX_Y};
pub use minecrust_shared::world::generator::WorldGenerator;
pub use mesher::{Mesher, ChunkMesh};

use std::collections::HashMap;
use crate::core::TaskPool;
use std::sync::Arc;

// World Subsystem skeleton
// This will encapsulate hecs::World and System dispatching
pub struct WorldManager {
    pub ecs: hecs::World,
    pub chunk_manager: ChunkManager,
    pub lod_manager: lod::LodManager,
    pub task_pool: Arc<TaskPool>,
}

impl WorldManager {
    pub fn new(seed: u32) -> Self {
        Self {
            ecs: hecs::World::new(),
            chunk_manager: ChunkManager::new(seed),
            lod_manager: lod::LodManager::new(),
            task_pool: Arc::new(TaskPool::default()),
        }
    }
}

pub struct ChunkManager {
    pub chunks: HashMap<(i32, i32), Chunk>,
    pub generator: WorldGenerator,
}

impl ChunkManager {
    pub fn new(seed: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            generator: WorldGenerator::new(seed),
        }
    }

    /// Retrieves a chunk, generating it synchronously if not present (MVP implementation)
    pub fn get_or_generate(&mut self, chunk_x: i32, chunk_z: i32) -> &Chunk {
        self.chunks.entry((chunk_x, chunk_z)).or_insert_with(|| {
            self.generator.generate_chunk(chunk_x, chunk_z)
        })
    }
}
