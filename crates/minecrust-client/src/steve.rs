use minecrust_engine::renderer::Vertex;
use minecrust_shared::AssetPack;

pub fn build_steve_vertices(
    position: glam::Vec3,
    pack: &AssetPack,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Fetch UV coordinates from asset pack block dictionary
    let get_block_uvs = |block_name: &str| -> [[f32; 4]; 6] {
        let mut uvs = [[0.0, 0.0, 0.0, 0.0]; 6];
        if let Some(block_data) = pack.block_dict.get(block_name) {
            for i in 0..6 {
                let face = &block_data.uv_faces[i % block_data.uv_faces.len()];
                uvs[i] = [face[0], face[1], face[2], face[3]];
            }
        }
        uvs
    };

    let head_uvs = get_block_uvs("minecraft:oak_planks");
    let torso_uvs = get_block_uvs("minecraft:lapis_block");
    let limb_uvs = get_block_uvs("minecraft:dirt");

    let c = position; // local center (base coordinates)

    // 1. Right Leg
    add_cuboid(
        &mut vertices,
        &mut indices,
        c + glam::Vec3::new(-0.25, 0.0, -0.125),
        c + glam::Vec3::new(0.0, 0.75, 0.125),
        &limb_uvs,
        [1.0, 1.0, 1.0],
    );

    // 2. Left Leg
    add_cuboid(
        &mut vertices,
        &mut indices,
        c + glam::Vec3::new(0.0, 0.0, -0.125),
        c + glam::Vec3::new(0.25, 0.75, 0.125),
        &limb_uvs,
        [1.0, 1.0, 1.0],
    );

    // 3. Torso
    add_cuboid(
        &mut vertices,
        &mut indices,
        c + glam::Vec3::new(-0.25, 0.75, -0.125),
        c + glam::Vec3::new(0.25, 1.5, 0.125),
        &torso_uvs,
        [1.0, 1.0, 1.0],
    );

    // 4. Right Arm
    add_cuboid(
        &mut vertices,
        &mut indices,
        c + glam::Vec3::new(-0.5, 0.75, -0.125),
        c + glam::Vec3::new(-0.25, 1.5, 0.125),
        &limb_uvs,
        [1.0, 1.0, 1.0],
    );

    // 5. Left Arm
    add_cuboid(
        &mut vertices,
        &mut indices,
        c + glam::Vec3::new(0.25, 0.75, -0.125),
        c + glam::Vec3::new(0.5, 1.5, 0.125),
        &limb_uvs,
        [1.0, 1.0, 1.0],
    );

    // 6. Head
    add_cuboid(
        &mut vertices,
        &mut indices,
        c + glam::Vec3::new(-0.25, 1.5, -0.25),
        c + glam::Vec3::new(0.25, 2.3, 0.25), // Steve head is 0.8m tall (actually 8 pixels = 0.5m but slightly enlarged head looks cuter!)
        &head_uvs,
        [1.0, 1.0, 1.0],
    );

    (vertices, indices)
}

fn add_cuboid(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    min: glam::Vec3,
    max: glam::Vec3,
    uvs_per_face: &[[f32; 4]; 6],
    color: [f32; 3],
) {
    let start_idx = vertices.len() as u32;

    let corners = [
        glam::Vec3::new(min.x, min.y, min.z), // 0
        glam::Vec3::new(max.x, min.y, min.z), // 1
        glam::Vec3::new(max.x, max.y, min.z), // 2
        glam::Vec3::new(min.x, max.y, min.z), // 3
        glam::Vec3::new(min.x, min.y, max.z), // 4
        glam::Vec3::new(max.x, min.y, max.z), // 5
        glam::Vec3::new(max.x, max.y, max.z), // 6
        glam::Vec3::new(min.x, max.y, max.z), // 7
    ];

    // Face UV mapping indices
    // 0: West (-X), 1: East (+X), 2: Down (-Y), 3: Up (+Y), 4: North (-Z), 5: South (+Z)

    // West (-X) face
    let uv = uvs_per_face[0];
    vertices.push(Vertex { position: corners[0].into(), uv: [uv[0], uv[3]], color });
    vertices.push(Vertex { position: corners[3].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[7].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[4].into(), uv: [uv[2], uv[3]], color });

    // East (+X) face
    let uv = uvs_per_face[1];
    vertices.push(Vertex { position: corners[5].into(), uv: [uv[0], uv[3]], color });
    vertices.push(Vertex { position: corners[6].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[2].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[1].into(), uv: [uv[2], uv[3]], color });

    // Down (-Y) face
    let uv = uvs_per_face[2];
    vertices.push(Vertex { position: corners[0].into(), uv: [uv[0], uv[3]], color });
    vertices.push(Vertex { position: corners[4].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[5].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[1].into(), uv: [uv[2], uv[3]], color });

    // Up (+Y) face
    let uv = uvs_per_face[3];
    vertices.push(Vertex { position: corners[3].into(), uv: [uv[0], uv[3]], color });
    vertices.push(Vertex { position: corners[2].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[6].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[7].into(), uv: [uv[2], uv[3]], color });

    // North (-Z) face
    let uv = uvs_per_face[4];
    vertices.push(Vertex { position: corners[1].into(), uv: [uv[0], uv[3]], color });
    vertices.push(Vertex { position: corners[2].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[3].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[0].into(), uv: [uv[2], uv[3]], color });

    // South (+Z) face
    let uv = uvs_per_face[5];
    vertices.push(Vertex { position: corners[4].into(), uv: [uv[0], uv[3]], color });
    vertices.push(Vertex { position: corners[7].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[6].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[5].into(), uv: [uv[2], uv[3]], color });

    for f in 0..6 {
        let f_start = start_idx + (f * 4) as u32;
        indices.extend_from_slice(&[
            f_start, f_start + 1, f_start + 2,
            f_start, f_start + 2, f_start + 3,
        ]);
    }
}
