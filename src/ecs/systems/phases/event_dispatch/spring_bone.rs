use std::path::PathBuf;

use crate::animation::spring_bone::SpringSpace;
use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::component::ColliderShape;
use crate::ecs::component::SpringChain;
use crate::ecs::component::SpringChainId;
use crate::ecs::component::SpringColliderDef;
use crate::ecs::component::SpringColliderGroup;
use crate::ecs::component::SpringColliderGroupId;
use crate::ecs::component::SpringColliderId;
use crate::ecs::component::SpringJointParam;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::systems::spring_bone_systems::spring_space_for_entity;
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum SpringBoneEvent {
    Bake,
    DiscardBake,
    Rebake,
    ChainAdd {
        entity: Entity,
        root_bone_id: BoneId,
        chain_length: u32,
    },
    ChainRemove {
        entity: Entity,
        chain_id: SpringChainId,
    },
    ChainUpdate {
        entity: Entity,
        chain_id: SpringChainId,
        chain: SpringChain,
    },
    JointUpdate {
        entity: Entity,
        chain_id: SpringChainId,
        joint_index: usize,
        joint: SpringJointParam,
    },
    ColliderAdd {
        entity: Entity,
        bone_id: BoneId,
        shape: ColliderShape,
    },
    ColliderRemove {
        entity: Entity,
        collider_id: SpringColliderId,
    },
    ColliderUpdate {
        entity: Entity,
        collider_id: SpringColliderId,
        collider: SpringColliderDef,
    },
    ColliderGroupAdd {
        entity: Entity,
        name: String,
    },
    ColliderGroupRemove {
        entity: Entity,
        group_id: SpringColliderGroupId,
    },
    ColliderGroupUpdate {
        entity: Entity,
        group_id: SpringColliderGroupId,
        group: SpringColliderGroup,
    },
    ToggleGizmo(bool),
}

impl UiCommand for SpringBoneEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        let events = [*self];
        dispatch_spring_bone_bake_events(&events, world, assets);
        dispatch_spring_bone_edit_events(&events, world, assets);
    }
}

#[derive(Debug)]
pub struct DumpSpringTails {
    pub path: PathBuf,
    pub sample_rate: f32,
}

impl UiCommand for DumpSpringTails {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_bake_systems::{simulate_spring_tails, BakeConfig};

        let inputs = match find_spring_bake_inputs(world, assets) {
            Some(i) => i,
            None => return,
        };

        let config = BakeConfig {
            start_time: 0.0,
            end_time: inputs.anim_clip.duration,
            sample_rate: self.sample_rate,
            space: inputs.space,
        };

        let samples = simulate_spring_tails(
            &config,
            &inputs.setup,
            &inputs.skeleton,
            &inputs.anim_clip,
            inputs.constraints.as_ref(),
            inputs.looping,
        );

        let csv = crate::animation::spring_bone::format_tail_csv(&samples);
        match std::fs::write(&self.path, &csv) {
            Ok(()) => log!(
                "Wrote {} spring tail samples to {:?}",
                samples.len(),
                self.path
            ),
            Err(e) => msg_warn!("Failed to write spring tails to {:?}: {}", self.path, e),
        }
    }
}

#[derive(Debug)]
pub struct DumpSkinnedVertices {
    pub path: PathBuf,
    pub sample_rate: f32,
    pub submesh: thyllore_avatar_core::garment::components::submesh_role::SubmeshKey,
}

impl UiCommand for DumpSkinnedVertices {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        use crate::ecs::systems::spring_bone_bake_systems::{
            simulate_skinned_positions, BakeConfig,
        };

        let inputs = match find_spring_bake_inputs(world, assets) {
            Some(i) => i,
            None => return,
        };

        let submesh_meshes = {
            let node_assets = world.resource::<crate::ecs::resource::NodeAssets>();
            crate::ecs::systems::avatar_submesh_role_systems::collect_submesh_meshes(
                assets,
                graphics,
                &node_assets.nodes,
            )
        };

        let Some(&(_, graphics_mesh_index)) =
            submesh_meshes.iter().find(|(k, _)| *k == self.submesh)
        else {
            let available: Vec<String> = submesh_meshes
                .iter()
                .map(|(k, _)| format!("{}/{}", k.mesh, k.material))
                .collect();
            msg_warn!(
                "Submesh {}/{} not found; available: {}",
                self.submesh.mesh,
                self.submesh.material,
                available.join(", ")
            );
            return;
        };

        let Some(skin) = graphics.meshes[graphics_mesh_index].skin_data.as_ref() else {
            msg_warn!(
                "Submesh {}/{} has no skin data",
                self.submesh.mesh,
                self.submesh.material
            );
            return;
        };

        let config = BakeConfig {
            start_time: 0.0,
            end_time: inputs.anim_clip.duration,
            sample_rate: self.sample_rate,
            space: inputs.space,
        };

        let frames = simulate_skinned_positions(
            &config,
            &inputs.setup,
            &inputs.skeleton,
            &inputs.anim_clip,
            inputs.constraints.as_ref(),
            inputs.looping,
            skin,
        );

        let mut csv = String::from("frame,vertex,bind_x,bind_y,bind_z,x,y,z\n");
        for (frame_idx, positions) in frames.iter().enumerate() {
            for (v_idx, pos) in positions.iter().enumerate() {
                let bind = &skin.base_positions[v_idx];
                csv.push_str(&format!(
                    "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
                    frame_idx, v_idx, bind.x, bind.y, bind.z, pos.x, pos.y, pos.z,
                ));
            }
        }

        match std::fs::write(&self.path, &csv) {
            Ok(()) => log!(
                "Wrote {} frames of skinned vertices to {:?}",
                frames.len(),
                self.path
            ),
            Err(e) => msg_warn!("Failed to write skinned vertices to {:?}: {}", self.path, e),
        }
    }
}

#[derive(Debug)]
pub struct FitSpringParams {
    pub reference: PathBuf,
    pub sample_rate: f32,
    pub params: thyllore_math_core::NelderMeadParams,
}

impl UiCommand for FitSpringParams {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_bake_systems::BakeConfig;
        use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
        use crate::ecs::systems::spring_bone_fit_systems::{fit_spring_params, SpringFitInputs};

        let Some(reference) = read_spring_reference(&self.reference) else {
            return;
        };

        let inputs = match find_spring_bake_inputs(world, assets) {
            Some(i) => i,
            None => return,
        };

        let config = BakeConfig {
            start_time: 0.0,
            end_time: inputs.anim_clip.duration,
            sample_rate: self.sample_rate,
            space: inputs.space,
        };

        let spring_fit_inputs = SpringFitInputs {
            config: &config,
            skeleton: &inputs.skeleton,
            clip: &inputs.anim_clip,
            constraints: inputs.constraints.as_ref(),
            looping: inputs.looping,
            reference: &reference,
        };

        let (fitted_setup, fits) =
            fit_spring_params(&spring_fit_inputs, &inputs.setup, &self.params);

        if fits.is_empty() {
            msg_warn!("Spring parameter estimation: no chains to fit");
            return;
        }

        for fit in &fits {
            msg_info!(
                "Fitted spring chain {}: rms {:.4} -> {:.4} (stiffness {:.3}, drag {:.3}, gravity {:.3})",
                fit.chain,
                fit.before_rms,
                fit.after_rms,
                fit.stiffness,
                fit.drag_force,
                fit.gravity_power
            );
        }

        world.insert_component(inputs.entity, fitted_setup);
        reset_spring_bone_state(world);
    }
}

fn read_spring_reference(
    path: &PathBuf,
) -> Option<Vec<crate::animation::spring_bone::SpringTailSample>> {
    let csv = match std::fs::read_to_string(path) {
        Ok(csv) => csv,
        Err(e) => {
            msg_error!("Failed to read spring reference CSV {:?}: {}", path, e);
            return None;
        }
    };

    match crate::animation::spring_bone::parse_tail_csv(&csv) {
        Ok(samples) => Some(samples),
        Err(e) => {
            msg_error!("Failed to parse spring reference CSV {:?}: {}", path, e);
            None
        }
    }
}

fn dispatch_spring_bone_bake_events(
    events: &[SpringBoneEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    for event in events {
        match event {
            SpringBoneEvent::Bake => {
                handle_spring_bone_bake(world, assets);
            }
            SpringBoneEvent::DiscardBake => {
                handle_spring_bone_discard(world, assets);
            }
            SpringBoneEvent::Rebake => {
                handle_spring_bone_discard(world, assets);
                handle_spring_bone_bake(world, assets);
            }
            _ => {}
        }
    }
}

pub fn transition_to_baked_override_if_needed(world: &mut World) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};

    if let Some(mut state) = world.get_resource_mut::<SpringBoneState>() {
        if state.mode == SpringBoneMode::Baked {
            state.mode = SpringBoneMode::BakedOverride;
            log!("Spring bone mode: Baked -> BakedOverride (manual edit detected)");
        }
    }
}

struct SpringBakeInputs {
    entity: Entity,
    space: SpringSpace,
    skeleton: crate::animation::Skeleton,
    setup: crate::ecs::component::SpringBoneSetup,
    constraints: Option<crate::ecs::component::ConstraintSet>,
    source_id: Option<u64>,
    looping: bool,
    anim_clip: crate::animation::AnimationClip,
    source_editable: crate::animation::editable::EditableAnimationClip,
}

fn find_spring_bake_inputs(world: &World, assets: &AssetStorage) -> Option<SpringBakeInputs> {
    use crate::ecs::component::{ConstraintSet, SpringBoneSetup, WithSpringBone};
    use crate::ecs::resource::{BakedHumanoidClips, TimelineState};

    let skeleton = match assets.skeletons.values().next() {
        Some(skel_asset) => skel_asset.skeleton.clone(),
        None => {
            msg_warn!("Spring bone bake failed: no skeleton found");
            return None;
        }
    };

    let spring_entity = world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(entity, _)| entity);

    let Some(entity) = spring_entity else {
        msg_warn!("Spring bone bake failed: no WithSpringBone entity");
        return None;
    };

    let setup = match world.get_component::<SpringBoneSetup>(entity) {
        Some(s) => s.clone(),
        None => {
            msg_warn!("Spring bone bake failed: no SpringBoneSetup");
            return None;
        }
    };

    let constraints = world.get_component::<ConstraintSet>(entity).cloned();

    let timeline_state = world.resource::<TimelineState>();
    let source_id = timeline_state.current_clip_id;
    let looping = timeline_state.looping;
    drop(timeline_state);

    let clip_library = world.resource::<ClipLibrary>();
    let source_editable = match source_id.and_then(|id| clip_library.get(id)) {
        Some(editable) => editable.clone(),
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no current clip");
            return None;
        }
    };

    let baked = match world.get_resource::<BakedHumanoidClips>() {
        Some(b) => b,
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no BakedHumanoidClips resource");
            return None;
        }
    };

    let source_id = match source_id {
        Some(id) => id,
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no current clip id");
            return None;
        }
    };

    let asset_id = match crate::ecs::systems::animation::collect::resolve_playable_asset(
        source_id,
        entity,
        &clip_library,
        &baked,
    ) {
        Some(id) => id,
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no playable asset for source");
            return None;
        }
    };
    drop(clip_library);

    let anim_clip = match assets.animation_clips.get(&asset_id) {
        Some(entry) => entry.clip.clone(),
        None => {
            msg_warn!("Spring bone bake failed: animation clip asset not found");
            return None;
        }
    };

    Some(SpringBakeInputs {
        entity,
        space: spring_space_for_entity(world, entity),
        skeleton,
        setup,
        constraints,
        source_id: Some(source_id),
        looping,
        anim_clip,
        source_editable,
    })
}

fn handle_spring_bone_bake(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState, TimelineState};
    use crate::ecs::systems::spring_bone_bake_systems::{
        merge_bake_into_clip, spring_bone_bake, BakeConfig,
    };

    let inputs = match find_spring_bake_inputs(world, assets) {
        Some(i) => i,
        None => return,
    };

    let config = BakeConfig {
        start_time: 0.0,
        end_time: inputs.anim_clip.duration,
        sample_rate: 30.0,
        space: inputs.space,
    };

    let bake_result = spring_bone_bake(
        &config,
        &inputs.setup,
        &inputs.skeleton,
        &inputs.anim_clip,
        inputs.constraints.as_ref(),
        inputs.looping,
    );

    let mut merged = inputs.source_editable;
    merge_bake_into_clip(&mut merged, &bake_result, &inputs.skeleton);
    merged.name = format!("{}_spring_baked", merged.name);

    log!(
        "[BakeDebug] bake_result: baked_bone_ids={:?}, clip_tracks={}",
        bake_result.baked_bone_ids,
        bake_result.clip.tracks.len()
    );
    log!(
        "[BakeDebug] merged clip: name={}, tracks={}, duration={}",
        merged.name,
        merged.tracks.len(),
        merged.duration
    );

    let new_id = register_baked_clip_and_update_schedules(world, assets, merged, inputs.source_id);

    let baked_bone_ids = bake_result.baked_bone_ids.clone();

    let mut spring_state = world.resource_mut::<SpringBoneState>();
    spring_state.mode = SpringBoneMode::Baked;
    spring_state.baked_clip_source_id = Some(new_id);
    spring_state.baked_bone_ids = bake_result.baked_bone_ids;
    spring_state.original_clip_source_id = inputs.source_id;

    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.current_clip_id = Some(new_id);
    timeline_state.baked_bone_ids = baked_bone_ids;

    log!("Spring bone baked to new clip (id={})", new_id);
}

fn register_baked_clip_and_update_schedules(
    world: &mut World,
    assets: &mut AssetStorage,
    clip: crate::animation::editable::EditableAnimationClip,
    source_id: Option<u64>,
) -> u64 {
    use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;

    let mut clip_library = world.resource_mut::<ClipLibrary>();
    let new_id = clip_library_register_and_activate(&mut clip_library, assets, clip);
    drop(clip_library);

    let mut updated_count = 0;
    let schedule_entities = world.component_entities::<ClipSchedule>();
    log!(
        "[BakeDebug] ClipSchedule entities count={}, original source_id={:?}",
        schedule_entities.len(),
        source_id
    );
    for sched_entity in &schedule_entities {
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*sched_entity) {
            if let Some(first) = schedule.instances.first_mut() {
                log!(
                    "[BakeDebug]   entity {:?}: schedule source_id={}, match={}",
                    sched_entity,
                    first.source_id,
                    Some(first.source_id) == source_id
                );
                if Some(first.source_id) == source_id {
                    first.source_id = new_id;
                    updated_count += 1;
                }
            }
        }
    }
    log!(
        "[BakeDebug] updated {} ClipSchedule(s) to new source_id={}",
        updated_count,
        new_id
    );

    new_id
}

pub fn handle_spring_bone_discard(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState, TimelineState};

    let mut spring_state = world.resource_mut::<SpringBoneState>();
    let original_id = spring_state.original_clip_source_id;
    let baked_id = spring_state.baked_clip_source_id;

    spring_state.mode = SpringBoneMode::Realtime;
    spring_state.baked_clip_source_id = None;
    spring_state.baked_bone_ids = Vec::new();
    spring_state.original_clip_source_id = None;
    spring_state.initialized = false;
    drop(spring_state);

    if let (Some(orig_id), Some(baked_source_id)) = (original_id, baked_id) {
        let schedule_entities = world.component_entities::<ClipSchedule>();
        for entity in &schedule_entities {
            if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                if let Some(first) = schedule.instances.first_mut() {
                    if first.source_id == baked_source_id {
                        first.source_id = orig_id;
                    }
                }
            }
        }
    }

    if let Some(baked_id) = baked_id {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        if let Some(asset_id) = clip_library.source_to_asset_id.remove(&baked_id) {
            assets.animation_clips.remove(&asset_id);
        }
        clip_library.remove(baked_id);
    }

    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.baked_bone_ids.clear();
    if let Some(orig) = original_id {
        timeline_state.current_clip_id = Some(orig);
    }

    log!("Discarded spring bone bake, restored original clip");
}

fn dispatch_spring_bone_edit_events(
    events: &[SpringBoneEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    use crate::ecs::systems::spring_bone_edit_systems::*;

    let skeleton = assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.clone());

    for event in events {
        match event {
            SpringBoneEvent::ChainAdd {
                entity,
                root_bone_id,
                chain_length,
            } => {
                if let Some(ref skel) = skeleton {
                    handle_spring_chain_add(world, *entity, *root_bone_id, *chain_length, skel);
                }
            }

            SpringBoneEvent::ChainRemove { entity, chain_id } => {
                handle_spring_chain_remove(world, *entity, *chain_id);
            }

            SpringBoneEvent::ChainUpdate {
                entity,
                chain_id,
                chain,
            } => {
                handle_spring_chain_update(world, *entity, *chain_id, chain.clone());
            }

            SpringBoneEvent::JointUpdate {
                entity,
                chain_id,
                joint_index,
                joint,
            } => {
                handle_spring_joint_update(world, *entity, *chain_id, *joint_index, joint.clone());
            }

            SpringBoneEvent::ColliderAdd {
                entity,
                bone_id,
                shape,
            } => {
                handle_spring_collider_add(world, *entity, *bone_id, shape.clone());
            }

            SpringBoneEvent::ColliderRemove {
                entity,
                collider_id,
            } => {
                handle_spring_collider_remove(world, *entity, *collider_id);
            }

            SpringBoneEvent::ColliderUpdate {
                entity,
                collider_id,
                collider,
            } => {
                handle_spring_collider_update(world, *entity, *collider_id, collider.clone());
            }

            SpringBoneEvent::ColliderGroupAdd { entity, name } => {
                handle_spring_collider_group_add(world, *entity, name.clone());
            }

            SpringBoneEvent::ColliderGroupRemove { entity, group_id } => {
                handle_spring_collider_group_remove(world, *entity, *group_id);
            }

            SpringBoneEvent::ColliderGroupUpdate {
                entity,
                group_id,
                group,
            } => {
                handle_spring_collider_group_update(world, *entity, *group_id, group.clone());
            }

            SpringBoneEvent::ToggleGizmo(visible) => {
                if let Some(mut gizmo) =
                    world.get_resource_mut::<crate::ecs::resource::gizmo::SpringBoneGizmoData>()
                {
                    gizmo.visible = *visible;
                }
            }

            _ => {}
        }
    }
}
