use crate::animation::Skeleton;
use crate::ecs::component::SpringBoneSetup;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use std::collections::HashSet;
use thyllore_avatar_core::stats::components::stats::AvatarStats;

pub(crate) fn count_avatar_stats(
    graphics: &GraphicsResources,
    skeleton: Option<&Skeleton>,
    spring_setup: Option<&SpringBoneSetup>,
) -> AvatarStats {
    let mut stats = count_mesh_stats(graphics);
    stats.bones = skeleton.map_or(0, |skeleton| skeleton.bones.len() as u32);
    if let Some(setup) = spring_setup {
        stats.spring_chains = setup.chains.len() as u32;
        stats.spring_transforms = setup
            .chains
            .iter()
            .map(|chain| chain.joints.len() as u32)
            .sum();
        stats.spring_colliders = setup.colliders.len() as u32;
    }
    stats
}

pub(crate) fn count_mesh_stats(graphics: &GraphicsResources) -> AvatarStats {
    let materials: HashSet<_> = graphics.mesh_material_ids.iter().collect();
    let morph_meshes: HashSet<&str> = graphics
        .meshes
        .iter()
        .map(|mesh| mesh.morph.source_mesh.as_str())
        .filter(|source_mesh| !source_mesh.is_empty())
        .collect();

    AvatarStats {
        triangles: graphics
            .meshes
            .iter()
            .map(|mesh| (mesh.vertex_data.indices.len() / 3) as u32)
            .sum(),
        meshes: graphics.meshes.len() as u32,
        skinned_meshes: graphics
            .meshes
            .iter()
            .filter(|mesh| mesh.skin_data.is_some())
            .count() as u32,
        materials: materials.len() as u32,
        morph_meshes: morph_meshes.len() as u32,
        texture_bytes: 0,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Matrix4, Vector3};
    use thyllore_model_core::MeshMorph;
    use thyllore_vulkan_core::resource::mesh_buffer::MeshBuffer;

    use super::*;
    use crate::animation::SkinData;
    use crate::ecs::component::{SpringChain, SpringJointParam};

    fn make_chain_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("chain");
        let root = skeleton.add_bone("Hips", None);
        let spine = skeleton.add_bone("Spine", Some(root));
        let chest = skeleton.add_bone("Chest", Some(spine));
        skeleton.bones[root as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));
        skeleton.bones[spine as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 0.5, 0.0));
        skeleton.bones[chest as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 0.25, 0.1));
        skeleton
    }

    fn make_mesh(triangle_count: usize, source_mesh: &str, skin: Option<SkinData>) -> MeshBuffer {
        let mut mesh = MeshBuffer::default();
        mesh.vertex_data.indices = vec![0; triangle_count * 3];
        mesh.morph = MeshMorph {
            source_mesh: source_mesh.to_string(),
            channels: Vec::new(),
        };
        mesh.skin_data = skin;
        mesh
    }

    #[test]
    fn test_count_avatar_stats() {
        let graphics = GraphicsResources {
            meshes: vec![
                make_mesh(10, "Face", Some(SkinData::default())),
                make_mesh(4, "Face", None),
                make_mesh(2, "", Some(SkinData::default())),
            ],
            mesh_material_ids: vec![0, 1, 1],
            ..Default::default()
        };
        let setup = SpringBoneSetup {
            chains: vec![SpringChain {
                joints: vec![SpringJointParam::default(), SpringJointParam::default()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let skeleton = make_chain_skeleton();

        let stats = count_avatar_stats(&graphics, Some(&skeleton), Some(&setup));

        assert_eq!(stats.triangles, 16);
        assert_eq!(stats.meshes, 3);
        assert_eq!(stats.skinned_meshes, 2);
        assert_eq!(stats.materials, 2);
        assert_eq!(stats.morph_meshes, 1);
        assert_eq!(stats.bones, 3);
        assert_eq!(stats.spring_chains, 1);
        assert_eq!(stats.spring_transforms, 2);
        assert_eq!(stats.spring_colliders, 0);
        assert_eq!(stats.texture_bytes, 0);
    }

    #[test]
    fn test_count_avatar_stats_without_skeleton_or_springs() {
        let graphics = GraphicsResources::default();

        let stats = count_avatar_stats(&graphics, None, None);

        assert_eq!(stats.triangles, 0);
        assert_eq!(stats.bones, 0);
        assert_eq!(stats.spring_chains, 0);
    }
}
