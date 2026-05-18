use crate::input::InputManager;
use crate::physics::{PhysicsManager, AABB};
use crate::world::WorldManager;
use glam::Vec3;
use winit::keyboard::{Key, NamedKey};

const GRAVITY: f32 = 32.0;
const JUMP_SPEED: f32 = 8.5;
const WALK_SPEED: f32 = 4.3;
const FLY_SPEED: f32 = 10.0;
const DOUBLE_TAP_TIME: f64 = 0.3;

pub struct PlayerController {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub is_flying: bool,
    pub is_grounded: bool,
    
    last_space_press: f64,
}

impl PlayerController {
    pub fn new(spawn_pos: Vec3) -> Self {
        Self {
            position: spawn_pos,
            velocity: Vec3::ZERO,
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
            is_flying: true,
            is_grounded: false,
            last_space_press: -10.0,
        }
    }

    pub fn update(
        &mut self,
        input: &mut InputManager,
        world: &mut WorldManager,
        dt: f64,
        time: f64,
    ) {
        let dt_f32 = dt as f32;

        // 1. Double tap Space to toggle flying
        if input.is_key_just_pressed(&Key::Named(NamedKey::Space)) {
            if time - self.last_space_press < DOUBLE_TAP_TIME {
                self.is_flying = !self.is_flying;
                // Double tap consumed, prevent triple tap toggling again
                self.last_space_press = -10.0;
            } else {
                self.last_space_press = time;
            }
        }

        // 2. Camera Orientation
        let sensitivity = 0.002;
        self.yaw += input.mouse_dx as f32 * sensitivity;
        self.pitch -= input.mouse_dy as f32 * sensitivity;
        
        // Clamp pitch to avoid gimbal lock (-89.9 to 89.9 degrees)
        self.pitch = self.pitch.clamp(-89.9_f32.to_radians(), 89.9_f32.to_radians());
        input.clear_frame_state(); // Clear input deltas and just_pressed

        // 3. Direction Vectors
        let forward = Vec3::new(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin()).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();

        // 4. Movement Intent
        let mut wish_dir = Vec3::ZERO;
        if input.is_key_pressed(&Key::Character("w".into())) { wish_dir += flat_forward; }
        if input.is_key_pressed(&Key::Character("s".into())) { wish_dir -= flat_forward; }
        if input.is_key_pressed(&Key::Character("d".into())) { wish_dir += right; }
        if input.is_key_pressed(&Key::Character("a".into())) { wish_dir -= right; }
        
        if wish_dir.length_squared() > 0.0 {
            wish_dir = wish_dir.normalize();
        }

        // 5. Apply Movement & Physics
        if self.is_flying {
            // Creative flying
            self.velocity = wish_dir * FLY_SPEED;
            if input.is_key_pressed(&Key::Named(NamedKey::Space)) { self.velocity.y += FLY_SPEED; }
            if input.is_key_pressed(&Key::Named(NamedKey::Shift)) { self.velocity.y -= FLY_SPEED; }
        } else {
            // Survival walking
            self.velocity.x = wish_dir.x * WALK_SPEED;
            self.velocity.z = wish_dir.z * WALK_SPEED;
            
            // Gravity
            self.velocity.y -= GRAVITY * dt_f32;
            
            // Jump
            if self.is_grounded && input.is_key_pressed(&Key::Named(NamedKey::Space)) {
                self.velocity.y = JUMP_SPEED;
                self.is_grounded = false; // Immediately leave ground
            }
        }

        // 6. Swept AABB Physics Resolution
        let player_aabb = AABB::new(
            self.position - Vec3::new(0.3, 0.0, 0.3),
            self.position + Vec3::new(0.3, 1.8, 0.3),
        );

        let (final_vel, grounded) = PhysicsManager::resolve_collision(world, &player_aabb, self.velocity, dt_f32);
        
        self.velocity = final_vel;
        if !self.is_flying {
            self.is_grounded = grounded;
        }

        self.position += self.velocity * dt_f32;
    }

    pub fn get_camera_vectors(&self) -> (Vec3, Vec3) {
        let eye = self.position + Vec3::new(0.0, 1.62, 0.0);
        let forward = Vec3::new(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin()).normalize();
        let target = eye + forward;
        (eye, target)
    }
}
