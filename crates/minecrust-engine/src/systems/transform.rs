use hecs::{World, Entity};
use minecrust_shared::ecs::transform::{LocalTransform, GlobalTransform, Parent, Children};
use glam::Mat4;

pub fn transform_update_system(world: &mut World) {
    // 1. Find root entities (entities with LocalTransform but no Parent)
    let mut roots = Vec::new();
    for (entity, local_transform, _) in world.query_mut::<(Entity, &LocalTransform, &GlobalTransform)>().without::<&Parent>() {
        roots.push((entity, local_transform.compute_matrix()));
    }

    // 2. Update their GlobalTransforms
    for (entity, global_mat) in roots.iter() {
        if let Ok(mut global_transform) = world.get::<&mut GlobalTransform>(*entity) {
            global_transform.0 = *global_mat;
        }
        
        // 3. Recursively update children
        update_children(world, *entity, *global_mat);
    }
}

fn update_children(world: &mut World, parent: Entity, parent_mat: Mat4) {
    let children_entities = if let Ok(children) = world.get::<&Children>(parent) {
        children.0.clone()
    } else {
        return;
    };

    for child in children_entities {
        let child_mat = if let Ok(local_transform) = world.get::<&LocalTransform>(child) {
            parent_mat * local_transform.compute_matrix()
        } else {
            parent_mat
        };

        if let Ok(mut global_transform) = world.get::<&mut GlobalTransform>(child) {
            global_transform.0 = child_mat;
        }

        update_children(world, child, child_mat);
    }
}
