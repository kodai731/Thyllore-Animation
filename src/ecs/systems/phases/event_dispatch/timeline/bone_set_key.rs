use crate::asset::AssetStorage;
use crate::ecs::resource::gizmo::BoneGizmoData;
use crate::ecs::resource::{BonePoseOverride, ClipLibrary, HumanoidRigState, TimelineState};
use crate::ecs::systems::process_bone_set_key;
use crate::ecs::world::World;

use super::history::ClipSnapshot;
use super::TimelineEvent;

pub(super) fn dispatch_bone_set_key_events(
    events: &[TimelineEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    if !events
        .iter()
        .any(|e| matches!(e, TimelineEvent::BoneSetKey))
    {
        return;
    }

    let overrides = match world.get_resource::<BonePoseOverride>() {
        Some(r) => r.overrides.clone(),
        None => return,
    };
    if overrides.is_empty() {
        return;
    }

    let skeleton_id = world
        .get_resource::<BoneGizmoData>()
        .and_then(|bg| bg.cached_skeleton_id);
    let Some(skel_id) = skeleton_id else { return };
    let Some(skeleton) = assets.get_skeleton_by_skeleton_id(skel_id) else {
        return;
    };
    let skeleton = skeleton.clone();

    let snapshot;
    let modified = {
        let timeline_state = world.resource::<TimelineState>();
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let rig_state = world.resource::<HumanoidRigState>();
        snapshot = ClipSnapshot::capture(&timeline_state, &clip_library);
        process_bone_set_key(
            &overrides,
            &mut clip_library,
            &timeline_state,
            &skeleton,
            rig_state.rig.as_ref(),
        )
    };
    if !modified {
        return;
    }

    if let Some(snapshot) = snapshot {
        snapshot.record(world, "bone set key");
    }
    if let Some(mut pose_overrides) = world.get_resource_mut::<BonePoseOverride>() {
        pose_overrides.clear();
    }
}
