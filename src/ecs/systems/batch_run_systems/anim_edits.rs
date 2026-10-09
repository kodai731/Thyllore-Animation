use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{
    BakedHumanoidClips, BatchAnimEdit, BoneAxis, ClipLibrary, PendingBatchAnimEdits, TimelineState,
};
use crate::ecs::systems::phases::event_dispatch::clip_instance::ClipInstanceEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::{
    dispatch_scalar_clip_events, ScalarCurveEvent,
};
use crate::ecs::world::World;

use thyllore_anim_core::editable::PropertyType;
use thyllore_avatar_core::motion::seed::components::motion_spec::MotionSpec;
use thyllore_avatar_core::motion::seed::components::pose_table::{PoseAxis, PoseTable};
use thyllore_avatar_core::motion::seed::systems::compose_motion::{
    compose_motion, settle_requests,
};

fn bone_axis_of(axis: PoseAxis) -> BoneAxis {
    match axis {
        PoseAxis::X => BoneAxis::RotationX,
        PoseAxis::Y => BoneAxis::RotationY,
        PoseAxis::Z => BoneAxis::RotationZ,
        PoseAxis::TranslationX => BoneAxis::TranslationX,
        PoseAxis::TranslationY => BoneAxis::TranslationY,
        PoseAxis::TranslationZ => BoneAxis::TranslationZ,
    }
}

#[cfg(feature = "ml")]
fn apply_settle_requests(
    world: &mut World,
    assets: &mut AssetStorage,
    requests: &[thyllore_avatar_core::motion::seed::systems::compose_motion::SettleRequest],
) {
    if requests.is_empty() {
        return;
    }
    let Some(model_path) = crate::ml::resolve_curve_copilot_model_path() else {
        log_warn!("compose settle: curve copilot model not found, keys stay as composed");
        return;
    };
    let mut session =
        match thyllore_ml_core::copilot::v2::inference::V2CurveCopilotSession::from_onnx_path(
            &model_path,
        ) {
            Ok(s) => s,
            Err(e) => {
                log_warn!("compose settle: failed to load model: {}", e);
                return;
            }
        };
    let Some(clip_id) = world.resource::<TimelineState>().current_clip_id else {
        return;
    };
    let Some(table) = crate::ecs::systems::engine_bone_name_to_id(world, assets) else {
        return;
    };

    let mut lib = world.resource_mut::<ClipLibrary>();
    let Some(clip) = lib.get_mut(clip_id) else {
        return;
    };
    for request in requests {
        let Some(&bone_id) = table.get(request.role.unity_name()) else {
            continue;
        };
        let Some(track) = clip.get_track_mut(bone_id) else {
            continue;
        };
        let curve = track.get_curve_mut(request.axis.property_type());
        match crate::ecs::systems::curve_copilot::copilot_settle_curve(
            &mut session,
            curve,
            request.time,
            request.until,
            request.max_frames,
            request.blend,
        ) {
            Ok(count) => log!(
                "compose settle: {}.{:?} @{:.2} added {} keys",
                request.role.unity_name(),
                request.axis,
                request.time,
                count
            ),
            Err(e) => log_warn!("compose settle failed: {}", e),
        }
    }
    lib.mark_dirty(clip_id);
}

#[cfg(not(feature = "ml"))]
fn apply_settle_requests(
    _world: &mut World,
    _assets: &mut AssetStorage,
    requests: &[thyllore_avatar_core::motion::seed::systems::compose_motion::SettleRequest],
) {
    if !requests.is_empty() {
        log_warn!("compose settle: ml feature is disabled, keys stay as composed");
    }
}

fn insert_bone_key(
    world: &mut World,
    assets: &mut AssetStorage,
    bone_name: &str,
    axis: BoneAxis,
    time: f32,
    value: f32,
) {
    let Some(clip_id) = world.resource::<TimelineState>().current_clip_id else {
        return;
    };
    let Some(table) = crate::ecs::systems::engine_bone_name_to_id(world, assets) else {
        log_warn!("key edit: no bone name table available, skipping");
        return;
    };
    let Some(&bone_id) = table.get(bone_name) else {
        log_warn!("key edit: bone {} is not on this model", bone_name);
        return;
    };

    let mut lib = world.resource_mut::<ClipLibrary>();
    let Some(clip) = lib.get_mut(clip_id) else {
        return;
    };
    let track = if let Some(t) = clip.get_track_mut(bone_id) {
        t
    } else {
        clip.add_track(bone_id, bone_name.to_string())
    };
    let curve = match axis {
        BoneAxis::RotationX => &mut track.rotation_x,
        BoneAxis::RotationY => &mut track.rotation_y,
        BoneAxis::RotationZ => &mut track.rotation_z,
        BoneAxis::TranslationX => &mut track.translation_x,
        BoneAxis::TranslationY => &mut track.translation_y,
        BoneAxis::TranslationZ => &mut track.translation_z,
    };
    use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
    curve_add_keyframe(curve, time, value);
    if time > clip.duration {
        clip.duration = time;
    }
    lib.mark_dirty(clip_id);
}

fn remove_clip_by_name(world: &mut World, assets: &mut AssetStorage, name: &str) {
    let source_id = match world.resource::<ClipLibrary>().find_source_by_name(name) {
        Some(id) => id,
        None => return,
    };
    world
        .resource_mut::<ClipLibrary>()
        .source_clips
        .remove(&source_id);
    if let Some(asset_id) = world
        .resource_mut::<ClipLibrary>()
        .source_to_asset_id
        .remove(&source_id)
    {
        assets.animation_clips.remove(&asset_id);
    }
    if let Some(mut baked) = world.get_resource_mut::<BakedHumanoidClips>() {
        let invalidated = baked.invalidate_source(source_id);
        for asset_id in invalidated {
            assets.animation_clips.remove(&asset_id);
        }
    }
}

pub fn batch_apply_pending_anim_edits(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::systems::avatar_setup_systems::find_first_skeleton;
    let edits = match world.remove_resource::<PendingBatchAnimEdits>() {
        Some(resource) => resource.edits,
        None => return,
    };
    if find_first_skeleton(assets).is_some() {
        batch_apply_anim_edits(world, assets, &edits);
    } else {
        world.insert_resource(PendingBatchAnimEdits { edits });
    }
}

pub fn batch_apply_anim_edits(
    world: &mut World,
    assets: &mut AssetStorage,
    edits: &[BatchAnimEdit],
) {
    crate::ecs::systems::humanoid_rig_systems::sync_humanoid_rig(world, assets);

    for edit in edits {
        match edit {
            BatchAnimEdit::DebugKeys { seed } => apply_debug_keys(world, assets, *seed),
            BatchAnimEdit::Key {
                property_type,
                time,
                value,
            } => apply_key(world, assets, *property_type, *time, *value),
            BatchAnimEdit::BoneKey {
                bone_name,
                axis,
                time,
                value,
            } => insert_bone_key(world, assets, bone_name, *axis, *time, *value),
            BatchAnimEdit::Compose { spec } => apply_compose(world, assets, spec),
            BatchAnimEdit::KeyAtPlayhead { property_type } => {
                apply_key_at_playhead(world, assets, *property_type)
            }
            BatchAnimEdit::TrimEnd { seconds } => apply_trim_end(world, assets, *seconds),
            BatchAnimEdit::NewClip { name } => apply_new_clip(world, assets, name),
            BatchAnimEdit::Template { path } => apply_template(world, assets, path),
            BatchAnimEdit::Save { path } => apply_save(world, path),
            BatchAnimEdit::Clear => apply_clear(world, assets),
            BatchAnimEdit::CopilotExtend {
                bone_name,
                axis,
                time,
                frames,
            } => apply_copilot_extend(world, assets, bone_name, *axis, *time, *frames),
        }
    }

    crate::ecs::systems::humanoid_bake_systems::request_bake_scan(world);
}

fn apply_debug_keys(world: &mut World, assets: &mut AssetStorage, seed: u64) {
    dispatch_scalar_clip_events(
        &[ScalarCurveEvent::InsertScalarDebugKeys { seed }],
        world,
        assets,
    );
}

fn apply_key(
    world: &mut World,
    assets: &mut AssetStorage,
    property_type: PropertyType,
    time: f32,
    value: f32,
) {
    let previous_time = {
        let mut timeline = world.resource_mut::<TimelineState>();
        let previous = timeline.current_time;
        timeline.current_time = time;
        previous
    };
    dispatch_scalar_clip_events(
        &[ScalarCurveEvent::InsertScalarKey {
            property_type,
            value,
        }],
        world,
        assets,
    );
    world.resource_mut::<TimelineState>().current_time = previous_time;
}

fn apply_compose(world: &mut World, assets: &mut AssetStorage, spec: &MotionSpec) {
    let keys = match compose_motion(PoseTable::builtin(), spec) {
        Ok(keys) => keys,
        Err(e) => {
            log_warn!("compose {}: {}", spec.motion, e);
            return;
        }
    };
    for key in &keys {
        insert_bone_key(
            world,
            assets,
            key.role.unity_name(),
            bone_axis_of(key.axis),
            key.time,
            key.value,
        );
    }
    match settle_requests(PoseTable::builtin(), spec, &keys) {
        Ok(requests) => apply_settle_requests(world, assets, &requests),
        Err(e) => log_warn!("compose {}: {}", spec.motion, e),
    }
}

fn apply_key_at_playhead(
    world: &mut World,
    assets: &mut AssetStorage,
    property_type: PropertyType,
) {
    dispatch_scalar_clip_events(
        &[ScalarCurveEvent::InsertScalarKeyAtPlayhead { property_type }],
        world,
        assets,
    );
}

fn apply_trim_end(world: &mut World, assets: &mut AssetStorage, seconds: f32) {
    use crate::ecs::systems::scalar_clip_systems::{
        ensure_entity_clip, resolve_selected_scalar_entity,
    };
    let Some((entity, domain)) = resolve_selected_scalar_entity(world) else {
        return;
    };
    let clip_id = ensure_entity_clip(world, assets, entity, domain);
    let Some(instance_id) = world.get_component::<ClipSchedule>(entity).and_then(|s| {
        s.instances
            .iter()
            .find(|i| i.source_id == clip_id)
            .map(|i| i.instance_id)
    }) else {
        return;
    };
    crate::ecs::systems::timeline_systems::process_clip_instance_events(
        &[ClipInstanceEvent::TrimEnd {
            entity,
            instance_id,
            new_clip_out: seconds,
        }],
        world,
    );
}

fn apply_new_clip(world: &mut World, assets: &mut AssetStorage, name: &str) {
    let clip = crate::ecs::systems::humanoid_bake_systems::new_empty_clip(name);
    let id = crate::ecs::systems::clip_library_register_and_activate(
        &mut world.resource_mut::<ClipLibrary>(),
        assets,
        clip,
    );
    world.resource_mut::<TimelineState>().current_clip_id = Some(id);
}

fn apply_template(world: &mut World, assets: &mut AssetStorage, path: &std::path::Path) {
    let mut loaded = match crate::scene::load_animation_clip(path) {
        Ok(c) => c,
        Err(e) => {
            log_warn!("template {} failed: {}", path.display(), e);
            return;
        }
    };
    if let Some(table) = crate::ecs::systems::engine_bone_name_to_id(world, assets) {
        thyllore_anim_core::editable::systems::clip_ops::clip_remap_bone_ids(&mut loaded, &table);
    }
    remove_clip_by_name(world, assets, &loaded.name);
    let id = crate::ecs::systems::clip_library_register_and_activate(
        &mut world.resource_mut::<ClipLibrary>(),
        assets,
        loaded,
    );
    world.resource_mut::<TimelineState>().current_clip_id = Some(id);
}

fn apply_save(world: &mut World, path: &std::path::Path) {
    let current_clip_id = world.resource::<TimelineState>().current_clip_id;
    let Some(clip_id) = current_clip_id else {
        log_warn!("save {}: no current clip", path.display());
        return;
    };
    let lib = world.resource::<ClipLibrary>();
    let Some(clip) = lib.get(clip_id) else {
        log_warn!("save {}: clip id {} not found", path.display(), clip_id);
        return;
    };
    if let Err(e) = crate::scene::save_animation_clip(path, clip) {
        log_warn!("save {} failed: {}", path.display(), e);
    }
}

fn apply_clear(world: &mut World, assets: &mut AssetStorage) {
    dispatch_scalar_clip_events(&[ScalarCurveEvent::ClearScalarKeys], world, assets);
}

#[cfg(feature = "ml")]
fn apply_copilot_extend(
    world: &mut World,
    assets: &mut AssetStorage,
    bone_name: &str,
    axis: BoneAxis,
    time: f32,
    frames: usize,
) {
    if let Err(e) = try_apply_copilot_extend(world, assets, bone_name, axis, time, frames) {
        log_warn!("copilot_extend: {:#}", e);
    }
}

#[cfg(feature = "ml")]
fn try_apply_copilot_extend(
    world: &mut World,
    assets: &mut AssetStorage,
    bone_name: &str,
    axis: BoneAxis,
    time: f32,
    frames: usize,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let Some(clip_id) = world.resource::<TimelineState>().current_clip_id else {
        return Ok(());
    };
    let mut lib = world.resource_mut::<ClipLibrary>();
    let Some(clip) = lib.get_mut(clip_id) else {
        return Ok(());
    };

    let model_path = crate::ml::resolve_curve_copilot_model_path()
        .context("curve copilot model not found, skipping")?;
    let mut session =
        thyllore_ml_core::copilot::v2::inference::V2CurveCopilotSession::from_onnx_path(
            &model_path,
        )
        .context("failed to load model")?;
    let table = crate::ecs::systems::engine_bone_name_to_id(world, assets)
        .context("no bone name table available, skipping")?;
    let &bone_id = table
        .get(bone_name)
        .with_context(|| format!("bone {} is not on this model", bone_name))?;

    let track = if let Some(t) = clip.get_track_mut(bone_id) {
        t
    } else {
        clip.add_track(bone_id, bone_name.to_string())
    };

    let curve = match axis {
        BoneAxis::RotationX => &mut track.rotation_x,
        BoneAxis::RotationY => &mut track.rotation_y,
        BoneAxis::RotationZ => &mut track.rotation_z,
        BoneAxis::TranslationX => &mut track.translation_x,
        BoneAxis::TranslationY => &mut track.translation_y,
        BoneAxis::TranslationZ => &mut track.translation_z,
    };

    match crate::ecs::systems::curve_copilot::copilot_extend_curve(
        &mut session,
        curve,
        time,
        frames,
    ) {
        Ok(count) => {
            log!("copilot_extend: added {} keys", count);
            let last_time = curve.keyframes.last().map(|k| k.time).unwrap_or(0.0);
            if last_time > clip.duration {
                clip.duration = last_time;
            }
            lib.mark_dirty(clip_id);
        }
        Err(e) => log_warn!("copilot_extend failed: {}", e),
    }
    Ok(())
}

#[cfg(not(feature = "ml"))]
fn apply_copilot_extend(
    _world: &mut World,
    _assets: &mut AssetStorage,
    _bone_name: &str,
    _axis: BoneAxis,
    _time: f32,
    _frames: usize,
) {
    log_warn!("copilot_extend: ml feature is disabled, skipping");
}
