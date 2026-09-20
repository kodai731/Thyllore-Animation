use crate::ecs::component::MotionPath;
use crate::ecs::resource::BatchFlameOrbit;
use crate::ecs::world::{Entity, Transform, World};
use crate::hooks::effect_spawn::EffectSpawnHooks;

/// XZ circular orbit offset at `t_seconds`: (R cos(2πt/T), 0, R sin(2πt/T)), zero for T <= 0.
pub fn compute_orbit_offset(radius: f32, period_seconds: f32, t_seconds: f32) -> [f32; 3] {
    if period_seconds <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let angle = 2.0 * std::f32::consts::PI * t_seconds / period_seconds;
    [radius * angle.cos(), 0.0, radius * angle.sin()]
}

fn collect_effect_entities(world: &World) -> Vec<Entity> {
    let mut entities = Vec::new();
    if let Some(hooks) = world.get_resource::<EffectSpawnHooks>() {
        for hook in hooks.ordered() {
            entities.extend((hook.entities)(world));
        }
    }
    entities
}

/// On its first call records the orbit center and gives every effect a `MotionPath`; afterwards
/// `sync_motion_paths` moves them.
pub fn batch_run_update_orbit(world: &mut World) {
    let effect_entities = collect_effect_entities(world);
    let (radius, period_seconds, center) = {
        let Some(mut orbit) = world.get_resource_mut::<BatchFlameOrbit>() else {
            return;
        };
        if orbit.initial.is_some() {
            return;
        }
        let center = effect_entities
            .first()
            .and_then(|&first| world.get_component::<Transform>(first))
            .map(|transform| transform.translation)
            .unwrap_or(cgmath::Vector3::new(0.0, 0.0, 0.0));
        orbit.initial = Some(center);
        (orbit.radius, orbit.period_seconds, center)
    };

    if period_seconds <= 0.0 {
        return;
    }

    for entity in effect_entities {
        world.insert_component(
            entity,
            MotionPath {
                center,
                radius,
                angular_speed: 2.0 * std::f32::consts::PI / period_seconds,
                phase_offset: 0.0,
                enabled: true,
            },
        );
    }
}
