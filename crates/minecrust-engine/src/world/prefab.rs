use hecs::{World, Entity};
use minecrust_shared::ecs::player::Player;
use minecrust_shared::ecs::transform::{LocalTransform, GlobalTransform, Children, Parent};
use minecrust_shared::ecs::animation::{Animator, Bone, BoneType};
use minecrust_shared::ecs::mesh::Mesh as EcsMesh;

pub fn spawn_steve(world: &mut World, translation: glam::Vec3) -> Entity {
    let local_player_entity = world.spawn((
        Player::new(),
        LocalTransform {
            translation,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        },
        GlobalTransform(glam::Mat4::IDENTITY),
        Animator {
            walk_timer: 0.0,
            speed: 0.0,
            body_yaw: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
        },
        Children(Vec::new()), // Will be populated with bones
    ));

    // Spawn bones
    let head = world.spawn((
        Bone { bone_type: BoneType::Head },
        LocalTransform {
            translation: glam::Vec3::new(0.0, 1.5, 0.0), // relative to player base
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        },
        GlobalTransform(glam::Mat4::IDENTITY),
        Parent(local_player_entity),
        EcsMesh::new("steve_head"),
    ));

    let body = world.spawn((
        Bone { bone_type: BoneType::Body },
        LocalTransform {
            translation: glam::Vec3::new(0.0, 0.75, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        },
        GlobalTransform(glam::Mat4::IDENTITY),
        Parent(local_player_entity),
        EcsMesh::new("steve_body"),
    ));

    let right_arm = world.spawn((
        Bone { bone_type: BoneType::RightArm },
        LocalTransform {
            translation: glam::Vec3::new(0.375, 1.375, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        },
        GlobalTransform(glam::Mat4::IDENTITY),
        Parent(local_player_entity),
        EcsMesh::new("steve_right_arm"),
    ));

    let left_arm = world.spawn((
        Bone { bone_type: BoneType::LeftArm },
        LocalTransform {
            translation: glam::Vec3::new(-0.375, 1.375, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        },
        GlobalTransform(glam::Mat4::IDENTITY),
        Parent(local_player_entity),
        EcsMesh::new("steve_left_arm"),
    ));

    let right_leg = world.spawn((
        Bone { bone_type: BoneType::RightLeg },
        LocalTransform {
            translation: glam::Vec3::new(0.125, 0.75, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        },
        GlobalTransform(glam::Mat4::IDENTITY),
        Parent(local_player_entity),
        EcsMesh::new("steve_right_leg"),
    ));

    let left_leg = world.spawn((
        Bone { bone_type: BoneType::LeftLeg },
        LocalTransform {
            translation: glam::Vec3::new(-0.125, 0.75, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        },
        GlobalTransform(glam::Mat4::IDENTITY),
        Parent(local_player_entity),
        EcsMesh::new("steve_left_leg"),
    ));

    // Add children to player
    if let Ok(mut children) = world.get::<&mut Children>(local_player_entity) {
        children.0.push(head);
        children.0.push(body);
        children.0.push(right_arm);
        children.0.push(left_arm);
        children.0.push(right_leg);
        children.0.push(left_leg);
    }
    
    local_player_entity
}
