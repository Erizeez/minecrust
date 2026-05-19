use crate::world::WorldManager;
use glam::Vec3;

pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn offset(&self, offset: Vec3) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x &&
        self.min.y < other.max.y && self.max.y > other.min.y &&
        self.min.z < other.max.z && self.max.z > other.min.z
    }
}

pub struct PhysicsManager {}

impl PhysicsManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Resolves collision for an entity with `aabb` trying to move by `velocity` * `dt`.
    /// Returns the modified velocity that does not interpenetrate with blocks.
    pub fn resolve_collision_with_chunks(
        chunk_manager: &crate::world::ChunkManager,
        aabb: &AABB,
        velocity: Vec3,
        dt: f32,
    ) -> (Vec3, bool) {
        let mut final_vel = velocity * dt;
        let mut grounded = false;

        // Perform swept collision by moving one axis at a time
        // This is a naive but stable approach for block games.
        
        // 1. Y Axis
        if final_vel.y != 0.0 {
            let next_aabb = aabb.offset(Vec3::new(0.0, final_vel.y, 0.0));
            if Self::check_aabb_collision_with_chunks(chunk_manager, &next_aabb) {
                if final_vel.y < 0.0 {
                    grounded = true;
                }
                final_vel.y = 0.0;
            }
        }
        
        // 2. X Axis
        if final_vel.x != 0.0 {
            let next_aabb = aabb.offset(Vec3::new(final_vel.x, final_vel.y, 0.0));
            if Self::check_aabb_collision_with_chunks(chunk_manager, &next_aabb) {
                final_vel.x = 0.0;
            }
        }

        // 3. Z Axis
        if final_vel.z != 0.0 {
            let next_aabb = aabb.offset(Vec3::new(final_vel.x, final_vel.y, final_vel.z));
            if Self::check_aabb_collision_with_chunks(chunk_manager, &next_aabb) {
                final_vel.z = 0.0;
            }
        }

        // Return the resolved velocity back to per-second (divide by dt)
        let resolved_vel = if dt > 0.0 {
            final_vel / dt
        } else {
            Vec3::ZERO
        };
        
        (resolved_vel, grounded)
    }

    /// Checks if the given AABB intersects with any solid block in the world.
    fn check_aabb_collision_with_chunks(chunk_manager: &crate::world::ChunkManager, aabb: &AABB) -> bool {
        use crate::world::{CHUNK_WIDTH, CHUNK_DEPTH};

        let min_x = aabb.min.x.floor() as i32;
        let max_x = aabb.max.x.floor() as i32;
        let min_y = aabb.min.y.floor() as i32;
        let max_y = aabb.max.y.floor() as i32;
        let min_z = aabb.min.z.floor() as i32;
        let max_z = aabb.max.z.floor() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                for z in min_z..=max_z {
                    // Convert world coordinates to chunk coordinates
                    let chunk_x = x.div_euclid(CHUNK_WIDTH as i32);
                    let chunk_z = z.div_euclid(CHUNK_DEPTH as i32);
                    
                    let local_x = x.rem_euclid(CHUNK_WIDTH as i32) as usize;
                    let local_z = z.rem_euclid(CHUNK_DEPTH as i32) as usize;

                    if let Some(chunk) = chunk_manager.chunks.get(&(chunk_x, chunk_z)) {
                        let block_id = chunk.get_block(local_x, y, local_z);
                        if block_id != 0 {
                            return true;
                        }
                    } else {
                        // If chunk is not loaded, act as solid to prevent falling out
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Performs a simple point raycast from `start` in the given `dir` (normalized) up to `max_dist`.
    /// Returns the distance at which it hits a solid block, or `max_dist` if no hit occurs.
    pub fn raycast_distance_with_chunks(chunk_manager: &crate::world::ChunkManager, start: Vec3, dir: Vec3, max_dist: f32) -> f32 {
        let step_size = 0.1;
        let steps = (max_dist / step_size).ceil() as usize;
        
        // Use a small AABB to simulate camera bounds to prevent clipping through tight corners
        let camera_bounds = Vec3::new(0.1, 0.1, 0.1);

        for i in 0..=steps {
            let dist = (i as f32) * step_size;
            if dist > max_dist {
                break;
            }
            
            let pos = start + dir * dist;
            let aabb = AABB::new(pos - camera_bounds, pos + camera_bounds);
            
            if Self::check_aabb_collision_with_chunks(chunk_manager, &aabb) {
                // Return distance but retract slightly so we aren't completely inside the block
                return (dist - 0.2).max(0.0);
            }
        }
        
        max_dist
    }
}
