use serde::{Serialize, Deserialize};

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_DEPTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 384;
pub const MIN_Y: i32 = -64;
pub const MAX_Y: i32 = 319; // inclusive

/// Linear size of a chunk in blocks.
pub const CHUNK_VOLUME: usize = CHUNK_WIDTH * CHUNK_HEIGHT * CHUNK_DEPTH;

use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct Chunk {
    // 16 * 384 * 16 = 98304 blocks
    // 0 = air, other IDs map to block types.
    pub blocks: Box<[u16]>,
    // The chunk's X and Z coordinates in chunk space (not block space)
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("chunk_x", &self.chunk_x)
            .field("chunk_z", &self.chunk_z)
            .field("blocks_len", &self.blocks.len())
            .finish()
    }
}

impl Chunk {
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            blocks: vec![0; CHUNK_VOLUME].into_boxed_slice(),
            chunk_x,
            chunk_z,
        }
    }

    /// Convert 3D local coordinates to 1D array index.
    /// x: 0..15, y: -64..319, z: 0..15
    #[inline]
    pub fn get_index(x: usize, y: i32, z: usize) -> Option<usize> {
        if x >= CHUNK_WIDTH || z >= CHUNK_DEPTH || y < MIN_Y || y > MAX_Y {
            return None;
        }
        let local_y = (y - MIN_Y) as usize;
        // YZX ordering is often better for greedy meshing cache locality (iterating X, then Z, then Y)
        let idx = (local_y * CHUNK_WIDTH * CHUNK_DEPTH) + (z * CHUNK_WIDTH) + x;
        Some(idx)
    }

    #[inline]
    pub fn get_block(&self, x: usize, y: i32, z: usize) -> u16 {
        if let Some(idx) = Self::get_index(x, y, z) {
            self.blocks[idx]
        } else {
            0 // Air
        }
    }

    #[inline]
    pub fn set_block(&mut self, x: usize, y: i32, z: usize, block_id: u16) {
        if let Some(idx) = Self::get_index(x, y, z) {
            self.blocks[idx] = block_id;
        }
    }
}
