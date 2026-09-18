use super::passes::compute_lightning_scissor;
use super::*;
use crate::ecs::component::{EditorDisplay, EntityIcon, LightningEffect};
use crate::ecs::resource::{HierarchyState, LightningRenderSettings, PickRay, ProjectionData};
use crate::ecs::world::{GlobalTransform, Name, Transform, World};
use cgmath::{Matrix4, SquareMatrix, Vector2, Vector3};
use thyllore_effect_core::{
    build_lightning_ubo, burst_start_time, compute_lightning_segment_aabb, LightningDebugView,
};
use vulkanalia::prelude::v1_0::*;

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

    let preset_name = thyllore_effect_core::LIGHTNING_PRESET_NAMES[0];
    let mut expected = LightningEffect::default();
    assert!(thyllore_effect_core::apply_lightning_preset(
        &mut expected,
        preset_name
    ));

    apply_lightning_preset_to_selected(&mut world, preset_name);
    assert_eq!(
        world
            .get_component::<LightningEffect>(entity)
            .map(|e| e.core_radius),
        Some(expected.core_radius)
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

fn lightning_inside_first_burst() -> LightningEffect {
    let mut effect = LightningEffect::default();
    effect.time = burst_start_time(&effect, 0) + effect.attack_time + effect.sustain_time * 0.5;
    effect
}

fn orthographic_projection(half_size: f32) -> ProjectionData {
    ProjectionData {
        view: Matrix4::identity(),
        proj: cgmath::ortho(-half_size, half_size, -half_size, half_size, -100.0, 100.0),
        screen_size: Vector2::new(200.0, 200.0),
        aspect: 1.0,
    }
}

#[test]
fn scissor_covers_the_projected_segment_aabb() {
    let effect = lightning_inside_first_burst();
    let (ubo, _) = build_lightning_ubo(&effect, Matrix4::identity());
    let extent = vk::Extent2D {
        width: 200,
        height: 200,
    };
    let pixels_per_unit = 10.0;
    let projection = orthographic_projection(extent.width as f32 / 2.0 / pixels_per_unit);

    let scissor = compute_lightning_scissor(
        Some(&projection),
        extent,
        LightningDebugView::Off,
        &effect,
        &ubo,
    )
    .expect("an alive strike is on screen");

    let corners = compute_lightning_segment_aabb(&effect, effect.time).expect("alive segments");
    let to_pixels = |local: f32| local * pixels_per_unit + extent.width as f32 / 2.0;
    let min_x = corners
        .iter()
        .map(|c| to_pixels(c.x))
        .fold(f32::INFINITY, f32::min);
    let max_x = corners
        .iter()
        .map(|c| to_pixels(c.x))
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = corners
        .iter()
        .map(|c| to_pixels(c.y))
        .fold(f32::INFINITY, f32::min);
    let max_y = corners
        .iter()
        .map(|c| to_pixels(c.y))
        .fold(f32::NEG_INFINITY, f32::max);
    let right = scissor.offset.x as f32 + scissor.extent.width as f32;
    let bottom = scissor.offset.y as f32 + scissor.extent.height as f32;

    assert!(
        scissor.offset.x as f32 <= min_x && right >= max_x,
        "{scissor:?}"
    );
    assert!(
        scissor.offset.y as f32 <= min_y && bottom >= max_y,
        "{scissor:?}"
    );
    assert!(
        right - scissor.offset.x as f32 <= max_x - min_x + 6.0,
        "{scissor:?}"
    );
    assert!(
        bottom - scissor.offset.y as f32 <= max_y - min_y + 6.0,
        "{scissor:?}"
    );
    assert!(scissor.extent.width < extent.width, "{scissor:?}");
}

#[test]
fn coverage_debug_view_scissor_spans_the_full_extent() {
    let mut effect = LightningEffect::default();
    effect.time = burst_start_time(&effect, 0) - 1.0;
    let (ubo, _) = build_lightning_ubo(&effect, Matrix4::identity());
    let extent = vk::Extent2D {
        width: 320,
        height: 180,
    };
    let projection = orthographic_projection(10.0);

    assert!(
        compute_lightning_scissor(
            Some(&projection),
            extent,
            LightningDebugView::Off,
            &effect,
            &ubo
        )
        .is_none(),
        "no alive segment leaves nothing to draw"
    );

    let scissor = compute_lightning_scissor(
        Some(&projection),
        extent,
        LightningDebugView::Coverage,
        &effect,
        &ubo,
    )
    .expect("coverage always draws");
    assert_eq!((scissor.offset.x, scissor.offset.y), (0, 0));
    assert_eq!(scissor.extent, extent);
}
