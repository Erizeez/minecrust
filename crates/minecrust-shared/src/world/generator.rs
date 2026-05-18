use crate::world::chunk::{Chunk, CHUNK_DEPTH, CHUNK_WIDTH, MIN_Y};
use noise::{NoiseFn, Simplex};

pub struct WorldGenerator {
    simplex: Simplex,
}

impl WorldGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            simplex: Simplex::new(seed),
        }
    }

    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_z);

        // Minecraft standard dirt/stone IDs (mock for now, should map from asset dictionary)
        let id_stone = 1;
        let id_dirt = 2;
        let id_grass = 3;

        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_DEPTH {
                // Calculate world coordinates
                let world_x = (chunk_x * CHUNK_WIDTH as i32 + x as i32) as f64;
                let world_z = (chunk_z * CHUNK_DEPTH as i32 + z as i32) as f64;

                // Scale for noise
                let scale = 0.02;
                let noise_val = self.simplex.get([world_x * scale, world_z * scale]);
                
                // Map noise [-1, 1] to height [-10, 30]
                let height = (noise_val * 20.0 + 10.0) as i32;

                // Fill columns
                for y in MIN_Y..=height {
                    let block_id = if y == height {
                        id_grass
                    } else if y > height - 3 {
                        id_dirt
                    } else {
                        id_stone
                    };
                    chunk.set_block(x, y, z, block_id);
                }
            }
        }

        chunk
    }
}
