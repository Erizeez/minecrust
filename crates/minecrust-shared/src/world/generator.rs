use crate::world::chunk::{Chunk, CHUNK_DEPTH, CHUNK_WIDTH, MIN_Y};
use noise::{NoiseFn, Simplex};

use std::sync::Arc;
use crate::world::block::BlockRegistry;

#[derive(Clone)]
pub struct WorldGenerator {
    simplex: Simplex,
    registry: Arc<BlockRegistry>,
    id_stone: u16,
    id_dirt: u16,
    id_grass: u16,
    id_sand: u16,
    id_water: u16,
    id_bedrock: u16,
    id_oak_log: u16,
    id_oak_leaves: u16,
    id_coal_ore: u16,
    id_iron_ore: u16,
    id_gold_ore: u16,
    id_diamond_ore: u16,
}

impl WorldGenerator {
    pub fn new(seed: u32, registry: Arc<BlockRegistry>) -> Self {
        let id_stone = registry.get_id("minecraft:stone").unwrap_or(0);
        let id_dirt = registry.get_id("minecraft:dirt").unwrap_or(0);
        let id_grass = registry.get_id("minecraft:grass_block").unwrap_or(0);
        let id_sand = registry.get_id("minecraft:sand").unwrap_or(0);
        let id_water = registry.get_id("minecraft:water").unwrap_or(0);
        let id_bedrock = registry.get_id("minecraft:bedrock").unwrap_or(0);
        let id_oak_log = registry.get_id("minecraft:oak_log").unwrap_or(0);
        let id_oak_leaves = registry.get_id("minecraft:oak_leaves").unwrap_or(0);
        let id_coal_ore = registry.get_id("minecraft:coal_ore").unwrap_or(0);
        let id_iron_ore = registry.get_id("minecraft:iron_ore").unwrap_or(0);
        let id_gold_ore = registry.get_id("minecraft:gold_ore").unwrap_or(0);
        let id_diamond_ore = registry.get_id("minecraft:diamond_ore").unwrap_or(0);

        Self {
            simplex: Simplex::new(seed),
            registry,
            id_stone,
            id_dirt,
            id_grass,
            id_sand,
            id_water,
            id_bedrock,
            id_oak_log,
            id_oak_leaves,
            id_coal_ore,
            id_iron_ore,
            id_gold_ore,
            id_diamond_ore,
        }
    }

    pub fn registry(&self) -> &Arc<BlockRegistry> {
        &self.registry
    }
    
    pub fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        let scale = 0.02;
        let noise_val = self.simplex.get([world_x * scale, world_z * scale]);
        (noise_val * 20.0 + 10.0) as i32
    }
    
    pub fn get_surface_block(&self, world_x: f64, world_z: f64) -> (i32, u16) {
        let height = self.get_surface_height(world_x, world_z);
        // Match chunk generation logic: top block is currently always grass (3)
        // If mountain biomes are added later, adjust this to match `generate_chunk`
        (height, 3)
    }

    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_z);
        let sea_level = 0;

        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_DEPTH {
                // Calculate world coordinates
                let world_x = (chunk_x * CHUNK_WIDTH as i32 + x as i32) as f64;
                let world_z = (chunk_z * CHUNK_DEPTH as i32 + z as i32) as f64;

                let height = self.get_surface_height(world_x, world_z);
                let surface_block = if height <= sea_level {
                    self.id_sand
                } else if height <= sea_level + 2 {
                    if self.simplex.get([world_x * 0.1, world_z * 0.1]) > 0.0 {
                        self.id_sand
                    } else {
                        self.id_grass
                    }
                } else {
                    self.id_grass
                };

                let water_level = sea_level.max(height);

                // Fill columns
                for y in MIN_Y..=water_level {
                    if y == MIN_Y {
                        chunk.set_block(x, y, z, self.id_bedrock);
                    } else if y > height {
                        chunk.set_block(x, y, z, self.id_water);
                    } else if y == height {
                        chunk.set_block(x, y, z, surface_block);
                    } else if y > height - 3 {
                        let sub_block = if surface_block == self.id_sand { self.id_sand } else { self.id_dirt };
                        chunk.set_block(x, y, z, sub_block);
                    } else {
                        // Generate Ores in stone
                        let ore_noise = self.simplex.get([world_x * 0.2, y as f64 * 0.2, world_z * 0.2]);
                        let block_id = if ore_noise > 0.85 {
                            if y < -40 { self.id_diamond_ore }
                            else if y < -20 { self.id_gold_ore }
                            else if y < 0 { self.id_iron_ore }
                            else { self.id_coal_ore }
                        } else {
                            self.id_stone
                        };
                        chunk.set_block(x, y, z, block_id);
                    }
                }

                // Simple tree generation on grass
                if surface_block == self.id_grass && height > sea_level {
                    let tree_noise = self.simplex.get([world_x * 1.5, world_z * 1.5]);
                    // 1 in ~50 chance per block, simplified by threshold
                    if tree_noise > 0.95 && x > 2 && x < CHUNK_WIDTH - 2 && z > 2 && z < CHUNK_DEPTH - 2 {
                        let tree_height = 4 + (self.simplex.get([world_x * 5.0, world_z * 5.0]).abs() * 3.0) as i32;
                        
                        // Leaves
                        for ly in (height + tree_height - 2)..=(height + tree_height + 1) {
                            let radius = if ly >= height + tree_height { 1 } else { 2 };
                            for lx in (x as i32 - radius)..=(x as i32 + radius) {
                                for lz in (z as i32 - radius)..=(z as i32 + radius) {
                                    if lx >= 0 && lx < CHUNK_WIDTH as i32 && lz >= 0 && lz < CHUNK_DEPTH as i32 {
                                        // Simple rounding
                                        if lx.abs_diff(x as i32) + lz.abs_diff(z as i32) <= (radius + 1) as u32 {
                                            if chunk.get_block(lx as usize, ly, lz as usize) == 0 {
                                                chunk.set_block(lx as usize, ly, lz as usize, self.id_oak_leaves);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Log trunk
                        for ty in 1..=tree_height {
                            chunk.set_block(x, height + ty, z, self.id_oak_log);
                        }
                    }
                }
            }
        }

        chunk
    }
}
