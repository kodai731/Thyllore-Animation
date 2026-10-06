use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{ClipLibrary, HumanoidRigState, TimelineState};
use crate::ecs::systems::avatar_setup_systems::find_first_skeleton;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Debug)]
pub struct EnsureBoneTrack {
    pub bone_id: BoneId,
}

impl UiCommand for EnsureBoneTrack {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        let Some(clip_id) = world.resource::<TimelineState>().current_clip_id else {
            return;
        };
        let needs_track = world
            .resource::<ClipLibrary>()
            .get(clip_id)
            .is_some_and(|clip| !clip.tracks.contains_key(&self.bone_id));
        if !needs_track {
            return;
        }

        let Some(bone_name) = resolve_track_name(world, assets, self.bone_id) else {
            return;
        };

        let mut clip_library = world.resource_mut::<ClipLibrary>();
        if let Some(clip) = clip_library.get_mut(clip_id) {
            clip.add_track(self.bone_id, bone_name);
            clip_library.mark_dirty(clip_id);
        }
    }
}

fn resolve_track_name(world: &World, assets: &AssetStorage, bone_id: BoneId) -> Option<String> {
    let rig_track_name = world
        .get_resource::<HumanoidRigState>()
        .and_then(|state| state.rig.as_ref()?.track_names.get(&bone_id).cloned());

    rig_track_name.or_else(|| {
        find_first_skeleton(assets)?
            .bones
            .get(bone_id as usize)
            .map(|bone| bone.name.clone())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

    use crate::animation::editable::{EditableAnimationClip, SourceClip, SourceClipId};
    use crate::ecs::systems::humanoid_rig_systems::{
        build_humanoid_rig, copy_test_humanoid_fixture, test_humanoid_world,
    };

    #[test]
    fn applying_to_a_clip_without_the_track_creates_it_with_the_role_name() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets)
            .expect("fixture has a skeleton")
            .clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton).expect("fixture is humanoid");
        let head_bone_id = rig.mapping.by_role[&HumanoidRole::Head] as BoneId;
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        let clip_id: SourceClipId = 1;
        let mut clip_library = ClipLibrary::new();
        clip_library.source_clips.insert(
            clip_id,
            SourceClip::new(
                clip_id,
                EditableAnimationClip::new(clip_id, "empty".to_string()),
            ),
        );
        world.insert_resource(clip_library);
        world.insert_resource(TimelineState {
            current_clip_id: Some(clip_id),
            ..Default::default()
        });

        Box::new(EnsureBoneTrack {
            bone_id: head_bone_id,
        })
        .apply(&mut world, &mut assets, &GraphicsResources::default());

        let clip_library = world.resource::<ClipLibrary>();
        let track = clip_library
            .get(clip_id)
            .and_then(|clip| clip.get_track(head_bone_id))
            .expect("track should be created");
        assert_eq!(track.bone_name, HumanoidRole::Head.unity_name());
    }
}
