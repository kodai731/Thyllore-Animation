use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::world::World;

pub fn dispatch_debug_constraint_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    use crate::ecs::systems::debug_constraint_systems::{
        clear_test_constraints, create_test_constraints,
    };
    use crate::ecs::systems::debug_spring_bone_systems::{
        clear_spring_bones, create_test_spring_bones,
    };

    for event in events {
        match event {
            UIEvent::CreateTestConstraints => {
                create_test_constraints(world, assets);
            }
            UIEvent::ClearTestConstraints => {
                clear_test_constraints(world);
            }
            UIEvent::AddTestSpringBones => {
                create_test_spring_bones(world, assets);
            }
            UIEvent::ClearSpringBones => {
                let is_baked = world
                    .get_resource::<crate::ecs::resource::SpringBoneState>()
                    .map_or(false, |s| s.baked_clip_source_id.is_some());
                if is_baked {
                    super::dispatch_spring_bone::handle_spring_bone_discard(world, assets);
                }
                clear_spring_bones(world);
            }
            _ => {}
        }
    }
}

pub fn dispatch_constraint_edit_events(events: &[UIEvent], world: &mut World) {
    use crate::ecs::systems::constraint_edit_systems::{
        handle_constraint_add, handle_constraint_remove, handle_constraint_update,
    };

    for event in events {
        match event {
            UIEvent::ConstraintAdd {
                entity,
                constraint_type_index,
            } => {
                handle_constraint_add(world, *entity, *constraint_type_index);
            }
            UIEvent::ConstraintRemove {
                entity,
                constraint_id,
            } => {
                handle_constraint_remove(world, *entity, *constraint_id);
            }
            UIEvent::ConstraintUpdate {
                entity,
                constraint_id,
                constraint,
            } => {
                handle_constraint_update(world, *entity, *constraint_id, constraint);
            }
            _ => {}
        }
    }
}

pub fn dispatch_constraint_bake_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    use crate::ecs::component::ConstraintSet;
    use crate::ecs::resource::{ClipLibrary, TimelineState};
    use crate::ecs::systems::constraint_bake_systems::{
        constraint_bake_evaluate, constraint_bake_register, constraint_bake_rest_pose,
    };

    for event in events {
        let UIEvent::ConstraintBakeToKeyframes { entity, sample_fps } = event else {
            continue;
        };

        let skeleton = match assets.skeletons.values().next() {
            Some(skel_asset) => skel_asset.skeleton.clone(),
            None => {
                crate::log!("Bake failed: no skeleton found");
                continue;
            }
        };

        let constraint_set = match world.get_component::<ConstraintSet>(*entity) {
            Some(set) => set.clone(),
            None => {
                crate::log!("Bake failed: no ConstraintSet on entity");
                continue;
            }
        };

        let timeline_state = world.resource::<TimelineState>();
        let clip_id = timeline_state.current_clip_id;
        let looping = timeline_state.looping;
        drop(timeline_state);

        let mut baked = if let Some(source_id) = clip_id {
            let clip_library = world.resource::<ClipLibrary>();
            match clip_library.get(source_id) {
                Some(editable) => {
                    let anim_clip = crate::animation::editable::clip_to_animation(editable);
                    let source_name = editable.name.clone();
                    drop(clip_library);

                    let mut result = constraint_bake_evaluate(
                        &anim_clip,
                        &skeleton,
                        &constraint_set,
                        *sample_fps,
                        looping,
                    );
                    result.name = format!("{}_baked", source_name);
                    result
                }
                None => {
                    drop(clip_library);
                    constraint_bake_rest_pose(&skeleton, &constraint_set)
                }
            }
        } else {
            constraint_bake_rest_pose(&skeleton, &constraint_set)
        };

        baked.name = if baked.name.is_empty() {
            "baked".to_string()
        } else {
            baked.name
        };

        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let new_id = constraint_bake_register(&mut clip_library, assets, baked);
        crate::log!("Baked constraints to new clip (id={})", new_id);
    }
}
