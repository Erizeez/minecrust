use minecrust_engine::renderer::Vertex;
use minecrust_shared::AssetPack;
use wgpu::{Device, Buffer, util::DeviceExt};
use std::collections::HashMap;

pub struct EntityMeshes {
    pub meshes: HashMap<String, (Buffer, Buffer, u32)>,
}

impl EntityMeshes {
    pub fn new(device: &Device, pack: &AssetPack, model_type: crate::steve::PlayerModelType) -> Self {
        let mut meshes = HashMap::new();
        
        let bones = [
            "steve_head",
            "steve_body",
            "steve_right_arm",
            "steve_left_arm",
            "steve_right_leg",
            "steve_left_leg",
        ];

        for bone in bones {
            let (v_buf, i_buf, count) = Self::create_bone(device, pack, model_type, bone);
            meshes.insert(bone.to_string(), (v_buf, i_buf, count));
        }

        Self { meshes }
    }

    fn create_bone(
        device: &Device, 
        pack: &AssetPack, 
        model_type: crate::steve::PlayerModelType, 
        bone: &str
    ) -> (Buffer, Buffer, u32) {
        let (vertices, indices) = crate::steve::build_steve_bone_vertices(pack, model_type, bone);

        let v_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Vertex Buffer", bone)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let i_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Index Buffer", bone)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (v_buf, i_buf, indices.len() as u32)
    }
}

