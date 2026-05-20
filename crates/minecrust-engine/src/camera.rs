use glam::{Mat4, Vec3};
use bytemuck::{Pod, Zeroable};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

pub struct Camera {
    pub eye: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        let forward = Vec3::new(cos_yaw * cos_pitch, sin_pitch, sin_yaw * cos_pitch);
        let right = Vec3::new(-sin_yaw, 0.0, cos_yaw).normalize();
        let up = right.cross(forward).normalize();

        let view = Mat4::look_to_rh(self.eye, forward, up);
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        
        // wgpu expects z in 0.0 to 1.0, so we convert from OpenGL's -1.0 to 1.0
        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    prev_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    world_time: f32,
    frame_index: u32,
    enable_rt: u32,
    _padding: u32, // Pad to 16 bytes alignment
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            prev_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0; 4],
            world_time: 0.0,
            frame_index: 0,
            enable_rt: 1,
            _padding: 0,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.prev_view_proj = self.view_proj;
        
        let view_proj = camera.build_view_projection_matrix();
        self.view_proj = view_proj.to_cols_array_2d();
        self.inv_view_proj = view_proj.inverse().to_cols_array_2d();
        self.camera_pos = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
    }
    
    pub fn update_time(&mut self, time: f32) {
        self.world_time = time;
    }

    pub fn update_frame_index(&mut self) {
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    pub fn update_settings(&mut self, enable_rt: bool) {
        self.enable_rt = if enable_rt { 1 } else { 0 };
    }

    pub fn enable_rt(&self) -> bool {
        self.enable_rt != 0
    }
}
