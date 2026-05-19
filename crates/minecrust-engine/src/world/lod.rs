use std::collections::HashMap;
use crate::world::{Chunk, CHUNK_WIDTH, CHUNK_DEPTH, MAX_Y, MIN_Y};
use glam::Vec3;

pub const LOD_TILE_SIZE: usize = 16; // A tile always contains 16x16 data points, regardless of level

#[derive(Clone)]
pub struct LodTileData {
    pub level: u8,
    pub tile_x: i32,
    pub tile_z: i32,
    // Heights mapped to MIN_Y..MAX_Y, so we can use i16
    pub heights: [i16; LOD_TILE_SIZE * LOD_TILE_SIZE],
    // Simplified RGB color
    pub colors: [[f32; 3]; LOD_TILE_SIZE * LOD_TILE_SIZE],
}

pub struct LodManager {
    // Key: (level, tile_x, tile_z)
    pub tiles: HashMap<(u8, i32, i32), LodTileData>,
}

impl LodManager {
    pub fn new() -> Self {
        Self {
            tiles: HashMap::new(),
        }
    }
    
    pub fn insert_tile(&mut self, tile: LodTileData) {
        self.tiles.insert((tile.level, tile.tile_x, tile.tile_z), tile);
    }
    
    pub fn get_tile(&self, level: u8, tile_x: i32, tile_z: i32) -> Option<&LodTileData> {
        self.tiles.get(&(level, tile_x, tile_z))
    }
}

pub struct LodGenerator;

impl LodGenerator {
    /// Procedurally generates a LodTileData directly from the WorldGenerator without building full chunks.
    pub fn generate_procedural(level: u8, tile_x: i32, tile_z: i32, generator: &crate::world::WorldGenerator) -> LodTileData {
        let mut heights = [MIN_Y as i16; LOD_TILE_SIZE * LOD_TILE_SIZE];
        let mut colors = [[0.0; 3]; LOD_TILE_SIZE * LOD_TILE_SIZE];
        
        let scale = 1 << level;
        let base_x = (tile_x * CHUNK_WIDTH as i32 * scale) as f64;
        let base_z = (tile_z * CHUNK_DEPTH as i32 * scale) as f64;
        
        for z in 0..LOD_TILE_SIZE {
            for x in 0..LOD_TILE_SIZE {
                let world_x = base_x + (x as f64 * scale as f64);
                let world_z = base_z + (z as f64 * scale as f64);
                
                let (h, block_id) = generator.get_surface_block(world_x, world_z);
                
                let idx = z * LOD_TILE_SIZE + x;
                heights[idx] = h as i16;
                
                // Exact dominant colors for block IDs
                colors[idx] = match block_id {
                    1 => [0.5, 0.5, 0.5],       // Stone
                    2 => [0.53, 0.40, 0.28],    // Dirt
                    3 => [0.44, 0.70, 0.33],    // Grass
                    _ => [0.8, 0.0, 0.8],       // Missing Texture (Magenta)
                };
            }
        }
        
        LodTileData {
            level,
            tile_x,
            tile_z,
            heights,
            colors,
        }
    }

    /// Extracts a heightmap from a standard chunk (LOD 0 data).
    /// Used as the base to downsample into higher LOD levels.
    pub fn extract_surface<F>(chunk: &Chunk, data_resolver: F) -> LodTileData
    where F: Fn(u16) -> [f32; 3]
    {
        let mut heights = [MIN_Y as i16; LOD_TILE_SIZE * LOD_TILE_SIZE];
        let mut colors = [[0.0; 3]; LOD_TILE_SIZE * LOD_TILE_SIZE];
        
        for z in 0..CHUNK_DEPTH {
            for x in 0..CHUNK_WIDTH {
                // Find top-most solid block
                let mut top_y = MIN_Y as i16;
                let mut top_color = [0.0; 3];
                for y in (MIN_Y..=MAX_Y).rev() {
                    let block_id = chunk.get_block(x, y, z);
                    if block_id != 0 {
                        top_y = y as i16;
                        top_color = data_resolver(block_id);
                        break;
                    }
                }
                
                let idx = z * CHUNK_WIDTH + x;
                heights[idx] = top_y;
                colors[idx] = top_color;
            }
        }
        
        LodTileData {
            level: 0,
            tile_x: chunk.chunk_x,
            tile_z: chunk.chunk_z,
            heights,
            colors,
        }
    }

    /// Merges 4 lower-level LodTileData into 1 higher-level LodTileData.
    pub fn downsample(level: u8, tile_x: i32, tile_z: i32, bottom_left: &LodTileData, bottom_right: &LodTileData, top_left: &LodTileData, top_right: &LodTileData) -> LodTileData {
        let mut heights = [MIN_Y as i16; LOD_TILE_SIZE * LOD_TILE_SIZE];
        let mut colors = [[0.0; 3]; LOD_TILE_SIZE * LOD_TILE_SIZE];
        
        for z in 0..LOD_TILE_SIZE {
            for x in 0..LOD_TILE_SIZE {
                let dst_idx = z * LOD_TILE_SIZE + x;
                
                let tile = if z < LOD_TILE_SIZE / 2 {
                    if x < LOD_TILE_SIZE / 2 { bottom_left } else { bottom_right }
                } else {
                    if x < LOD_TILE_SIZE / 2 { top_left } else { top_right }
                };
                
                let src_x_base = (x % (LOD_TILE_SIZE / 2)) * 2;
                let src_z_base = (z % (LOD_TILE_SIZE / 2)) * 2;
                
                let mut max_h = MIN_Y as i16;
                let mut avg_color = [0.0; 3];
                let mut valid_samples = 0;
                
                for dz in 0..2 {
                    for dx in 0..2 {
                        let sx = src_x_base + dx;
                        let sz = src_z_base + dz;
                        let src_idx = sz * LOD_TILE_SIZE + sx;
                        let h = tile.heights[src_idx];
                        if h > MIN_Y as i16 {
                            if h > max_h { max_h = h; } // Take max height to preserve silhouettes
                            let c = tile.colors[src_idx];
                            avg_color[0] += c[0];
                            avg_color[1] += c[1];
                            avg_color[2] += c[2];
                            valid_samples += 1;
                        }
                    }
                }
                
                if valid_samples > 0 {
                    avg_color[0] /= valid_samples as f32;
                    avg_color[1] /= valid_samples as f32;
                    avg_color[2] /= valid_samples as f32;
                }
                
                heights[dst_idx] = max_h;
                colors[dst_idx] = avg_color;
            }
        }
        
        LodTileData {
            level,
            tile_x,
            tile_z,
            heights,
            colors,
        }
    }
}
