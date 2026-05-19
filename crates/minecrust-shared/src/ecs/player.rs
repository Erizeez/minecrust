use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    FirstPerson,
    ThirdPersonBack,
    ThirdPersonFront,
}

/// Component that marks an entity as the player and stores player-specific state.
#[derive(Debug, Clone)]
pub struct Player {
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub is_flying: bool,
    pub is_grounded: bool,
    pub camera_mode: CameraMode,
    pub last_space_press: f64,
}

impl Player {
    pub fn new() -> Self {
        Self {
            velocity: Vec3::ZERO,
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
            is_flying: true,
            is_grounded: false,
            camera_mode: CameraMode::FirstPerson,
            last_space_press: -10.0,
        }
    }
}
