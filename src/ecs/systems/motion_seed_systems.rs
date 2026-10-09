use anyhow::{anyhow, Result};
use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
use thyllore_anim_core::editable::SourceClipId;
use thyllore_avatar_core::motion::seed::components::motion_spec::{MotionSide, MotionSpec};
use thyllore_avatar_core::motion::seed::components::pose_table::{PoseAxis, PoseTable};
use thyllore_avatar_core::motion::seed::systems::compose_motion::{
    compose_motion, settle_requests, SettleRequest,
};

use crate::asset::AssetStorage;
use crate::ecs::resource::{BoneAxis, ClipLibrary, TimelineState};
use crate::ecs::world::World;

pub fn bone_axis_of(axis: PoseAxis) -> BoneAxis {
    match axis {
        PoseAxis::X => BoneAxis::RotationX,
        PoseAxis::Y => BoneAxis::RotationY,
        PoseAxis::Z => BoneAxis::RotationZ,
        PoseAxis::TranslationX => BoneAxis::TranslationX,
        PoseAxis::TranslationY => BoneAxis::TranslationY,
        PoseAxis::TranslationZ => BoneAxis::TranslationZ,
    }
}

pub fn insert_bone_key(
    world: &mut World,
    assets: &mut AssetStorage,
    clip_id: SourceClipId,
    bone_name: &str,
    axis: BoneAxis,
    time: f32,
    value: f32,
) {
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
    curve_add_keyframe(curve, time, value);
    if time > clip.duration {
        clip.duration = time;
    }
    lib.mark_dirty(clip_id);
}

/// Keys the composed motion into `clip_id` and lets the Copilot settle the arrivals it allows.
pub fn compose_into_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    clip_id: SourceClipId,
    spec: &MotionSpec,
) -> Result<()> {
    let keys = compose_motion(PoseTable::builtin(), spec)?;
    for key in &keys {
        insert_bone_key(
            world,
            assets,
            clip_id,
            key.role.unity_name(),
            bone_axis_of(key.axis),
            key.time,
            key.value,
        );
    }
    let requests = settle_requests(PoseTable::builtin(), spec, &keys)?;
    apply_settle_requests(world, assets, clip_id, &requests);
    Ok(())
}

/// Registers a new clip named after the spec, composes into it and makes it the current clip.
pub fn compose_new_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    spec: &MotionSpec,
) -> Result<SourceClipId> {
    if PoseTable::builtin().motion(&spec.motion).is_none() {
        return Err(anyhow!("unknown motion '{}'", spec.motion));
    }
    crate::ecs::systems::humanoid_rig_systems::sync_humanoid_rig(world, assets);
    if crate::ecs::systems::engine_bone_name_to_id(world, assets).is_none() {
        return Err(anyhow!("no rigged model to compose '{}' onto", spec.motion));
    }

    let clip =
        crate::ecs::systems::humanoid_bake_systems::new_empty_clip(&composed_clip_name(spec));
    let clip_id = crate::ecs::systems::clip_library_register_and_activate(
        &mut world.resource_mut::<ClipLibrary>(),
        assets,
        clip,
    );
    world.resource_mut::<TimelineState>().current_clip_id = Some(clip_id);

    compose_into_clip(world, assets, clip_id, spec)?;
    crate::ecs::systems::humanoid_bake_systems::request_bake_scan(world);
    log!("compose_motion: {} -> clip {}", spec, clip_id);
    Ok(clip_id)
}

pub fn composed_clip_name(spec: &MotionSpec) -> String {
    let side = match spec.side {
        MotionSide::Right => "right",
        MotionSide::Left => "left",
    };
    format!("{side}_{}", spec.motion)
}

#[cfg(feature = "ml")]
fn apply_settle_requests(
    world: &mut World,
    assets: &mut AssetStorage,
    clip_id: SourceClipId,
    requests: &[SettleRequest],
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
    _clip_id: SourceClipId,
    requests: &[SettleRequest],
) {
    if !requests.is_empty() {
        log_warn!("compose settle: ml feature is disabled, keys stay as composed");
    }
}
