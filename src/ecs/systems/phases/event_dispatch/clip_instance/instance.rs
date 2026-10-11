use crate::ecs::resource::TimelineState;
use crate::ecs::systems::humanoid_bake_systems::request_bake_scan;
use crate::ecs::systems::timeline::clip_instance::{
    add_clip_instance, modify_clip_instance, remove_clip_instance, select_clip_instance,
};
use crate::ecs::world::World;

use super::group::apply_clip_group_events;
use super::ClipInstanceEvent;

/// Applies the batch to the schedules without recording the edit history; the batch CLI uses it.
pub fn apply_clip_instance_events(events: &[ClipInstanceEvent], world: &mut World) {
    for event in events {
        match event {
            ClipInstanceEvent::Add {
                entity,
                source_id,
                start_time,
            } => add_clip_instance(world, *entity, *source_id, *start_time),

            ClipInstanceEvent::Select {
                entity,
                instance_id,
            } => select_clip_instance(world, *entity, *instance_id),

            ClipInstanceEvent::Deselect => {
                world.resource_mut::<TimelineState>().selected_clip_instance = None;
            }

            ClipInstanceEvent::Move {
                entity,
                instance_id,
                new_start_time,
            } => modify_clip_instance(world, *entity, *instance_id, |inst| {
                inst.start_time = *new_start_time;
            }),

            ClipInstanceEvent::TrimStart {
                entity,
                instance_id,
                new_clip_in,
            } => modify_clip_instance(world, *entity, *instance_id, |inst| {
                inst.clip_in = new_clip_in.clamp(0.0, inst.clip_out);
            }),

            ClipInstanceEvent::TrimEnd {
                entity,
                instance_id,
                new_clip_out,
            } => modify_clip_instance(world, *entity, *instance_id, |inst| {
                inst.clip_out = new_clip_out.max(inst.clip_in);
            }),

            ClipInstanceEvent::ToggleMute {
                entity,
                instance_id,
            } => modify_clip_instance(world, *entity, *instance_id, |inst| {
                inst.muted = !inst.muted;
            }),

            ClipInstanceEvent::Delete {
                entity,
                instance_id,
            } => remove_clip_instance(world, *entity, *instance_id),

            ClipInstanceEvent::SetWeight {
                entity,
                instance_id,
                weight,
            } => modify_clip_instance(world, *entity, *instance_id, |inst| {
                inst.weight = weight.clamp(0.0, 1.0);
            }),

            ClipInstanceEvent::SetBlendMode {
                entity,
                instance_id,
                blend_mode,
            } => modify_clip_instance(world, *entity, *instance_id, |inst| {
                inst.blend_mode = *blend_mode;
            }),

            ClipInstanceEvent::GroupCreate { .. }
            | ClipInstanceEvent::GroupDelete { .. }
            | ClipInstanceEvent::GroupAddInstance { .. }
            | ClipInstanceEvent::GroupRemoveInstance { .. }
            | ClipInstanceEvent::GroupToggleMute { .. }
            | ClipInstanceEvent::GroupSetWeight { .. } => {}
        }
    }

    apply_clip_group_events(events, world);
    request_bake_scan(world);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{
        ClipInstance, EditableAnimationClip, SourceClip, SourceClipId,
    };
    use crate::ecs::component::ClipSchedule;
    use crate::ecs::resource::{ClipLibrary, HumanoidRigState};
    use crate::ecs::systems::avatar_setup_systems::find_first_skeleton;
    use crate::ecs::systems::humanoid_bake_systems::unresolved_clip_roles;
    use crate::ecs::systems::humanoid_rig_systems::{
        build_humanoid_rig, copy_test_humanoid_fixture, test_humanoid_world,
    };
    use crate::ecs::world::Entity;
    use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
    use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

    fn spawn_zero_length_clip_instance(world: &mut World) -> Entity {
        let entity = world.spawn();
        let mut schedule = ClipSchedule::default();
        schedule.next_instance_id = 2;
        schedule.instances.push(ClipInstance::new(1, 1, 0.0));
        world.insert_component(entity, schedule);
        entity
    }

    fn instance_clip_range(world: &World, entity: Entity) -> (f32, f32) {
        let schedule = world.get_component::<ClipSchedule>(entity).unwrap();
        let inst = &schedule.instances[0];
        (inst.clip_in, inst.clip_out)
    }

    #[test]
    fn trim_end_extends_a_zero_length_clip_freely() {
        let mut world = World::new();
        let entity = spawn_zero_length_clip_instance(&mut world);

        apply_clip_instance_events(
            &[ClipInstanceEvent::TrimEnd {
                entity,
                instance_id: 1,
                new_clip_out: 3.0,
            }],
            &mut world,
        );

        let (clip_in, clip_out) = instance_clip_range(&world, entity);
        assert!((clip_in - 0.0).abs() < 1e-6);
        assert!((clip_out - 3.0).abs() < 1e-6);
    }

    #[test]
    fn trim_end_never_drops_below_clip_in() {
        let mut world = World::new();
        let entity = spawn_zero_length_clip_instance(&mut world);

        apply_clip_instance_events(
            &[
                ClipInstanceEvent::TrimStart {
                    entity,
                    instance_id: 1,
                    new_clip_in: 0.0,
                },
                ClipInstanceEvent::TrimEnd {
                    entity,
                    instance_id: 1,
                    new_clip_out: -2.0,
                },
            ],
            &mut world,
        );

        let (clip_in, clip_out) = instance_clip_range(&world, entity);
        assert!(clip_out >= clip_in);
    }

    #[test]
    fn trim_start_never_exceeds_clip_out() {
        let mut world = World::new();
        let entity = spawn_zero_length_clip_instance(&mut world);

        apply_clip_instance_events(
            &[
                ClipInstanceEvent::TrimEnd {
                    entity,
                    instance_id: 1,
                    new_clip_out: 2.0,
                },
                ClipInstanceEvent::TrimStart {
                    entity,
                    instance_id: 1,
                    new_clip_in: 5.0,
                },
            ],
            &mut world,
        );

        let (clip_in, clip_out) = instance_clip_range(&world, entity);
        assert!((clip_in - 2.0).abs() < 1e-6);
        assert!((clip_out - 2.0).abs() < 1e-6);
    }

    #[test]
    fn clip_with_unresolved_roles_is_scheduled() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let mut rig =
            build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");

        let clip_id: SourceClipId = 1;
        let mut clip = EditableAnimationClip::new(clip_id, "role".to_string());
        let track = clip.add_track(rig.track_bones["Hips"], "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_x, 0.0, 0.0);

        rig.mapping.by_role.remove(&HumanoidRole::Hips);
        assert!(!unresolved_clip_roles(&clip, Some(&rig.mapping)).is_empty());
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        let mut library = ClipLibrary::default();
        library
            .source_clips
            .insert(clip_id, SourceClip::new(clip_id, clip));
        world.insert_resource(library);
        let entity = world.spawn();
        world.insert_component(entity, ClipSchedule::default());

        apply_clip_instance_events(
            &[ClipInstanceEvent::Add {
                entity,
                source_id: clip_id,
                start_time: 0.0,
            }],
            &mut world,
        );

        let schedule = world.get_component::<ClipSchedule>(entity).unwrap();
        assert_eq!(
            schedule.instances.len(),
            1,
            "a clip with unresolved roles should still be scheduled"
        );
    }
}
