use hecs::{World, Entity, Without};

pub struct LocalTransform;
pub struct GlobalTransform;
pub struct Parent;

pub fn test(w: &mut World) {
    for (entity, local, _) in w.query_mut::<(Entity, &LocalTransform, &GlobalTransform)>().without::<&Parent>() {
        
    }
}
