use crate::app::FrameContext;
use crate::ecs::component::{motion_path_position, MotionPath};
use crate::ecs::resource::{FrameClock, TimelineState};
use crate::ecs::world::{Entity, Transform, World};

/// Sync `Transform.translation` for all entities with an enabled `MotionPath`.
///
/// For each entity that has `MotionPath` (enabled), computes the circular position from
/// `motion_path_position` at the current timeline time and writes it into `Transform.translation`.
/// If the entity does not have a `Transform`, one is inserted.
///
/// Under a fixed-step `FrameClock` the time is the clock's; otherwise `TimelineState.current_time`.
pub fn sync_motion_paths(world: &mut World) {
    let fixed_step_time = world
        .get_resource::<FrameClock>()
        .and_then(|clock| clock.fixed_time_seconds());
    let current_time = fixed_step_time.unwrap_or_else(|| {
        world
            .get_resource::<TimelineState>()
            .map(|ts| ts.current_time)
            .unwrap_or(0.0)
    });

    // Collect entities with MotionPath first to avoid borrow conflicts
    let entities: Vec<Entity> = world
        .iter_components::<MotionPath>()
        .map(|(e, _)| e)
        .collect();
    for entity in entities {
        let path = match world.get_component::<MotionPath>(entity) {
            Some(p) => p,
            None => continue,
        };
        if !path.enabled {
            continue;
        }

        let new_pos = motion_path_position(&path, current_time);

        if let Some(mut transform) = world.get_component_mut::<Transform>(entity) {
            transform.translation = new_pos;
        } else {
            let transform = Transform {
                translation: new_pos,
                ..Default::default()
            };
            world.insert_component(entity, transform);
        }
    }
}

/// Legacy wrapper — calls `sync_motion_paths` via `ctx.world`.
pub fn motion_path_sync(ctx: &mut FrameContext) {
    sync_motion_paths(ctx.world);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_run_inserts_transform_and_moves_along_orbit() {
        let mut world = World::new();

        world.insert_resource(FrameClock::fixed(FrameClock::BATCH_DELTA_SECONDS));

        // Spawn entity with MotionPath but NO Transform
        let entity = world.spawn();
        world.insert_component(
            entity,
            MotionPath {
                center: cgmath::Vector3::new(0.0, 0.0, 0.0),
                radius: 5.0,
                angular_speed: 1.0,
                phase_offset: 0.0,
                enabled: true,
            },
        );

        sync_motion_paths(&mut world);

        let transform = world.get_component::<Transform>(entity).unwrap();
        // At time 0.0, angle = 0.0, so position should be (radius, 0, 0) = (5.0, 0.0, 0.0)
        assert!(
            (transform.translation.x - 5.0).abs() < 1e-6,
            "Expected translation.x ~= 5.0, got {}",
            transform.translation.x
        );
        assert!(
            transform.translation.y.abs() < 1e-6,
            "Expected translation.y ~= 0.0, got {}",
            transform.translation.y
        );
        assert!(
            transform.translation.z.abs() < 1e-6,
            "Expected translation.z ~= 0.0, got {}",
            transform.translation.z
        );

        world.resource_mut::<FrameClock>().frame = 15;
        sync_motion_paths(&mut world);

        let transform = world.get_component::<Transform>(entity).unwrap();
        // At time 0.25, angle = 1.0 * 0.25 = 0.25 radians
        // x = 5.0 * cos(0.25) ~= 4.80, z = 5.0 * sin(0.25) ~= 1.23
        let expected_x = 5.0 * 0.25_f32.cos();
        let expected_z = 5.0 * 0.25_f32.sin();
        assert!(
            (transform.translation.x - expected_x).abs() < 1e-4,
            "Expected translation.x ~= {}, got {}",
            expected_x,
            transform.translation.x
        );
        assert!(
            transform.translation.y.abs() < 1e-6,
            "Expected translation.y ~= 0.0, got {}",
            transform.translation.y
        );
        assert!(
            (transform.translation.z - expected_z).abs() < 1e-4,
            "Expected translation.z ~= {}, got {}",
            expected_z,
            transform.translation.z
        );

        world.resource_mut::<FrameClock>().frame = 30;
        sync_motion_paths(&mut world);

        let transform = world.get_component::<Transform>(entity).unwrap();
        // At time 0.5, angle = 1.0 * 0.5 = 0.5 radians
        // x = 5.0 * cos(0.5) ~= 4.38, z = 5.0 * sin(0.5) ~= 2.39
        let expected_x = 5.0 * 0.5_f32.cos();
        let expected_z = 5.0 * 0.5_f32.sin();
        assert!(
            (transform.translation.x - expected_x).abs() < 1e-4,
            "Expected translation.x ~= {}, got {}",
            expected_x,
            transform.translation.x
        );
        assert!(
            transform.translation.y.abs() < 1e-6,
            "Expected translation.y ~= 0.0, got {}",
            transform.translation.y
        );
        assert!(
            (transform.translation.z - expected_z).abs() < 1e-4,
            "Expected translation.z ~= {}, got {}",
            expected_z,
            transform.translation.z
        );
    }
}
