use crate::renderer::Vertex;
use crate::world::mesher::ChunkMesh;
use crate::world::lod::{LodTileData, LOD_TILE_SIZE};
use crate::world::{CHUNK_WIDTH, CHUNK_DEPTH, MIN_Y};
use glam::Vec3;

pub struct LodMesher;

impl LodMesher {
    pub fn mesh_tile(tile: &LodTileData, uv_center: [f32; 2]) -> ChunkMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let scale = 1 << tile.level;
        let base_x = (tile.tile_x * CHUNK_WIDTH as i32 * scale) as f32;
        let base_z = (tile.tile_z * CHUNK_DEPTH as i32 * scale) as f32;
        let quad_size = scale as f32;
        
        let get_h = |x: i32, z: i32| -> i16 {
            if x < 0 || x >= LOD_TILE_SIZE as i32 || z < 0 || z >= LOD_TILE_SIZE as i32 {
                // Skirt depth at chunk boundary
                MIN_Y as i16
            } else {
                tile.heights[(z as usize) * LOD_TILE_SIZE + (x as usize)]
            }
        };

        for z in 0..LOD_TILE_SIZE as i32 {
            for x in 0..LOD_TILE_SIZE as i32 {
                let h = get_h(x, z);
                if h <= MIN_Y as i16 { continue; }
                
                let px = base_x + (x as f32) * quad_size;
                let pz = base_z + (z as f32) * quad_size;
                let py = h as f32;
                
                let mut color = tile.colors[(z as usize) * LOD_TILE_SIZE + (x as usize)];
                
                let h_nz = get_h(x, z - 1);
                let h_pz = get_h(x, z + 1);
                let h_nx = get_h(x - 1, z);
                let h_px = get_h(x + 1, z);
                
                // Fake Lighting / Slope Shading
                let mut light_factor: f32 = 1.0;
                
                // Sun from South-East
                if h > h_pz { light_factor += 0.1; } else if h < h_pz { light_factor -= 0.1; } // South
                if h > h_nz { light_factor -= 0.1; } else if h < h_nz { light_factor += 0.1; } // North
                if h > h_px { light_factor += 0.05; } else if h < h_px { light_factor -= 0.05; } // East
                if h > h_nx { light_factor -= 0.05; } else if h < h_nx { light_factor += 0.05; } // West
                
                light_factor = light_factor.clamp(0.7, 1.2);
                color[0] *= light_factor;
                color[1] *= light_factor;
                color[2] *= light_factor;
                
                // Top Face (+Y)
                Self::add_quad(
                    &mut vertices, &mut indices,
                    [px, py, pz + quad_size],
                    [px + quad_size, py, pz + quad_size],
                    [px + quad_size, py, pz],
                    [px, py, pz],
                    [color[0], color[1], color[2], 1.0],
                    uv_center,
                );
                
                // West Face (-X)
                if h > h_nx {
                    Self::add_quad(
                        &mut vertices, &mut indices,
                        [px, py, pz],
                        [px, py, pz + quad_size],
                        [px, h_nx as f32, pz + quad_size],
                        [px, h_nx as f32, pz],
                        [color[0] * 0.7, color[1] * 0.7, color[2] * 0.7, 1.0], // Fake lighting
                        uv_center,
                    );
                }
                
                // East Face (+X)
                if h > h_px {
                    Self::add_quad(
                        &mut vertices, &mut indices,
                        [px + quad_size, py, pz + quad_size],
                        [px + quad_size, py, pz],
                        [px + quad_size, h_px as f32, pz],
                        [px + quad_size, h_px as f32, pz + quad_size],
                        [color[0] * 0.9, color[1] * 0.9, color[2] * 0.9, 1.0],
                        uv_center,
                    );
                }
                
                // North Face (-Z)
                if h > h_nz {
                    Self::add_quad(
                        &mut vertices, &mut indices,
                        [px + quad_size, py, pz],
                        [px, py, pz],
                        [px, h_nz as f32, pz],
                        [px + quad_size, h_nz as f32, pz],
                        [color[0] * 0.6, color[1] * 0.6, color[2] * 0.6, 1.0],
                        uv_center,
                    );
                }
                
                // South Face (+Z)
                if h > h_pz {
                    Self::add_quad(
                        &mut vertices, &mut indices,
                        [px, py, pz + quad_size],
                        [px + quad_size, py, pz + quad_size],
                        [px + quad_size, h_pz as f32, pz + quad_size],
                        [px, h_pz as f32, pz + quad_size],
                        [color[0] * 1.0, color[1] * 1.0, color[2] * 1.0, 1.0],
                        uv_center,
                    );
                }
            }
        }
        
        ChunkMesh { vertices, indices }
    }

    fn add_quad(
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        v3: [f32; 3],
        color: [f32; 4],
        uv_center: [f32; 2],
    ) {
        let base_idx = vertices.len() as u32;
        vertices.push(Vertex { position: v0, uv: uv_center, color });
        vertices.push(Vertex { position: v1, uv: uv_center, color });
        vertices.push(Vertex { position: v2, uv: uv_center, color });
        vertices.push(Vertex { position: v3, uv: uv_center, color });
        
        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);
    }
}
