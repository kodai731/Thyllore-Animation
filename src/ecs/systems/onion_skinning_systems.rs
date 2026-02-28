use cgmath::{Matrix4, Vector3};

use crate::animation::editable::SourceClipId;
use crate::animation::{Skeleton, SkeletonId, SkinData};
use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{
    ClipLibrary, GhostFrameInfo, GhostMeshData, OnionSkinningConfig, OnionSkinningResult,
};
use crate::ecs::world::{Animator, World};
use crate::ecs::{compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose};
use crate::vulkanr::data::Vertex;

pub struct OnionSkinMeshContext<'a> {
    pub base_vertices: &'a [Vertex],
    pub mesh_index: usize,
    pub skin_data: &'a SkinData,
}

pub fn compute_total_ghost_count(config: &OnionSkinningConfig) -> u32 {
    if config.enabled {
        config.past_count + config.future_count
    } else {
        0
    }
}

pub fn compute_ghost_time_offsets(config: &OnionSkinningConfig) -> Vec<GhostFrameInfo> {
    if !config.enabled {
        return Vec::new();
    }

    let mut offsets = Vec::new();

    for i in 1..=config.past_count {
        let distance = i as f32;
        let opacity = config.opacity * (1.0 - (distance - 1.0) / config.past_count.max(1) as f32);
        offsets.push(GhostFrameInfo {
            time_offset: -(i as f32) * config.frame_step,
            tint_color: config.past_color,
            opacity,
        });
    }

    for i in 1..=config.future_count {
        let distance = i as f32;
        let opacity = config.opacity * (1.0 - (distance - 1.0) / config.future_count.max(1) as f32);
        offsets.push(GhostFrameInfo {
            time_offset: i as f32 * config.frame_step,
            tint_color: config.future_color,
            opacity,
        });
    }

    offsets
}

pub fn compute_onion_skin_ghosts(
    config: &OnionSkinningConfig,
    current_time: f32,
    world: &World,
    assets: &AssetStorage,
    clip_library: &ClipLibrary,
    mesh_ctx: &OnionSkinMeshContext,
) -> OnionSkinningResult {
    if !config.enabled {
        return OnionSkinningResult {
            ghost_meshes: Vec::new(),
        };
    }

    let ghost_infos = compute_ghost_time_offsets(config);
    if ghost_infos.is_empty() {
        return OnionSkinningResult {
            ghost_meshes: Vec::new(),
        };
    }

    let animation_context = match collect_animation_context(
        world,
        assets,
        mesh_ctx.mesh_index,
        mesh_ctx.skin_data.clone(),
    ) {
        Some(ctx) => ctx,
        None => {
            return OnionSkinningResult {
                ghost_meshes: Vec::new(),
            };
        }
    };

    let ghost_meshes = ghost_infos
        .iter()
        .filter_map(|info| {
            compute_single_ghost(
                info,
                current_time,
                &animation_context,
                assets,
                clip_library,
                mesh_ctx.base_vertices,
                mesh_ctx.mesh_index,
            )
        })
        .collect();

    OnionSkinningResult { ghost_meshes }
}

struct AnimationContext {
    skeleton_id: SkeletonId,
    source_id: SourceClipId,
    looping: bool,
    skin_data: SkinData,
}

fn collect_animation_context(
    world: &World,
    assets: &AssetStorage,
    mesh_index: usize,
    skin_data: SkinData,
) -> Option<AnimationContext> {
    for (entity, animator) in world.iter_components::<Animator>() {
        let schedule = world.get_component::<ClipSchedule>(entity)?;
        let mesh_ref = world.get_component::<crate::ecs::world::MeshRef>(entity)?;
        let mesh_asset = assets.get_mesh(mesh_ref.mesh_asset_id)?;

        if mesh_asset.graphics_mesh_index != mesh_index {
            continue;
        }

        let skeleton_id = mesh_asset.skeleton_id?;
        let source_id = schedule.instances.first().map(|inst| inst.source_id)?;
        let _skeleton = assets.get_skeleton_by_skeleton_id(skeleton_id)?;

        return Some(AnimationContext {
            skeleton_id,
            source_id,
            looping: animator.looping,
            skin_data,
        });
    }

    None
}

fn compute_single_ghost(
    info: &GhostFrameInfo,
    current_time: f32,
    ctx: &AnimationContext,
    assets: &AssetStorage,
    clip_library: &ClipLibrary,
    base_vertices: &[Vertex],
    mesh_index: usize,
) -> Option<GhostMeshData> {
    let ghost_time = (current_time + info.time_offset).max(0.0);
    let skeleton = assets.get_skeleton_by_skeleton_id(ctx.skeleton_id)?;
    let asset_id = clip_library.get_asset_id_for_source(ctx.source_id)?;
    let clip_asset = assets.animation_clips.get(&asset_id)?;

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(
        &clip_asset.clip,
        ghost_time,
        skeleton,
        &mut pose,
        ctx.looping,
    );

    let globals = compute_pose_global_transforms(skeleton, &pose);

    let skinning_result =
        apply_skinning_to_vertices(&ctx.skin_data, &globals, skeleton, base_vertices);

    Some(GhostMeshData {
        vertices: skinning_result,
        tint_color: info.tint_color,
        opacity: info.opacity,
        mesh_index,
    })
}

fn apply_skinning_to_vertices(
    skin_data: &SkinData,
    global_transforms: &[Matrix4<f32>],
    skeleton: &Skeleton,
    base_vertices: &[Vertex],
) -> Vec<Vertex> {
    let vertex_count = skin_data.base_positions.len();

    let mut skinned_positions = vec![Vector3::new(0.0, 0.0, 0.0); vertex_count];
    let mut skinned_normals = vec![Vector3::new(0.0, 1.0, 0.0); vertex_count];

    let _ = crate::ecs::apply_skinning(
        skin_data,
        global_transforms,
        skeleton,
        &mut skinned_positions,
        &mut skinned_normals,
    );

    assert_eq!(
        skinned_positions.len(),
        base_vertices.len(),
        "skinned positions count must match base vertices count"
    );

    let mut result = base_vertices.to_vec();

    for (i, pos) in skinned_positions.iter().enumerate() {
        result[i].pos.x = pos.x;
        result[i].pos.y = pos.y;
        result[i].pos.z = pos.z;
    }

    for (i, normal) in skinned_normals.iter().enumerate() {
        result[i].normal.x = normal.x;
        result[i].normal.y = normal.y;
        result[i].normal.z = normal.z;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::resource::{GhostMeshData, OnionSkinningConfig, OnionSkinningResult};

    #[test]
    fn test_total_ghost_count_disabled() {
        let config = OnionSkinningConfig::default();
        assert_eq!(compute_total_ghost_count(&config), 0);
    }

    #[test]
    fn test_total_ghost_count_enabled() {
        let mut config = OnionSkinningConfig::default();
        config.enabled = true;
        assert_eq!(compute_total_ghost_count(&config), 4);
    }

    #[test]
    fn test_compute_ghosts_disabled() {
        let config = OnionSkinningConfig::default();
        let result = OnionSkinningResult {
            ghost_meshes: Vec::new(),
        };
        assert!(result.ghost_meshes.is_empty());
        assert!(!config.enabled);
    }

    #[test]
    fn test_ghost_mesh_data_fields() {
        let ghost = GhostMeshData {
            vertices: Vec::new(),
            tint_color: [0.2, 0.4, 1.0],
            opacity: 0.3,
            mesh_index: 0,
        };
        assert_eq!(ghost.tint_color, [0.2, 0.4, 1.0]);
        assert!((ghost.opacity - 0.3).abs() < f32::EPSILON);
        assert_eq!(ghost.mesh_index, 0);
    }

    #[test]
    fn test_ghost_mesh_data_clone() {
        let ghost = GhostMeshData {
            vertices: Vec::new(),
            tint_color: [1.0, 0.4, 0.2],
            opacity: 0.5,
            mesh_index: 3,
        };
        let cloned = ghost.clone();
        assert_eq!(cloned.tint_color, ghost.tint_color);
        assert_eq!(cloned.mesh_index, ghost.mesh_index);
        assert!((cloned.opacity - ghost.opacity).abs() < f32::EPSILON);
    }

    #[test]
    fn test_onion_skinning_result_empty() {
        let result = OnionSkinningResult {
            ghost_meshes: Vec::new(),
        };
        assert_eq!(result.ghost_meshes.len(), 0);
    }

    #[test]
    fn test_onion_skinning_result_with_ghosts() {
        let result = OnionSkinningResult {
            ghost_meshes: vec![
                GhostMeshData {
                    vertices: Vec::new(),
                    tint_color: [0.2, 0.4, 1.0],
                    opacity: 0.4,
                    mesh_index: 0,
                },
                GhostMeshData {
                    vertices: Vec::new(),
                    tint_color: [1.0, 0.4, 0.2],
                    opacity: 0.3,
                    mesh_index: 0,
                },
            ],
        };
        assert_eq!(result.ghost_meshes.len(), 2);
        assert_eq!(result.ghost_meshes[0].tint_color, [0.2, 0.4, 1.0]);
        assert_eq!(result.ghost_meshes[1].tint_color, [1.0, 0.4, 0.2]);
    }

    #[test]
    fn test_compute_ghosts_returns_empty_when_disabled() {
        use crate::animation::SkinData;
        use crate::asset::AssetStorage;
        use crate::ecs::resource::ClipLibrary;
        use crate::ecs::World;

        let config = OnionSkinningConfig::default();
        let world = World::new();
        let assets = AssetStorage::default();
        let clip_library = ClipLibrary::default();
        let skin_data = SkinData::default();

        let mesh_ctx = OnionSkinMeshContext {
            base_vertices: &[],
            mesh_index: 0,
            skin_data: &skin_data,
        };
        let result =
            compute_onion_skin_ghosts(&config, 0.0, &world, &assets, &clip_library, &mesh_ctx);
        assert!(result.ghost_meshes.is_empty());
    }

    #[test]
    fn test_compute_ghosts_returns_empty_with_no_entities() {
        use crate::animation::SkinData;
        use crate::asset::AssetStorage;
        use crate::ecs::resource::ClipLibrary;
        use crate::ecs::World;

        let mut config = OnionSkinningConfig::default();
        config.enabled = true;
        let world = World::new();
        let assets = AssetStorage::default();
        let clip_library = ClipLibrary::default();
        let skin_data = SkinData::default();

        let mesh_ctx = OnionSkinMeshContext {
            base_vertices: &[],
            mesh_index: 0,
            skin_data: &skin_data,
        };
        let result =
            compute_onion_skin_ghosts(&config, 1.0, &world, &assets, &clip_library, &mesh_ctx);
        assert!(result.ghost_meshes.is_empty());
    }

    #[test]
    fn test_ghost_time_offsets_disabled() {
        let config = OnionSkinningConfig::default();
        assert!(compute_ghost_time_offsets(&config).is_empty());
    }

    #[test]
    fn test_ghost_time_offsets_enabled() {
        let mut config = OnionSkinningConfig::default();
        config.enabled = true;
        config.past_count = 2;
        config.future_count = 1;

        let offsets = compute_ghost_time_offsets(&config);
        assert_eq!(offsets.len(), 3);

        assert!(offsets[0].time_offset < 0.0);
        assert!(offsets[1].time_offset < 0.0);
        assert!(offsets[2].time_offset > 0.0);

        assert!(offsets[0].opacity > 0.0);
        assert!(offsets[1].opacity > 0.0);
        assert!(offsets[2].opacity > 0.0);

        assert_eq!(offsets[0].tint_color, config.past_color);
        assert_eq!(offsets[2].tint_color, config.future_color);
    }

    #[test]
    fn test_ghost_opacity_falloff() {
        let mut config = OnionSkinningConfig::default();
        config.enabled = true;
        config.past_count = 3;
        config.future_count = 0;
        config.opacity = 0.6;

        let offsets = compute_ghost_time_offsets(&config);
        assert_eq!(offsets.len(), 3);

        assert!(offsets[0].opacity >= offsets[1].opacity);
        assert!(offsets[1].opacity >= offsets[2].opacity);
    }
}
