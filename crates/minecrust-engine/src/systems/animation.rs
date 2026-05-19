use hecs::{World, Entity};
use glam::Quat;
use minecrust_shared::ecs::transform::{LocalTransform, Children};
use minecrust_shared::ecs::animation::{Animator, Bone, BoneType};
use std::f32::consts::PI;

pub fn procedural_animation_system(world: &mut World, dt: f32) {
    let mut animators_to_update = Vec::new();

    for (entity, animator, children) in world.query_mut::<(Entity, &mut Animator, &Children)>() {
        // Step timer based on speed
        animator.walk_timer += animator.speed * dt * 2.0;
        
        animators_to_update.push((entity, animator.clone(), children.0.clone()));
    }

    // Update children local transforms
    for (_, animator, children) in animators_to_update {
        // 1.5 multiplier makes the swing visible
        let swing = animator.walk_timer.sin();

        for child in children {
            if let Ok(bone) = world.get::<&Bone>(child) {
                if let Ok(mut transform) = world.get::<&mut LocalTransform>(child) {
                    match bone.bone_type {
                        BoneType::Head => {
                            // Relative yaw for head turning
                            let mut relative_yaw = animator.head_yaw - animator.body_yaw;
                            // Normalize to -PI..PI
                            while relative_yaw > PI { relative_yaw -= 2.0 * PI; }
                            while relative_yaw < -PI { relative_yaw += 2.0 * PI; }
                            
                            // Need to clamp to prevent Exorcist head turning
                            let clamped_yaw = relative_yaw.clamp(-PI / 2.0, PI / 2.0);
                            
                            transform.rotation = Quat::from_euler(
                                glam::EulerRot::YXZ,
                                clamped_yaw,
                                animator.head_pitch,
                                0.0,
                            );
                        }
                        BoneType::LeftArm => {
                            // Arms swing opposite to legs
                            transform.rotation = Quat::from_rotation_x(swing * 1.2);
                        }
                        BoneType::RightArm => {
                            transform.rotation = Quat::from_rotation_x(-swing * 1.2);
                        }
                        BoneType::LeftLeg => {
                            transform.rotation = Quat::from_rotation_x(-swing * 1.2);
                        }
                        BoneType::RightLeg => {
                            transform.rotation = Quat::from_rotation_x(swing * 1.2);
                        }
                        BoneType::Body => {
                            transform.rotation = Quat::IDENTITY; 
                        }
                    }
                }
            }
        }
    }
}
