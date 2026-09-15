use super::*;
use crate::ecs::component::{EditorDisplay, EntityIcon, LightningEffect};
use crate::ecs::resource::{HierarchyState, LightningRenderSettings, PickRay};
use crate::ecs::world::{GlobalTransform, Name, Transform, World};
use cgmath::Vector3;

#[test]
fn spawned_lightning_carries_the_components_the_editor_queries() {
    let mut world = World::new();
    let entity = spawn_lightning(
        &mut world,
        DEFAULT_LIGHTNING_NAME,
        LightningEffect::default(),
    );

    assert_eq!(
        world.get_component::<Name>(entity).map(|n| n.0.clone()),
        Some(DEFAULT_LIGHTNING_NAME.to_string())
    );
    assert!(world.get_component::<Transform>(entity).is_some());
    assert!(world.get_component::<GlobalTransform>(entity).is_some());
    assert!(world.get_component::<LightningEffect>(entity).is_some());
    let display = world.get_component::<EditorDisplay>(entity).unwrap();
    assert_eq!(display.icon, EntityIcon::Lightning);
}

#[test]
fn selected_lightning_wins_over_the_first_lightning() {
    let mut world = World::new();
    let first = spawn_lightning(&mut world, "Lightning 1", LightningEffect::default());
    let second = spawn_lightning(&mut world, "Lightning 2", LightningEffect::default());
    assert_eq!(resolve_selected_lightning(&world), Some(first));

    let mut hierarchy = HierarchyState::default();
    hierarchy.selected_entity = Some(second);
    world.insert_resource(hierarchy);
    assert_eq!(resolve_selected_lightning(&world), Some(second));
}

#[test]
fn pick_ray_hits_the_strike_sphere_and_misses_beside_it() {
    let mut world = World::new();
    let entity = spawn_lightning(&mut world, "Lightning", LightningEffect::default());

    let hit = PickRay {
        origin: Vector3::new(-10.0, -4.0, 0.0),
        direction: Vector3::new(1.0, 0.0, 0.0),
    };
    let (picked, distance) =
        find_lightning_by_pick_ray(&world, &hit).expect("ray enters the bounding sphere");
    assert_eq!(picked, entity);
    assert!(distance > 0.0 && distance < 10.0, "distance {distance}");

    let miss = PickRay {
        origin: Vector3::new(-10.0, 30.0, 0.0),
        direction: Vector3::new(1.0, 0.0, 0.0),
    };
    assert!(find_lightning_by_pick_ray(&world, &miss).is_none());
}

#[test]
fn only_a_known_preset_name_replaces_the_selected_effect() {
    let mut world = World::new();
    let entity = spawn_lightning(&mut world, "Lightning", LightningEffect::default());
    world
        .get_component_mut::<LightningEffect>(entity)
        .expect("lightning effect")
        .core_radius = 0.9;

    apply_lightning_preset_to_selected(&mut world, "no_such_preset");
    assert_eq!(
        world
            .get_component::<LightningEffect>(entity)
            .map(|e| e.core_radius),
        Some(0.9)
    );

    apply_lightning_preset_to_selected(&mut world, LIGHTNING_PRESET_NAMES[0]);
    assert_eq!(
        world
            .get_component::<LightningEffect>(entity)
            .map(|e| e.core_radius),
        Some(LightningEffect::default().core_radius)
    );
}

#[test]
fn default_render_settings_select_closed_form_shading_without_debug_view() {
    let settings = LightningRenderSettings::default();
    let push = LightningPushConstants::new(
        settings.shading_mode.as_shader_value(),
        settings.reference_step_count as i32,
        settings.debug_view.as_shader_value(),
    );

    assert_eq!((push.mode, push.step_count, push.debug_view), (0, 1024, 0));
    assert_eq!(push.as_bytes().len(), 12);
}

#[test]
fn lightning_effect_hook_is_subscribed_after_wind() {
    let mut hooks = crate::hooks::effect::EffectHooks::default();
    crate::effect::subscription::subscribe_effects(&mut hooks);
    let names = hooks.names();

    let wind = names.iter().position(|name| *name == "wind");
    let lightning = names.iter().position(|name| *name == "lightning");
    assert!(
        matches!((wind, lightning), (Some(wind), Some(lightning)) if lightning > wind),
        "hook order {names:?}"
    );
}
