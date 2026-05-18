use crate::renderer::Vertex;
use crate::world::{Chunk, CHUNK_DEPTH, CHUNK_HEIGHT, CHUNK_WIDTH, MAX_Y, MIN_Y};
use glam::Vec3;

pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// A simple surface culling mesher.
pub struct Mesher;

impl Mesher {
    /// Generates a mesh for a chunk.
    /// `uv_resolver` takes a `block_id` and a `face_idx` (0=North, 1=South, 2=East, 3=West, 4=Up, 5=Down)
    /// and returns the (u0, v0, u1, v1) coordinates.
    pub fn mesh_chunk<F>(chunk: &Chunk, data_resolver: F) -> ChunkMesh
    where
        F: Fn(u16, usize) -> ([f32; 4], [f32; 3]),
    {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let base_x = (chunk.chunk_x * CHUNK_WIDTH as i32) as f32;
        let base_z = (chunk.chunk_z * CHUNK_DEPTH as i32) as f32;

        for y in MIN_Y..=MAX_Y {
            for z in 0..CHUNK_DEPTH {
                for x in 0..CHUNK_WIDTH {
                    let block_id = chunk.get_block(x, y, z);
                    if block_id == 0 {
                        continue; // Air
                    }

                    // Check 6 faces
                    // Coordinates relative to chunk
                    let pos = Vec3::new(base_x + x as f32, y as f32, base_z + z as f32);

                    // North (-Z)
                    if z == 0 || chunk.get_block(x, y, z - 1) == 0 {
                        let (uvs, color) = data_resolver(block_id, 0);
                        Self::add_face(&mut vertices, &mut indices, pos, Vec3::NEG_Z, uvs, color);
                    }
                    // South (+Z)
                    if z == CHUNK_DEPTH - 1 || chunk.get_block(x, y, z + 1) == 0 {
                        let (uvs, color) = data_resolver(block_id, 1);
                        Self::add_face(&mut vertices, &mut indices, pos, Vec3::Z, uvs, color);
                    }
                    // East (+X)
                    if x == CHUNK_WIDTH - 1 || chunk.get_block(x + 1, y, z) == 0 {
                        let (uvs, color) = data_resolver(block_id, 2);
                        Self::add_face(&mut vertices, &mut indices, pos, Vec3::X, uvs, color);
                    }
                    // West (-X)
                    if x == 0 || chunk.get_block(x - 1, y, z) == 0 {
                        let (uvs, color) = data_resolver(block_id, 3);
                        Self::add_face(&mut vertices, &mut indices, pos, Vec3::NEG_X, uvs, color);
                    }
                    // Up (+Y)
                    if y == MAX_Y || chunk.get_block(x, y + 1, z) == 0 {
                        let (uvs, color) = data_resolver(block_id, 4);
                        Self::add_face(&mut vertices, &mut indices, pos, Vec3::Y, uvs, color);
                    }
                    // Down (-Y)
                    if y == MIN_Y || chunk.get_block(x, y - 1, z) == 0 {
                        let (uvs, color) = data_resolver(block_id, 5);
                        Self::add_face(&mut vertices, &mut indices, pos, Vec3::NEG_Y, uvs, color);
                    }
                }
            }
        }

        ChunkMesh { vertices, indices }
    }

    fn add_face(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, pos: Vec3, normal: Vec3, uvs: [f32; 4], color: [f32; 3]) {
        let (u0, v0, u1, v1) = (uvs[0], uvs[1], uvs[2], uvs[3]);
        let base_idx = vertices.len() as u32;

        let (tangent, bitangent) = if normal.y.abs() > 0.5 {
            (Vec3::X, Vec3::Z * -normal.y) // Up/Down
        } else {
            (Vec3::Y.cross(normal), Vec3::Y) // Sides
        };

        let v0_pos = pos + normal * 0.5 - tangent * 0.5 - bitangent * 0.5;
        let v1_pos = pos + normal * 0.5 + tangent * 0.5 - bitangent * 0.5;
        let v2_pos = pos + normal * 0.5 + tangent * 0.5 + bitangent * 0.5;
        let v3_pos = pos + normal * 0.5 - tangent * 0.5 + bitangent * 0.5;

        vertices.push(Vertex { position: v0_pos.into(), uv: [u0, v1], color });
        vertices.push(Vertex { position: v1_pos.into(), uv: [u1, v1], color });
        vertices.push(Vertex { position: v2_pos.into(), uv: [u1, v0], color });
        vertices.push(Vertex { position: v3_pos.into(), uv: [u0, v0], color });

        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);
    }
}
