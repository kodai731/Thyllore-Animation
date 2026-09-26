use crate::asset::AssetStorage;
use crate::ecs::component::{
    apply_flame_param_value, AppliedFlameStyle, EntityIcon, FlameBaked, FlameBoneAttachment,
    FlameEffect, FlameParam, FlameTemporalAccum, FlameTrail, FLAME_DOMAIN,
};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{
    BatchRun, ClipLibrary, FlameHistorySnapshot, FlameHistorySnapshotState, FlameRenderSettings,
    FlameWallProbeCapture, HierarchyState, LightState, ProjectionData, TimelineState,
};
use crate::ecs::systems::{resolve_engine_cli_overrides, EngineCliOverrides};
use crate::ecs::world::{Entity, Transform, World};
use crate::ecs::FrameContext;
use thyllore_effect_core::{advance_flame_time, advance_flame_trail};

use super::*;
use crate::ecs::component::EditorDisplay;
use crate::ecs::resource::PickRay;
use crate::ecs::systems::object_picking_systems::resolve_closest_pick;
use crate::ecs::world::{GlobalTransform, Name};
use crate::hooks::pick::PickHooks;
use cgmath::Vector3;

fn spawn_default_flame(world: &mut World, name: &str) -> Entity {
    spawn_flame(world, name, FlameEffect::default())
}

#[test]
fn spawned_flame_carries_the_components_the_editor_queries() {
    let mut world = World::new();
    let entity = spawn_default_flame(&mut world, DEFAULT_FLAME_NAME);

    assert_eq!(
        world.get_component::<Name>(entity).map(|n| n.0.clone()),
        Some(DEFAULT_FLAME_NAME.to_string())
    );
    assert!(world.get_component::<Transform>(entity).is_some());
    assert!(world.get_component::<GlobalTransform>(entity).is_some());
    assert!(world.get_component::<EditorDisplay>(entity).is_some());
    assert!(world.get_component::<FlameEffect>(entity).is_some());
}

#[test]
fn spawned_flame_appears_as_a_hierarchy_root() {
    let mut world = World::new();
    let entity = spawn_default_flame(&mut world, DEFAULT_FLAME_NAME);

    assert!(world.get_root_entities().contains(&entity));
}

#[test]
fn spawn_mirrors_the_effect_position_onto_the_transform() {
    let mut world = World::new();
    let effect = FlameEffect {
        position: cgmath::Vector3::new(1.5, 2.5, -3.5),
        ..FlameEffect::default()
    };
    let entity = spawn_flame(&mut world, "Flame 2", effect);

    let transform = world.get_component::<Transform>(entity).unwrap();
    assert_eq!(transform.translation, cgmath::Vector3::new(1.5, 2.5, -3.5));
}

#[test]
fn selection_falls_back_to_the_first_flame_when_nothing_is_selected() {
    let mut world = World::new();
    world.insert_resource(HierarchyState::default());
    let first = spawn_default_flame(&mut world, "Flame 1");
    spawn_default_flame(&mut world, "Flame 2");

    assert_eq!(resolve_selected_flame(&world), Some(first));
}

#[test]
fn selection_follows_the_hierarchy_when_a_flame_is_selected() {
    let mut world = World::new();
    world.insert_resource(HierarchyState::default());
    spawn_default_flame(&mut world, "Flame 1");
    let second = spawn_default_flame(&mut world, "Flame 2");

    world
        .get_resource_mut::<HierarchyState>()
        .unwrap()
        .selected_entity = Some(second);

    assert_eq!(resolve_selected_flame(&world), Some(second));
}

#[test]
fn selecting_a_non_flame_entity_keeps_editing_the_first_flame() {
    let mut world = World::new();
    world.insert_resource(HierarchyState::default());
    let first = spawn_default_flame(&mut world, "Flame 1");
    let other = world.entity().with_name("Not a flame").build();

    world
        .get_resource_mut::<HierarchyState>()
        .unwrap()
        .selected_entity = Some(other);

    assert_eq!(resolve_selected_flame(&world), Some(first));
}

#[test]
fn apply_effect_update_moves_the_flame_transform() {
    let mut world = World::new();
    let entity = spawn_default_flame(&mut world, DEFAULT_FLAME_NAME);
    let translation = cgmath::Vector3::new(4.0, 0.0, 2.0);
    let effect = FlameEffect {
        position: translation,
        rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
        ..FlameEffect::default()
    };

    crate::ecs::systems::apply_effect_update(&mut world, entity, effect);

    let transform = world.get_component::<Transform>(entity).unwrap();
    assert_eq!(transform.translation, translation);
}

#[test]
fn resolve_returns_none_without_any_flame() {
    let mut world = World::new();
    world.insert_resource(HierarchyState::default());

    assert_eq!(resolve_selected_flame(&world), None);
}

fn flame_with_keyed_clip(world: &mut World, assets: &mut AssetStorage) -> Entity {
    use thyllore_anim_core::editable::{curve_add_keyframe, InterpolationType};

    let entity = spawn_flame_with_clip(world, assets, DEFAULT_FLAME_NAME, FlameEffect::default());
    let clip_id = crate::ecs::systems::find_entity_clip_id(world, entity).expect("flame clip");
    let mut library = world.resource_mut::<ClipLibrary>();
    let clip = library.get_mut(clip_id).expect("clip registered");
    let curve = clip.get_or_add_scalar_curve(FlameParam::Height.property_type());
    let key = curve_add_keyframe(curve, 1.0, 2.0);
    curve
        .get_keyframe_mut(key)
        .expect("key inserted")
        .interpolation = InterpolationType::Bezier;
    entity
}

#[test]
fn scene_entities_restore_the_flame_style_and_its_keyed_clip() {
    use thyllore_anim_core::editable::InterpolationType;

    let mut source = crate::scene::world_with_scene_hooks();
    let mut source_assets = AssetStorage::new();
    let flame = flame_with_keyed_clip(&mut source, &mut source_assets);
    source
        .get_component_mut::<FlameEffect>(flame)
        .expect("flame effect")
        .height = 7.5;
    source.insert_component(
        flame,
        AppliedFlameStyle {
            name: "pillar".to_string(),
            version: 3,
        },
    );
    let entities = crate::scene::capture_scene_entities(&source);
    let clips = crate::scene::capture_scheduled_clips(&source);
    assert_eq!(clips.len(), 1);
    assert!(entities[0].components.contains_key("clip"));
    assert!(entities[0].components.contains_key("flame_style"));

    let mut world = crate::scene::world_with_scene_hooks();
    let mut assets = AssetStorage::new();
    crate::ecs::systems::clip_library_systems::clip_library_register_loaded(
        &mut world,
        &mut assets,
        clips,
    );
    crate::scene::apply_scene_entities(&mut world, &mut assets, &entities);

    let flames = world.entities_with::<FlameEffect>();
    assert_eq!(flames.len(), 1);
    assert_eq!(
        world
            .get_component::<FlameEffect>(flames[0])
            .map(|e| e.height),
        Some(7.5)
    );
    let style = world
        .get_component::<AppliedFlameStyle>(flames[0])
        .expect("style restored");
    assert_eq!((style.name.as_str(), style.version), ("pillar", 3));

    let clip_id = crate::ecs::systems::find_entity_clip_id(&world, flames[0]).expect("clip");
    let library = world.resource::<ClipLibrary>();
    let curve = library
        .get(clip_id)
        .expect("scheduled clip is in the library")
        .get_scalar_curve(FlameParam::Height.property_type())
        .expect("keyed curve restored");
    assert_eq!(curve.keyframes.len(), 1);
    assert_eq!(curve.keyframes[0].interpolation, InterpolationType::Bezier);
    assert_eq!(library.source_clips.len(), 1, "no orphan clip");
}

#[test]
fn reloading_scene_entities_replaces_the_flame_and_its_clip() {
    let mut source = crate::scene::world_with_scene_hooks();
    let mut source_assets = AssetStorage::new();
    spawn_flame_with_clip(
        &mut source,
        &mut source_assets,
        DEFAULT_FLAME_NAME,
        FlameEffect::default(),
    );
    let entities = crate::scene::capture_scene_entities(&source);

    let mut world = crate::scene::world_with_scene_hooks();
    let mut assets = AssetStorage::new();
    crate::scene::apply_scene_entities(&mut world, &mut assets, &entities);
    let first = world.entities_with::<FlameEffect>()[0];
    assert!(crate::ecs::systems::find_entity_clip_id(&world, first).is_some());
    crate::scene::apply_scene_entities(&mut world, &mut assets, &entities);

    let flames = world.entities_with::<FlameEffect>();
    assert_eq!(flames.len(), 1);
    assert_ne!(flames[0], first);
    assert_eq!(world.resource::<ClipLibrary>().source_clips.len(), 1);
}

fn wall_probe_overrides(extra: &[&str]) -> EngineCliOverrides {
    let mut args: Vec<String> = vec!["bin".to_string()];
    args.extend(extra.iter().map(|s| s.to_string()));
    args.push("--batch-debug-action".to_string());
    args.push("dump_wall_probe".to_string());
    resolve_engine_cli_overrides(&args).expect("engine overrides parse")
}

#[test]
fn a_batch_run_defers_the_wall_probe_dump_to_the_capture_frame() {
    let overrides = wall_probe_overrides(&["--batch-screenshot", "/tmp/out.png"]);
    let mut world = World::new();
    crate::ecs::systems::apply_engine_overrides(&mut world, &mut AssetStorage::new(), &overrides);

    assert!(world.contains_resource::<BatchRun>());
    assert!(world.contains_resource::<FlameWallProbeCapture>());
}

#[test]
fn without_a_batch_run_the_wall_probe_dump_goes_through_the_event_queue() {
    let overrides = wall_probe_overrides(&[]);
    let mut world = World::new();
    world.insert_resource(UIEventQueue::new());
    crate::ecs::systems::apply_engine_overrides(&mut world, &mut AssetStorage::new(), &overrides);

    assert!(world.get_resource::<BatchRun>().is_none());
    assert!(world.get_resource::<FlameWallProbeCapture>().is_none());

    let events: Vec<UIEvent> = world.resource_mut::<UIEventQueue>().drain().collect();
    assert!(matches!(events[0], UIEvent::CaptureNow(_)));
}

#[test]
fn engine_overrides_carry_no_flame_subsystem_flags() {
    let overrides = resolve_engine_cli_overrides(&[
        "bin".to_string(),
        "--batch-screenshot".to_string(),
        "/tmp/out.png".to_string(),
        "--batch-flame-mode".to_string(),
        "raymarch".to_string(),
        "--batch-play".to_string(),
    ])
    .unwrap();
    assert!(overrides.batch_run.is_some());
    assert!(overrides.batch_play);
    assert!(overrides.debug_actions.is_empty());
}

const RAY_START_Z: f32 = -10.0;

fn world_with_flame_at(x: f32) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(PickHooks::collect().expect("pick hooks"));
    let effect = FlameEffect {
        position: Vector3::new(x, 0.0, 0.0),
        ..FlameEffect::default()
    };
    let entity = spawn_flame(&mut world, "Flame", effect);
    (world, entity)
}

fn ray_towards_origin() -> PickRay {
    PickRay {
        origin: Vector3::new(0.0, 0.5, RAY_START_Z),
        direction: Vector3::new(0.0, 0.0, 1.0),
    }
}

#[test]
fn a_ray_through_the_flame_finds_it() {
    let (world, flame) = world_with_flame_at(0.0);

    let hit = find_flame_by_pick_ray(&world, &ray_towards_origin());

    assert_eq!(hit.map(|(entity, _)| entity), Some(flame));
}

#[test]
fn a_ray_beside_the_flame_finds_nothing() {
    let (world, _) = world_with_flame_at(50.0);

    assert!(find_flame_by_pick_ray(&world, &ray_towards_origin()).is_none());
}

#[test]
fn the_nearest_flame_wins_when_two_overlap() {
    let (mut world, far_flame) = world_with_flame_at(0.0);
    let near_effect = FlameEffect {
        position: Vector3::new(0.0, 0.0, -5.0),
        ..FlameEffect::default()
    };
    let near_flame = spawn_flame(&mut world, "Flame 2", near_effect);

    let hit = find_flame_by_pick_ray(&world, &ray_towards_origin());

    assert_eq!(hit.map(|(entity, _)| entity), Some(near_flame));
    assert_ne!(hit.map(|(entity, _)| entity), Some(far_flame));
}

#[test]
fn a_surface_in_front_of_the_flame_wins() {
    let (mut world, _) = world_with_flame_at(0.0);
    let ray = ray_towards_origin();
    let surface = world.entity().with_name("mesh").build();
    let in_front_of_the_flame = [0.0, 0.5, -5.0];

    let picked = resolve_closest_pick(
        &world,
        Some(surface),
        Some(&ray),
        Some(in_front_of_the_flame),
    );

    assert_eq!(picked, Some(surface));
}

#[test]
fn a_flame_in_front_of_the_surface_wins() {
    let (mut world, flame) = world_with_flame_at(0.0);
    let ray = ray_towards_origin();
    let surface = world.entity().with_name("mesh").build();
    let behind_the_flame = [0.0, 0.5, 5.0];

    let picked = resolve_closest_pick(&world, Some(surface), Some(&ray), Some(behind_the_flame));

    assert_eq!(picked, Some(flame));
}

#[test]
fn a_flame_over_the_background_is_picked() {
    let (world, flame) = world_with_flame_at(0.0);

    let picked = resolve_closest_pick(&world, None, Some(&ray_towards_origin()), None);

    assert_eq!(picked, Some(flame));
}

#[test]
fn without_a_ray_the_surface_decides() {
    let (mut world, _) = world_with_flame_at(0.0);
    let surface = world.entity().with_name("mesh").build();

    assert_eq!(
        resolve_closest_pick(&world, Some(surface), None, None),
        Some(surface)
    );
}

#[test]
fn clicking_empty_space_selects_nothing() {
    let (world, _) = world_with_flame_at(50.0);

    assert_eq!(
        resolve_closest_pick(&world, None, Some(&ray_towards_origin()), None),
        None
    );
}
