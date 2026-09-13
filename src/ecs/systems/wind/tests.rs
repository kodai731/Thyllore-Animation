use super::*;
use crate::ecs::component::{EditorDisplay, EntityIcon, WindTornadoEffect};
use crate::ecs::resource::{HierarchyState, PickRay, WindRenderSettings};
use crate::ecs::systems::effect_time::{resolve_effect_time, EffectTimeSources, TimelineSample};
use crate::ecs::world::{GlobalTransform, Name, Transform, World};
use cgmath::Vector3;

#[test]
fn spawned_wind_carries_the_components_the_editor_queries() {
    let mut world = World::new();
    let entity = spawn_wind(&mut world, DEFAULT_WIND_NAME, WindTornadoEffect::default());

    assert_eq!(
        world.get_component::<Name>(entity).map(|n| n.0.clone()),
        Some(DEFAULT_WIND_NAME.to_string())
    );
    assert!(world.get_component::<Transform>(entity).is_some());
    assert!(world.get_component::<GlobalTransform>(entity).is_some());
    assert!(world.get_component::<WindTornadoEffect>(entity).is_some());
    let display = world.get_component::<EditorDisplay>(entity).unwrap();
    assert_eq!(display.icon, EntityIcon::Wind);
}

#[test]
fn selected_wind_wins_over_the_first_wind() {
    let mut world = World::new();
    let first = spawn_wind(&mut world, "Wind 1", WindTornadoEffect::default());
    let second = spawn_wind(&mut world, "Wind 2", WindTornadoEffect::default());
    assert_eq!(resolve_selected_wind(&world), Some(first));

    let mut hierarchy = HierarchyState::default();
    hierarchy.selected_entity = Some(second);
    world.insert_resource(hierarchy);
    assert_eq!(resolve_selected_wind(&world), Some(second));
}

#[test]
fn pick_ray_hits_the_wind_envelope_and_misses_beside_it() {
    let mut world = World::new();
    let entity = spawn_wind(&mut world, "Wind", WindTornadoEffect::default());

    let hit = PickRay {
        origin: Vector3::new(-5.0, 1.0, 0.0),
        direction: Vector3::new(1.0, 0.0, 0.0),
    };
    let (picked, distance) = find_wind_by_pick_ray(&world, &hit).expect("ray enters the envelope");
    assert_eq!(picked, entity);
    assert!(distance > 3.0 && distance < 5.0, "distance {distance}");

    let miss = PickRay {
        origin: Vector3::new(-5.0, 1.0, 4.0),
        direction: Vector3::new(1.0, 0.0, 0.0),
    };
    assert!(find_wind_by_pick_ray(&world, &miss).is_none());
}

#[test]
fn batch_fixed_time_wins_over_the_timeline_for_wind() {
    let sources = EffectTimeSources {
        batch_fixed_time: Some(10.5),
        batch_frames_rendered: Some(120.0),
        timeline: Some(TimelineSample {
            current_time: 2.0,
            playing: true,
        }),
        delta_time: 1.0 / 60.0,
        free_run_when_paused: true,
    };

    let mut wind_time = 3.0;
    resolve_effect_time(&mut wind_time, 2.0, 1.5, sources);
    assert_eq!(wind_time, 10.5);
}

#[test]
fn wind_follows_the_timeline_while_paused_by_default() {
    let settings = WindRenderSettings::default();
    let sources = EffectTimeSources {
        batch_fixed_time: None,
        batch_frames_rendered: None,
        timeline: Some(TimelineSample {
            current_time: 2.0,
            playing: false,
        }),
        delta_time: 1.0 / 60.0,
        free_run_when_paused: settings.free_run_when_paused,
    };

    let mut wind_time = 3.0;
    resolve_effect_time(&mut wind_time, 1.0, 0.0, sources);
    assert_eq!(wind_time, 2.0);
}

#[test]
fn wind_scene_roundtrip_keeps_parameters_and_preset() {
    let dir = std::env::temp_dir().join("thyllore_scene_wind_roundtrip");
    let _ = std::fs::remove_dir_all(&dir);
    let scenes_dir = dir.join("scenes");
    std::fs::create_dir_all(&scenes_dir).unwrap();
    let scene_path = scenes_dir.join("test.scene.ron");

    let mut world = crate::scene::world_with_scene_hooks();
    let mut assets = crate::asset::AssetStorage::new();
    let entity = spawn_wind_with_clip(
        &mut world,
        &mut assets,
        DEFAULT_WIND_NAME,
        WindTornadoEffect::default(),
    );
    world
        .get_component_mut::<WindTornadoEffect>(entity)
        .expect("wind effect")
        .column_height = 3.5;
    world.insert_component(
        entity,
        crate::ecs::component::AppliedWindPreset {
            name: "storm".to_string(),
        },
    );

    crate::scene::save_scene(&scene_path, &world).unwrap();
    let loaded = crate::scene::load_scene(&scene_path).unwrap();
    let mut restored = crate::scene::world_with_scene_hooks();
    let mut restored_assets = crate::asset::AssetStorage::new();
    crate::scene::apply_loaded_scene_to_world(&loaded, &mut restored, &mut restored_assets);

    let winds = restored.query_winds();
    assert_eq!(winds.len(), 1);
    let wind = restored
        .get_component::<WindTornadoEffect>(winds[0])
        .unwrap();
    assert_eq!(wind.column_height, 3.5);
    let preset = restored
        .get_component::<crate::ecs::component::AppliedWindPreset>(winds[0])
        .unwrap();
    assert_eq!(preset.name, "storm");
}
