use minecrust_engine::renderer::Vertex;
use minecrust_shared::AssetPack;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerModelType {
    Steve,
    Alex,
}

pub fn build_steve_vertices(
    position: glam::Vec3,
    pack: &AssetPack,
    model_type: PlayerModelType,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let skin_name = match model_type {
        PlayerModelType::Steve => "steve",
        PlayerModelType::Alex => "alex",
    };

    let skin_uv = pack.texture_dict.get(skin_name).copied().unwrap_or([0.0, 0.0, 1.0, 1.0]);

    // Helper to extract sub-UVs from the 64x64 skin
    let get_uv = |x: f32, y: f32, w: f32, h: f32| -> [f32; 4] {
        let u0 = skin_uv[0] + (x / 64.0) * (skin_uv[2] - skin_uv[0]);
        let v0 = skin_uv[1] + (y / 64.0) * (skin_uv[3] - skin_uv[1]);
        let u1 = skin_uv[0] + ((x + w) / 64.0) * (skin_uv[2] - skin_uv[0]);
        let v1 = skin_uv[1] + ((y + h) / 64.0) * (skin_uv[3] - skin_uv[1]);
        [u0, v0, u1, v1]
    };

    // Helper to define 6 faces from skin part layout
    // Minecraft layout for a part at (X, Y) with size (W, H, D):
    // Top: (X+D, Y) size (W, D)
    // Bottom: (X+D+W, Y) size (W, D)
    // Right (West -X): (X, Y+D) size (D, H)
    // Front (North -Z): (X+D, Y+D) size (W, H)
    // Left (East +X): (X+D+W, Y+D) size (D, H)
    // Back (South +Z): (X+D+W+D, Y+D) size (W, H)
    // Note: The ordering in `add_cuboid` is:
    // 0: West (-X)
    // 1: East (+X)
    // 2: Down (-Y)
    // 3: Up (+Y)
    // 4: North (-Z)
    // 5: South (+Z)
    let get_faces = |x: f32, y: f32, w: f32, h: f32, d: f32| -> [[f32; 4]; 6] {
        [
            get_uv(x, y + d, d, h),               // 0: West (-X)
            get_uv(x + d + w, y + d, d, h),       // 1: East (+X)
            get_uv(x + d + w, y, w, d),           // 2: Down (-Y)
            get_uv(x + d, y, w, d),               // 3: Up (+Y)
            get_uv(x + d, y + d, w, h),           // 4: North (-Z)
            get_uv(x + d + w + d, y + d, w, h),   // 5: South (+Z)
        ]
    };

    let p = 0.0625; // 1 pixel = 1/16 meters
    let c = position;

    // 1. Head (8x8x8 pixels)
    // Center is 0, bottom is 24p, top is 32p. 
    // Minecraft actual head: -4 to 4 in X/Z, 24 to 32 in Y.
    let head_uvs = get_faces(0.0, 0.0, 8.0, 8.0, 8.0);
    add_cuboid(
        &mut vertices, &mut indices,
        c + glam::Vec3::new(-4.0 * p, 24.0 * p, -4.0 * p),
        c + glam::Vec3::new(4.0 * p, 32.0 * p, 4.0 * p),
        &head_uvs, [1.0, 1.0, 1.0, 1.0],
    );

    // 2. Torso (8x12x4 pixels)
    // X: -4 to 4, Y: 12 to 24, Z: -2 to 2
    let torso_uvs = get_faces(16.0, 16.0, 8.0, 12.0, 4.0);
    add_cuboid(
        &mut vertices, &mut indices,
        c + glam::Vec3::new(-4.0 * p, 12.0 * p, -2.0 * p),
        c + glam::Vec3::new(4.0 * p, 24.0 * p, 2.0 * p),
        &torso_uvs, [1.0, 1.0, 1.0, 1.0],
    );

    // 3. Right Leg (-X side in MC, +X side in model if we look from front. Wait, Right leg is -X.)
    // X: -4 to 0, Y: 0 to 12, Z: -2 to 2
    let r_leg_uvs = get_faces(0.0, 16.0, 4.0, 12.0, 4.0);
    add_cuboid(
        &mut vertices, &mut indices,
        c + glam::Vec3::new(-4.0 * p, 0.0 * p, -2.0 * p),
        c + glam::Vec3::new(0.0 * p, 12.0 * p, 2.0 * p),
        &r_leg_uvs, [1.0, 1.0, 1.0, 1.0],
    );

    // 4. Left Leg (+X side)
    // X: 0 to 4, Y: 0 to 12, Z: -2 to 2
    let l_leg_uvs = get_faces(16.0, 48.0, 4.0, 12.0, 4.0);
    add_cuboid(
        &mut vertices, &mut indices,
        c + glam::Vec3::new(0.0 * p, 0.0 * p, -2.0 * p),
        c + glam::Vec3::new(4.0 * p, 12.0 * p, 2.0 * p),
        &l_leg_uvs, [1.0, 1.0, 1.0, 1.0],
    );

    // 5. Right Arm (-X side)
    // Steve: 4x12x4. Alex: 3x12x4
    // Steve X: -8 to -4. Alex X: -7 to -4. Y: 12 to 24, Z: -2 to 2
    let arm_w = if model_type == PlayerModelType::Alex { 3.0 } else { 4.0 };
    let r_arm_uvs = get_faces(40.0, 16.0, arm_w, 12.0, 4.0);
    add_cuboid(
        &mut vertices, &mut indices,
        c + glam::Vec3::new(-4.0 * p - arm_w * p, 12.0 * p, -2.0 * p),
        c + glam::Vec3::new(-4.0 * p, 24.0 * p, 2.0 * p),
        &r_arm_uvs, [1.0, 1.0, 1.0, 1.0],
    );

    // 6. Left Arm (+X side)
    // Steve X: 4 to 8. Alex X: 4 to 7. Y: 12 to 24, Z: -2 to 2
    let l_arm_uvs = get_faces(32.0, 48.0, arm_w, 12.0, 4.0);
    add_cuboid(
        &mut vertices, &mut indices,
        c + glam::Vec3::new(4.0 * p, 12.0 * p, -2.0 * p),
        c + glam::Vec3::new(4.0 * p + arm_w * p, 24.0 * p, 2.0 * p),
        &l_arm_uvs, [1.0, 1.0, 1.0, 1.0],
    );

    (vertices, indices)
}

pub fn build_steve_bone_vertices(
    pack: &AssetPack,
    model_type: PlayerModelType,
    bone_name: &str,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let skin_name = match model_type {
        PlayerModelType::Steve => "steve",
        PlayerModelType::Alex => "alex",
    };

    let skin_uv = pack.texture_dict.get(skin_name).copied().unwrap_or([0.0, 0.0, 1.0, 1.0]);

    let get_uv = |x: f32, y: f32, w: f32, h: f32| -> [f32; 4] {
        let u0 = skin_uv[0] + (x / 64.0) * (skin_uv[2] - skin_uv[0]);
        let v0 = skin_uv[1] + (y / 64.0) * (skin_uv[3] - skin_uv[1]);
        let u1 = skin_uv[0] + ((x + w) / 64.0) * (skin_uv[2] - skin_uv[0]);
        let v1 = skin_uv[1] + ((y + h) / 64.0) * (skin_uv[3] - skin_uv[1]);
        [u0, v0, u1, v1]
    };

    let get_faces = |x: f32, y: f32, w: f32, h: f32, d: f32| -> [[f32; 4]; 6] {
        [
            get_uv(x, y + d, d, h),               // 0: West (-X)
            get_uv(x + d + w, y + d, d, h),       // 1: East (+X)
            get_uv(x + d + w, y, w, d),           // 2: Down (-Y)
            get_uv(x + d, y, w, d),               // 3: Up (+Y)
            get_uv(x + d, y + d, w, h),           // 4: North (-Z)
            get_uv(x + d + w + d, y + d, w, h),   // 5: South (+Z)
        ]
    };

    let p = 0.0625;
    let arm_w = if model_type == PlayerModelType::Alex { 3.0 } else { 4.0 };

    match bone_name {
        "steve_head" => {
            let uvs = get_faces(0.0, 0.0, 8.0, 8.0, 8.0);
            add_cuboid(&mut vertices, &mut indices, glam::Vec3::new(-4.0 * p, 0.0, -4.0 * p), glam::Vec3::new(4.0 * p, 8.0 * p, 4.0 * p), &uvs, [1.0, 1.0, 1.0, 1.0]);
        }
        "steve_body" => {
            let uvs = get_faces(16.0, 16.0, 8.0, 12.0, 4.0);
            add_cuboid(&mut vertices, &mut indices, glam::Vec3::new(-4.0 * p, 0.0, -2.0 * p), glam::Vec3::new(4.0 * p, 12.0 * p, 2.0 * p), &uvs, [1.0, 1.0, 1.0, 1.0]);
        }
        "steve_right_arm" => {
            // Note: In our ECS, right arm is +X side (model right)
            let uvs = get_faces(40.0, 16.0, arm_w, 12.0, 4.0);
            let aw_p = arm_w * p / 2.0;
            add_cuboid(&mut vertices, &mut indices, glam::Vec3::new(-aw_p, -10.0 * p, -2.0 * p), glam::Vec3::new(aw_p, 2.0 * p, 2.0 * p), &uvs, [1.0, 1.0, 1.0, 1.0]);
        }
        "steve_left_arm" => {
            // Left arm is -X side (model left)
            let uvs = get_faces(32.0, 48.0, arm_w, 12.0, 4.0);
            let aw_p = arm_w * p / 2.0;
            add_cuboid(&mut vertices, &mut indices, glam::Vec3::new(-aw_p, -10.0 * p, -2.0 * p), glam::Vec3::new(aw_p, 2.0 * p, 2.0 * p), &uvs, [1.0, 1.0, 1.0, 1.0]);
        }
        "steve_right_leg" => {
            let uvs = get_faces(0.0, 16.0, 4.0, 12.0, 4.0);
            add_cuboid(&mut vertices, &mut indices, glam::Vec3::new(-2.0 * p, -12.0 * p, -2.0 * p), glam::Vec3::new(2.0 * p, 0.0, 2.0 * p), &uvs, [1.0, 1.0, 1.0, 1.0]);
        }
        "steve_left_leg" => {
            let uvs = get_faces(16.0, 48.0, 4.0, 12.0, 4.0);
            add_cuboid(&mut vertices, &mut indices, glam::Vec3::new(-2.0 * p, -12.0 * p, -2.0 * p), glam::Vec3::new(2.0 * p, 0.0, 2.0 * p), &uvs, [1.0, 1.0, 1.0, 1.0]);
        }
        _ => {}
    }

    (vertices, indices)
}

fn add_cuboid(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    min: glam::Vec3,
    max: glam::Vec3,
    uvs_per_face: &[[f32; 4]; 6],
    color: [f32; 4],
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
    vertices.push(Vertex { position: corners[1].into(), uv: [uv[2], uv[3]], color });
    vertices.push(Vertex { position: corners[2].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[3].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[0].into(), uv: [uv[0], uv[3]], color });

    // South (+Z) face
    let uv = uvs_per_face[5];
    vertices.push(Vertex { position: corners[4].into(), uv: [uv[2], uv[3]], color });
    vertices.push(Vertex { position: corners[7].into(), uv: [uv[2], uv[1]], color });
    vertices.push(Vertex { position: corners[6].into(), uv: [uv[0], uv[1]], color });
    vertices.push(Vertex { position: corners[5].into(), uv: [uv[0], uv[3]], color });

    for f in 0..6 {
        let f_start = start_idx + (f * 4) as u32;
        indices.extend_from_slice(&[
            f_start, f_start + 2, f_start + 1,
            f_start, f_start + 3, f_start + 2,
        ]);
    }
}
