/// Defines the type of a skeletal bone (rigid part).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoneType {
    Head,
    Body,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

/// Component attached to a child entity marking it as a specific bone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bone {
    pub bone_type: BoneType,
}

/// Component attached to the root entity to manage animation state.
#[derive(Debug, Clone, PartialEq)]
pub struct Animator {
    /// Timer for procedural walk cycle
    pub walk_timer: f32,
    /// Absolute yaw of the head
    pub head_yaw: f32,
    /// Absolute pitch of the head
    pub head_pitch: f32,
    /// Absolute yaw of the body
    pub body_yaw: f32,
    /// Current scalar movement speed
    pub speed: f32,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            walk_timer: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
            body_yaw: 0.0,
            speed: 0.0,
        }
    }
}
