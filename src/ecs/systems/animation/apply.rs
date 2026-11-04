use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector4};

use crate::animation::{compose_transform, Skeleton, SkeletonPose};
use crate::render::RenderBackend;
use crate::vulkanr::resource::graphics_resource::{GraphicsResources, NodeData};

pub(crate) fn build_node_based_bone_transforms(
    nodes: &[NodeData],
    skeleton: &Skeleton,
) -> Vec<Matrix4<f32>> {
    let mut transforms = vec![Matrix4::identity(); skeleton.bones.len()];
    for bone in &skeleton.bones {
        let matched_node = nodes.iter().find(|n| n.name == bone.name).or_else(|| {
            bone.node_index
                .and_then(|idx| nodes.iter().find(|n| n.index == idx))
        });

        if let Some(node) = matched_node {
            transforms[bone.id as usize] = node.global_transform;
        }
    }
    transforms
}

pub fn apply_skinning_to_single_mesh(
    graphics: &mut GraphicsResources,
    mesh_idx: usize,
    global_transforms: &[Matrix4<f32>],
    skeleton: &Skeleton,
) -> bool {
    let Some(mesh) = graphics.meshes.get_mut(mesh_idx) else {
        return false;
    };

    let Some(skin_data) = &mesh.skin_data else {
        return false;
    };

    let _ = thyllore_model_core::apply_skinning_to_vertices(
        skin_data,
        global_transforms,
        skeleton,
        &mut mesh.vertex_data.vertices,
    );

    true
}

pub fn apply_node_animation_to_single_mesh(
    graphics: &mut GraphicsResources,
    mesh_idx: usize,
    nodes: &[NodeData],
    scale: f32,
) -> bool {
    if mesh_idx >= graphics.meshes.len() {
        return false;
    }

    let mesh = &graphics.meshes[mesh_idx];
    if mesh.skin_data.is_some() || mesh.base_vertices.is_empty() {
        return false;
    }

    let Some(node_idx) = mesh.node_index else {
        return false;
    };

    let node_found = nodes.iter().find(|n| n.index == node_idx);
    let Some(node) = node_found else {
        return false;
    };

    let transform = node.global_transform;

    let mesh = &mut graphics.meshes[mesh_idx];
    for (i, v) in mesh.vertex_data.vertices.iter_mut().enumerate() {
        if i < mesh.base_vertices.len() {
            let base = &mesh.base_vertices[i];
            let pos = transform * Vector4::new(base.pos.x, base.pos.y, base.pos.z, 1.0);
            v.pos.x = pos.x * scale;
            v.pos.y = pos.y * scale;
            v.pos.z = pos.z * scale;
        }
    }

    true
}

pub fn compute_node_global_transforms(
    nodes: &mut [NodeData],
    skeleton: &Skeleton,
    pose: &SkeletonPose,
) {
    if nodes.is_empty() {
        return;
    }

    for bone in &skeleton.bones {
        if let Some(node) = nodes.iter_mut().find(|n| n.name == bone.name) {
            let idx = bone.id as usize;
            if idx < pose.bone_poses.len() {
                let bp = &pose.bone_poses[idx];
                node.local_transform = compose_transform(bp.translation, bp.rotation, bp.scale);
            }
        }
    }

    let node_count = nodes.len();

    fn compute_global(
        nodes: &[NodeData],
        node_idx: usize,
        computed: &mut [bool],
        global_transforms: &mut [Matrix4<f32>],
    ) -> Matrix4<f32> {
        if computed[node_idx] {
            return global_transforms[node_idx];
        }

        let local = nodes[node_idx].local_transform;
        let global = if let Some(parent_idx) = nodes[node_idx].parent_index {
            if let Some(parent_array_idx) = nodes.iter().position(|n| n.index == parent_idx) {
                let parent_global =
                    compute_global(nodes, parent_array_idx, computed, global_transforms);
                parent_global * local
            } else {
                local
            }
        } else {
            local
        };

        global_transforms[node_idx] = global;
        computed[node_idx] = true;
        global
    }

    let mut computed = vec![false; node_count];
    let mut global_transforms = vec![Matrix4::identity(); node_count];

    for i in 0..node_count {
        compute_global(nodes, i, &mut computed, &mut global_transforms);
    }

    for (i, node) in nodes.iter_mut().enumerate() {
        node.global_transform = global_transforms[i];
    }
}

pub unsafe fn upload_animations(
    backend: &mut dyn RenderBackend,
    updated_meshes: &[usize],
) -> Result<()> {
    if !updated_meshes.is_empty() {
        backend.upload_animated_meshes(updated_meshes)?;
    }

    Ok(())
}

pub fn prepare_node_animation(
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    skeleton: &Skeleton,
    pose: &SkeletonPose,
    node_animation_scale: f32,
) -> Vec<usize> {
    compute_node_global_transforms(nodes, skeleton, pose);

    let mut updated_mesh_indices = Vec::new();

    for (mesh_idx, mesh) in graphics.meshes.iter_mut().enumerate() {
        if mesh.skin_data.is_some() || mesh.base_vertices.is_empty() {
            continue;
        }

        let Some(node_idx) = mesh.node_index else {
            continue;
        };

        let node_found = nodes.iter().find(|n| n.index == node_idx);
        let Some(node) = node_found else {
            continue;
        };

        let transform = node.global_transform;

        for (i, v) in mesh.vertex_data.vertices.iter_mut().enumerate() {
            if i < mesh.base_vertices.len() {
                let base = &mesh.base_vertices[i];
                let pos = transform * Vector4::new(base.pos.x, base.pos.y, base.pos.z, 1.0);
                v.pos.x = pos.x * node_animation_scale;
                v.pos.y = pos.y * node_animation_scale;
                v.pos.z = pos.z * node_animation_scale;
            }
        }

        updated_mesh_indices.push(mesh_idx);
    }

    updated_mesh_indices
}
