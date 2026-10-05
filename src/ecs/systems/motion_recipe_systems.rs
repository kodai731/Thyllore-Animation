use std::path::Path;

use anyhow::Context;
use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use crate::animation::editable::{ClipInstanceId, SourceClipId};
use crate::animation::{BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::component::{AnimationMeta, ClipSchedule};
use crate::ecs::resource::{
    AnimationType, ClipLibrary, HierarchyState, RecipeClipSource, RecipeClipSources, TimelineState,
};
use crate::ecs::systems::clip_library_systems::{
    clip_library_register_and_activate, find_clip_schedule_owner,
};
use crate::ecs::systems::clip_schedule_systems::{
    clip_schedule_add_instance, clip_schedule_remove_instance, find_preview_owner,
};
use crate::ecs::systems::role_clip_systems::{
    baked_motion_to_clip, build_role_retarget_context, resolve_model_rig,
};
use crate::ecs::world::World;

pub fn recipe_to_clip(
    recipe_json: &str,
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
) -> anyhow::Result<EditableAnimationClip> {
    let recipe = thyllore_avatar_core::motion::systems::recipe_io::parse_recipe(recipe_json)?;

    let ctx = build_role_retarget_context(skeleton, mapping)?;

    let curves = thyllore_avatar_core::motion::systems::recipe_curves::build_recipe_curves(&recipe);
    let baked = thyllore_avatar_core::motion::systems::bake::bake_recipe_motion(&ctx, &curves);

    let hips_bone = *mapping
        .by_role
        .get(&HumanoidRole::Hips)
        .ok_or_else(|| anyhow::anyhow!("mapping has no Hips bone"))?;

    Ok(baked_motion_to_clip(
        &baked,
        skeleton,
        hips_bone,
        &recipe.name,
    ))
}

pub fn apply_recipe_file(
    world: &mut World,
    assets: &mut AssetStorage,
    path: &Path,
) -> anyhow::Result<SourceClipId> {
    let (skeleton, mapping) =
        resolve_model_rig(world, assets).context("no skeleton or model loaded")?;
    let recipe_json = std::fs::read_to_string(path).context("cannot read recipe")?;
    let recipe = thyllore_avatar_core::motion::systems::recipe_io::parse_recipe(&recipe_json)?;
    let pose_times: Vec<f32> = recipe.poses.iter().map(|p| p.time).collect();
    let mut roles: Vec<(BoneId, HumanoidRole)> = mapping
        .by_role
        .iter()
        .map(|(role, &bone_index)| (bone_index as BoneId, *role))
        .collect();
    roles.sort_by_key(|(_, role)| *role);

    let clip = recipe_to_clip(&recipe_json, &skeleton, &mapping)?;

    let clip_name = clip.name.clone();
    let duration = clip.duration;

    let replaced_id = world
        .resource::<ClipLibrary>()
        .find_source_by_name(&clip_name);
    if let Some(replaced_id) = replaced_id {
        remove_recipe_clip(world, replaced_id);
    }

    let source_id =
        clip_library_register_and_activate(&mut world.resource_mut::<ClipLibrary>(), assets, clip);

    register_recipe_source(
        world,
        source_id,
        RecipeClipSource {
            path: path.to_path_buf(),
            pose_times,
            roles,
            detached: false,
            pose_rotations: recipe.poses.iter().map(|p| p.rotations.clone()).collect(),
        },
    );

    if let Some(ref mut timeline) = world.get_resource_mut::<TimelineState>() {
        timeline.current_clip_id = Some(source_id);
    }

    let schedule =
        find_preview_owner(world).and_then(|owner| world.get_component_mut::<ClipSchedule>(owner));
    let Some(schedule) = schedule else {
        log_warn!(
            "recipe {}: clip '{clip_name}' registered but no model clip schedule was found",
            path.display()
        );
        return Ok(source_id);
    };
    let instance_id = clip_schedule_add_instance(schedule, source_id, duration);
    mute_overlapping_instances(schedule, instance_id);
    log!(
        "recipe {}: clip '{clip_name}' (src {source_id}) scheduled",
        path.display()
    );

    Ok(source_id)
}

fn mute_overlapping_instances(schedule: &mut ClipSchedule, keep: ClipInstanceId) {
    let keep_instance = schedule.instances.iter().find(|i| i.instance_id == keep);
    let Some(keep_instance) = keep_instance else {
        return;
    };
    let keep_end = keep_instance.end_time();
    let keep_start = keep_instance.start_time;

    let mut muted_sources: Vec<SourceClipId> = Vec::new();

    for instance in &mut schedule.instances {
        if instance.instance_id == keep {
            continue;
        }
        if instance.muted {
            continue;
        }
        let inst_end = instance.end_time();
        if instance.start_time < keep_end && keep_start < inst_end {
            let source_id = instance.source_id;
            instance.muted = true;
            muted_sources.push(source_id);
        }
    }

    if !muted_sources.is_empty() {
        log!(
            "recipe: muted {} overlapping instance(s) (source ids: {:?})",
            muted_sources.len(),
            muted_sources
        );
    }
}

fn remove_clip_instances(world: &mut World, source_id: SourceClipId) {
    for entity in world.component_entities::<ClipSchedule>() {
        let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) else {
            continue;
        };
        let instance_ids: Vec<_> = schedule
            .instances
            .iter()
            .filter(|instance| instance.source_id == source_id)
            .map(|instance| instance.instance_id)
            .collect();
        for instance_id in instance_ids {
            clip_schedule_remove_instance(schedule, instance_id);
        }
    }
}

fn remove_recipe_clip(world: &mut World, clip: SourceClipId) {
    world.resource_mut::<ClipLibrary>().remove(clip);
    world
        .resource_mut::<ClipLibrary>()
        .source_to_asset_id
        .remove(&clip);
    remove_clip_instances(world, clip);
    if let Some(ref mut recipe_sources) = world.get_resource_mut::<RecipeClipSources>() {
        recipe_sources.by_clip.remove(&clip);
    }
}

fn register_recipe_source(world: &mut World, source_id: SourceClipId, source: RecipeClipSource) {
    if !world.contains_resource::<RecipeClipSources>() {
        world.insert_resource(RecipeClipSources::default());
    }
    let mut recipe_sources = world.resource_mut::<RecipeClipSources>();
    recipe_sources.by_clip.insert(source_id, source);
}

pub fn set_recipe_pose_rotation(
    world: &mut World,
    assets: &mut AssetStorage,
    clip: SourceClipId,
    role: HumanoidRole,
    pose_index: usize,
    euler: [f32; 3],
) -> anyhow::Result<SourceClipId> {
    let path = find_recipe_path(world, clip)?;
    let recipe_json = std::fs::read_to_string(&path).context("cannot read recipe")?;
    let mut recipe = thyllore_avatar_core::motion::systems::recipe_io::parse_recipe(&recipe_json)?;
    let pose_count = recipe.poses.len();
    let pose = recipe
        .poses
        .get_mut(pose_index)
        .with_context(|| format!("pose index {pose_index} out of range ({pose_count} poses)"))?;
    pose.rotations.insert(role, euler);

    let edited_json = serde_json::to_string_pretty(&recipe)?;
    std::fs::write(&path, edited_json).context("cannot write recipe")?;

    apply_recipe_file(world, assets, &path)
}

fn find_recipe_path(world: &World, clip: SourceClipId) -> anyhow::Result<std::path::PathBuf> {
    let recipe_sources = world
        .get_resource::<RecipeClipSources>()
        .context("no recipe clips registered")?;
    let source = recipe_sources
        .by_clip
        .get(&clip)
        .with_context(|| format!("clip {clip} has no recipe source"))?;
    if source.detached {
        anyhow::bail!("clip {clip} is detached from its recipe");
    }
    Ok(source.path.clone())
}

pub fn detach_recipe_clip(world: &mut World, clip: SourceClipId) {
    let Some(mut recipe_sources) = world.get_resource_mut::<RecipeClipSources>() else {
        return;
    };
    let Some(source) = recipe_sources.by_clip.get_mut(&clip) else {
        return;
    };
    if source.detached {
        return;
    }
    source.detached = true;
    drop(recipe_sources);

    if let Some(editable_clip) = world.resource_mut::<ClipLibrary>().get_mut(clip) {
        editable_clip.name.push_str(" (edited)");
    }
}

pub fn rebake_recipe_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    clip: SourceClipId,
) -> anyhow::Result<SourceClipId> {
    let path = world
        .get_resource::<RecipeClipSources>()
        .and_then(|recipe_sources| recipe_sources.by_clip.get(&clip).map(|s| s.path.clone()))
        .with_context(|| format!("clip {clip} has no recipe source"))?;

    remove_recipe_clip(world, clip);

    apply_recipe_file(world, assets, &path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::asset::storage::{AssetStorage, SkeletonAsset};
    use crate::ecs::component::{AnimationMeta, ClipSchedule};
    use crate::ecs::resource::{AnimationType, HierarchyState, ModelState};
    use crate::ecs::systems::avatar_setup_systems::skeleton_to_bone_inputs;
    use crate::ecs::systems::role_clip_systems::load_mixamo_fixture_skeleton;
    use crate::ecs::world::Entity;
    use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;

    static FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn make_recipe_world(schedule_owner_count: usize) -> (World, AssetStorage) {
        let mut world = World::new();
        let temp_dir = std::env::temp_dir();
        let fake_model_path = temp_dir
            .join("recipe_test_model.fbx")
            .to_string_lossy()
            .to_string();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(HierarchyState::default());
        world.insert_resource(ModelState {
            model_path: fake_model_path,
            ..Default::default()
        });
        for _ in 0..schedule_owner_count {
            world
                .entity()
                .with_clip_schedule(ClipSchedule::new())
                .with_animation_meta(AnimationMeta {
                    animation_type: AnimationType::Skeletal,
                    node_animation_scale: 1.0,
                })
                .build();
        }
        let mut assets = AssetStorage::new();
        let skeleton = load_mixamo_fixture_skeleton();
        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: 0,
            skeleton,
        });
        (world, assets)
    }

    #[test]
    fn test_recipe_to_clip_wave() {
        use std::fs;

        let skeleton = load_mixamo_fixture_skeleton();

        let recipe_path = wave_recipe_fixture_path();

        let bones = skeleton_to_bone_inputs(&skeleton);
        let (mapping, _) = infer_mapping(&bones);

        let recipe_json = fs::read_to_string(recipe_path).expect("Failed to read recipe");

        let clip = recipe_to_clip(&recipe_json, &skeleton, &mapping).unwrap();

        assert_eq!(clip.name, "wave_right_hand");
        assert!(
            (clip.duration - 2.8).abs() < 1e-4,
            "duration={:.4}, expected 2.8",
            clip.duration
        );

        let right_lower_arm_bone_idx = mapping
            .by_role
            .get(&HumanoidRole::RightLowerArm)
            .expect("RightLowerArm not found in mapping");
        let track = clip
            .get_track(*right_lower_arm_bone_idx as BoneId)
            .expect("RightLowerArm track not found");

        let key_count = track.rotation_x.keyframes.len();
        assert!(
            key_count == 85,
            "RightLowerArm rotation key count={}, expected 85 (2.8 * 30 + 1)",
            key_count
        );

        let hips_bone_idx = mapping
            .by_role
            .get(&HumanoidRole::Hips)
            .expect("Hips not found in mapping");
        let hips_track = clip
            .get_track(*hips_bone_idx as BoneId)
            .expect("Hips track not found");

        assert!(
            !hips_track.translation_x.keyframes.is_empty(),
            "Hips translation keys are empty"
        );
    }

    #[test]
    fn test_apply_recipe_file_without_selection_schedules_on_sole_model() {
        let (mut world, mut assets) = make_recipe_world(1);

        let recipe_path = wave_recipe_fixture_path();

        let id = apply_recipe_file(&mut world, &mut assets, &recipe_path).unwrap();

        let entities: Vec<Entity> = world.component_entities::<ClipSchedule>();
        assert_eq!(entities.len(), 1, "expected exactly 1 ClipSchedule entity");

        let schedule = world.get_component::<ClipSchedule>(entities[0]).unwrap();
        assert_eq!(schedule.instances.len(), 1, "expected exactly 1 instance");
        assert_eq!(
            schedule.instances[0].source_id, id,
            "instance source_id should match the returned clip id"
        );
    }

    #[test]
    fn test_apply_recipe_file_twice_replaces_clip() {
        let (mut world, mut assets) = make_recipe_world(1);

        let recipe_path = wave_recipe_fixture_path();

        apply_recipe_file(&mut world, &mut assets, &recipe_path).unwrap();
        apply_recipe_file(&mut world, &mut assets, &recipe_path).unwrap();

        let clip_library = world.resource::<ClipLibrary>();
        assert_eq!(
            clip_library.source_clips.len(),
            1,
            "expected exactly 1 clip after 2 applies"
        );

        let entities: Vec<Entity> = world.component_entities::<ClipSchedule>();
        let schedule = world.get_component::<ClipSchedule>(entities[0]).unwrap();
        assert_eq!(
            schedule.instances.len(),
            1,
            "expected exactly 1 instance after 2 applies"
        );

        let recipe_sources = world.resource::<RecipeClipSources>();
        assert_eq!(
            recipe_sources.by_clip.len(),
            1,
            "expected exactly 1 entry in RecipeClipSources.by_clip"
        );

        let source_id = schedule.instances[0].source_id;
        let clip_source = recipe_sources
            .by_clip
            .get(&source_id)
            .expect("clip source not found");
        assert!(
            clip_source.path.ends_with("wave.json"),
            "path should end with wave.json, got {:?}",
            clip_source.path
        );
        assert_eq!(
            clip_source.pose_times,
            [0.0, 0.4, 0.8, 2.8],
            "pose_times should match recipe poses"
        );
        let has_right_lower_arm = clip_source
            .roles
            .iter()
            .any(|(_, role)| *role == HumanoidRole::RightLowerArm);
        assert!(has_right_lower_arm, "roles should contain RightLowerArm");
    }

    fn wave_recipe_fixture_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/recipies/wave.json")
    }

    fn copy_wave_recipe_to_temp() -> std::path::PathBuf {
        let fixture_path = wave_recipe_fixture_path();
        let counter = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let copy_path = std::env::temp_dir().join(format!(
            "motion_recipe_wave_{}_{}.json",
            std::process::id(),
            counter
        ));
        std::fs::copy(&fixture_path, &copy_path).unwrap();
        copy_path
    }

    fn find_recipe_role_bone(world: &World, clip: SourceClipId, role: HumanoidRole) -> BoneId {
        world.resource::<RecipeClipSources>().by_clip[&clip]
            .roles
            .iter()
            .find(|(_, candidate)| *candidate == role)
            .map(|(bone, _)| *bone)
            .expect("role not found in recipe roles")
    }

    fn sample_rotation_near(
        world: &World,
        clip: SourceClipId,
        bone: BoneId,
        time: f32,
    ) -> [f32; 3] {
        let clip_library = world.resource::<ClipLibrary>();
        let track = clip_library
            .get(clip)
            .and_then(|editable_clip| editable_clip.get_track(bone))
            .expect("track not found");
        let value_near =
            |curve: &thyllore_anim_core::editable::components::curve::PropertyCurve| {
                curve
                    .keyframes
                    .iter()
                    .min_by(|a, b| (a.time - time).abs().total_cmp(&(b.time - time).abs()))
                    .expect("curve has no keyframes")
                    .value
            };
        [
            value_near(&track.rotation_x),
            value_near(&track.rotation_y),
            value_near(&track.rotation_z),
        ]
    }

    #[test]
    fn test_set_pose_rotation_rewrites_recipe_and_rebakes() {
        let (mut world, mut assets) = make_recipe_world(1);
        world.insert_resource(TimelineState::new());
        let copy_path = copy_wave_recipe_to_temp();

        let id = apply_recipe_file(&mut world, &mut assets, &copy_path).unwrap();
        let right_lower_arm_bone = find_recipe_role_bone(&world, id, HumanoidRole::RightLowerArm);
        let old_rotation = sample_rotation_near(&world, id, right_lower_arm_bone, 0.4);

        let new_id = set_recipe_pose_rotation(
            &mut world,
            &mut assets,
            id,
            HumanoidRole::RightLowerArm,
            1,
            [0.0, 0.0, 45.0],
        )
        .unwrap();

        let recipe_json = std::fs::read_to_string(&copy_path).unwrap();
        let recipe =
            thyllore_avatar_core::motion::systems::recipe_io::parse_recipe(&recipe_json).unwrap();
        assert_eq!(
            recipe.poses[1].rotations.get(&HumanoidRole::RightLowerArm),
            Some(&[0.0, 0.0, 45.0])
        );

        let new_rotation = sample_rotation_near(&world, new_id, right_lower_arm_bone, 0.4);
        assert!(
            old_rotation
                .iter()
                .zip(new_rotation.iter())
                .any(|(old, new)| (old - new).abs() > 1e-3),
            "rotation near t=0.4 should change: before {old_rotation:?}, after {new_rotation:?}"
        );

        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            Some(new_id)
        );

        std::fs::remove_file(&copy_path).ok();
    }

    #[test]
    fn test_detach_recipe_clip_marks_and_renames() {
        let (mut world, mut assets) = make_recipe_world(1);
        let copy_path = copy_wave_recipe_to_temp();
        let id = apply_recipe_file(&mut world, &mut assets, &copy_path).unwrap();

        detach_recipe_clip(&mut world, id);
        detach_recipe_clip(&mut world, id);

        assert!(world.resource::<RecipeClipSources>().by_clip[&id].detached);
        let clip_library = world.resource::<ClipLibrary>();
        let name = &clip_library.get(id).expect("clip not found").name;
        assert_eq!(name, "wave_right_hand (edited)");

        std::fs::remove_file(&copy_path).ok();
    }

    #[test]
    fn test_rebake_recipe_clip_restores_recipe_clip() {
        let (mut world, mut assets) = make_recipe_world(1);
        let copy_path = copy_wave_recipe_to_temp();
        let id = apply_recipe_file(&mut world, &mut assets, &copy_path).unwrap();
        detach_recipe_clip(&mut world, id);

        let new_id = rebake_recipe_clip(&mut world, &mut assets, id).unwrap();

        let clip_library = world.resource::<ClipLibrary>();
        assert_eq!(
            clip_library.source_clips.len(),
            1,
            "expected exactly 1 clip"
        );
        let name = &clip_library.get(new_id).expect("new clip not found").name;
        assert_eq!(name, "wave_right_hand");

        let recipe_sources = world.resource::<RecipeClipSources>();
        assert_eq!(
            recipe_sources.by_clip.len(),
            1,
            "expected exactly 1 source entry"
        );
        let source = recipe_sources
            .by_clip
            .get(&new_id)
            .expect("source not found");
        assert!(!source.detached, "detached should be false after rebake");

        let entities: Vec<Entity> = world.component_entities::<ClipSchedule>();
        let schedule = world.get_component::<ClipSchedule>(entities[0]).unwrap();
        assert_eq!(
            schedule.instances.len(),
            1,
            "expected exactly 1 schedule instance"
        );

        std::fs::remove_file(&copy_path).ok();
    }

    #[test]
    fn test_apply_recipe_file_mutes_overlapping_instances() {
        let (mut world, mut assets) = make_recipe_world(1);
        let copy_path = copy_wave_recipe_to_temp();

        let entities: Vec<Entity> = world.component_entities::<ClipSchedule>();
        let schedule_entity = entities[0];
        {
            let mut schedule = world
                .get_component_mut::<ClipSchedule>(schedule_entity)
                .unwrap();
            clip_schedule_add_instance(&mut schedule, 99, 5.0);
        }

        let id = apply_recipe_file(&mut world, &mut assets, &copy_path).unwrap();

        let schedule = world
            .get_component::<ClipSchedule>(schedule_entity)
            .unwrap();
        assert_eq!(
            schedule.instances.len(),
            2,
            "expected exactly 2 schedule instances"
        );

        let overlap_instance = schedule
            .instances
            .iter()
            .find(|i| i.source_id == 99)
            .expect("source 99 instance not found");
        assert!(overlap_instance.muted, "source 99 instance should be muted");

        let recipe_instance = schedule
            .instances
            .iter()
            .find(|i| i.source_id == id)
            .expect("recipe instance not found");
        assert!(
            !recipe_instance.muted,
            "recipe instance should not be muted"
        );

        std::fs::remove_file(&copy_path).ok();
    }
}
