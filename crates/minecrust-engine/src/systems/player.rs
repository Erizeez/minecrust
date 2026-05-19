use hecs::{World, Entity};
use minecrust_shared::ecs::player::{Player, CameraMode};
use minecrust_shared::ecs::transform::LocalTransform;
use minecrust_shared::ecs::animation::Animator;
use crate::input::InputManager;
use crate::physics::{PhysicsManager, AABB};
use crate::world::ChunkManager;
use glam::Vec3;
use winit::keyboard::{Key, NamedKey};

const GRAVITY: f32 = 32.0;
const JUMP_SPEED: f32 = 8.5;
const WALK_SPEED: f32 = 4.3;
const FLY_SPEED: f32 = 10.0;
const DOUBLE_TAP_TIME: f64 = 0.3;

pub fn player_movement_system(
    world: &mut World,
    input: &mut InputManager,
    chunk_manager: &ChunkManager,
    dt: f64,
    time: f64,
) {
    let dt_f32 = dt as f32;

    for (entity, player, transform, mut animator_opt) in world.query_mut::<(Entity, &mut Player, &mut LocalTransform, Option<&mut Animator>)>() {
        // 1. Double tap Space to toggle flying
        if input.is_key_just_pressed(&Key::Named(NamedKey::Space)) {
            if time - player.last_space_press < DOUBLE_TAP_TIME {
                player.is_flying = !player.is_flying;
                player.last_space_press = -10.0;
            } else {
                player.last_space_press = time;
            }
        }

        // 1.5 Toggle Camera Mode
        if input.is_key_just_pressed(&Key::Named(NamedKey::F5)) {
            player.camera_mode = match player.camera_mode {
                CameraMode::FirstPerson => CameraMode::ThirdPersonBack,
                CameraMode::ThirdPersonBack => CameraMode::ThirdPersonFront,
                CameraMode::ThirdPersonFront => CameraMode::FirstPerson,
            };
        }

        // 2. Camera Orientation
        let sensitivity = 0.002;
        player.yaw += input.mouse_dx as f32 * sensitivity;
        player.pitch -= input.mouse_dy as f32 * sensitivity;
        player.pitch = player.pitch.clamp(-89.9_f32.to_radians(), 89.9_f32.to_radians());

        // 3. Direction Vectors
        let forward = Vec3::new(player.yaw.cos() * player.pitch.cos(), player.pitch.sin(), player.yaw.sin()).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();

        // Update body rotation to match yaw (only Y axis rotation)
        transform.rotation = glam::Quat::from_rotation_y(-player.yaw - std::f32::consts::FRAC_PI_2);

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
        if player.is_flying {
            player.velocity = wish_dir * FLY_SPEED;
            if input.is_key_pressed(&Key::Named(NamedKey::Space)) { player.velocity.y += FLY_SPEED; }
            if input.is_key_pressed(&Key::Named(NamedKey::Shift)) { player.velocity.y -= FLY_SPEED; }
        } else {
            player.velocity.x = wish_dir.x * WALK_SPEED;
            player.velocity.z = wish_dir.z * WALK_SPEED;
            
            player.velocity.y -= GRAVITY * dt_f32;
            
            if player.is_grounded && input.is_key_pressed(&Key::Named(NamedKey::Space)) {
                player.velocity.y = JUMP_SPEED;
                player.is_grounded = false;
            }
        }

        // 6. Swept AABB Physics Resolution
        let player_aabb = AABB::new(
            transform.translation - Vec3::new(0.3, 0.0, 0.3),
            transform.translation + Vec3::new(0.3, 1.8, 0.3),
        );

        let (final_vel, grounded) = PhysicsManager::resolve_collision_with_chunks(chunk_manager, &player_aabb, player.velocity, dt_f32);
        
        player.velocity = final_vel;
        if !player.is_flying {
            player.is_grounded = grounded;
        }

        transform.translation += player.velocity * dt_f32;
        
        // Update Animator speed based on lateral movement
        let lateral_speed = Vec3::new(player.velocity.x, 0.0, player.velocity.z).length();
        if let Some(animator) = animator_opt.as_deref_mut() {
            let (target_body_yaw, target_head_yaw, target_head_pitch) = match player.camera_mode {
                CameraMode::ThirdPersonFront => {
                    (-player.yaw + std::f32::consts::PI - std::f32::consts::FRAC_PI_2,
                     -player.yaw + std::f32::consts::PI - std::f32::consts::FRAC_PI_2,
                     player.pitch) // Character head pitch matches player intent
                },
                _ => {
                    (-player.yaw - std::f32::consts::FRAC_PI_2,
                     -player.yaw - std::f32::consts::FRAC_PI_2,
                     player.pitch)
                }
            };

            animator.body_yaw = target_body_yaw;
            animator.head_yaw = target_head_yaw;
            animator.head_pitch = target_head_pitch;

            if player.is_flying || !player.is_grounded {
                animator.speed = 0.0;
            } else {
                animator.speed = lateral_speed / WALK_SPEED;
            }
        }
    }
    input.clear_frame_state();
}

pub fn get_camera_vectors(player: &Player, transform: &LocalTransform, chunk_manager: &ChunkManager) -> (Vec3, f32, f32) {
    let eye = transform.translation + Vec3::new(0.0, 1.62, 0.0);
    let forward = Vec3::new(player.yaw.cos() * player.pitch.cos(), player.pitch.sin(), player.yaw.sin()).normalize();
    
    match player.camera_mode {
        CameraMode::FirstPerson => {
            (eye, player.yaw, player.pitch)
        }
        CameraMode::ThirdPersonBack => {
            let max_dist = 4.0;
            let actual_dist = PhysicsManager::raycast_distance_with_chunks(chunk_manager, eye, -forward, max_dist);
            let actual_eye = eye - forward * actual_dist;
            (actual_eye, player.yaw, player.pitch)
        }
        CameraMode::ThirdPersonFront => {
            let max_dist = 4.0;
            let actual_dist = PhysicsManager::raycast_distance_with_chunks(chunk_manager, eye, forward, max_dist);
            let actual_eye = eye + forward * actual_dist;
            (actual_eye, player.yaw + std::f32::consts::PI, -player.pitch)
        }
    }
}
