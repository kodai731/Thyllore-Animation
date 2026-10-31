use crate::ecs::component::CapsuleFitParams;
use thyllore_avatar_core::cloth_setup::components::hidden_mask::HiddenMaskParams;
use thyllore_avatar_core::cloth_setup::components::submesh_role::{SubmeshKey, SubmeshRole};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::vrchat::rank::Platform;

use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::events::{send_dialog_request, DialogRequest, UiCommand};
use crate::ecs::resource::OutputQueue;
use crate::ecs::systems::{
    add_spring_chains_under, compute_hidden_triangles, export_unity_avatar, fit_body_colliders,
    invalidate_humanoid_rig, open_avatar_setup, save_humanoid_mapping, save_material_textures,
    select_submesh_entity, set_avatar_rank_platform, set_chain_gen_settings, set_humanoid_role,
    set_material_texture, set_submesh_role,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_avatar_core::cloth_setup::components::chain_gen::{ChainGenRequest, ChainGenSettings};

#[derive(Clone, Debug)]
pub enum AvatarSetupEvent {
    OpenAvatarSetup,
    SetHumanoidRole {
        role: HumanoidRole,
        bone: Option<usize>,
    },
    SaveHumanoidMapping,
    SetAvatarRankPlatform(Platform),
    ExportUnityAvatar,
    AddSpringChainsUnder {
        root: BoneId,
        min_joints: u32,
    },
    FitBodyColliders {
        params: CapsuleFitParams,
    },
    PickMaterialTexture {
        material: String,
    },
    ClearMaterialTexture {
        material: String,
    },
    SaveMaterialTextures,
    SetSubmeshRole {
        key: SubmeshKey,
        role: SubmeshRole,
    },
    SelectSubmesh {
        key: SubmeshKey,
    },
    GenerateChainBones {
        request: ChainGenRequest,
    },
    SetChainGenSettings {
        settings: ChainGenSettings,
    },
    ComputeHiddenTriangles {
        params: HiddenMaskParams,
    },
}

impl UiCommand for AvatarSetupEvent {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        match *self {
            AvatarSetupEvent::OpenAvatarSetup => open_avatar_setup(world),
            AvatarSetupEvent::SetHumanoidRole { role, bone } => {
                set_humanoid_role(world, role, bone)
            }
            AvatarSetupEvent::SaveHumanoidMapping => {
                save_humanoid_mapping(world);
                invalidate_humanoid_rig(world);
            }
            AvatarSetupEvent::SetAvatarRankPlatform(platform) => {
                set_avatar_rank_platform(world, platform)
            }
            AvatarSetupEvent::ExportUnityAvatar => export_unity_avatar(world, assets, graphics),
            AvatarSetupEvent::AddSpringChainsUnder { root, min_joints } => {
                add_spring_chains_under(world, assets, root, min_joints)
            }
            AvatarSetupEvent::FitBodyColliders { params } => {
                fit_body_colliders(world, assets, graphics, &params)
            }
            AvatarSetupEvent::PickMaterialTexture { material } => {
                send_dialog_request(world, DialogRequest::PickMaterialTexture { material })
            }
            AvatarSetupEvent::ClearMaterialTexture { material } => {
                set_material_texture(world, &material, None)
            }
            AvatarSetupEvent::SaveMaterialTextures => save_material_textures(world),
            AvatarSetupEvent::SetSubmeshRole { key, role } => set_submesh_role(world, key, role),
            AvatarSetupEvent::SelectSubmesh { key } => {
                select_submesh_entity(world, assets, graphics, &key)
            }
            AvatarSetupEvent::GenerateChainBones { request } => {
                world
                    .resource_mut::<OutputQueue>()
                    .push(crate::ecs::resource::OutputCommand::GenerateChainBones { request });
            }
            AvatarSetupEvent::SetChainGenSettings { settings } => {
                set_chain_gen_settings(world, settings)
            }
            AvatarSetupEvent::ComputeHiddenTriangles { params } => {
                compute_hidden_triangles(world, assets, graphics, &params)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::EventQueue;

    #[test]
    fn test_pick_material_texture_sends_dialog_request() {
        let mut world = World::new();
        world.insert_resource(EventQueue::<DialogRequest>::default());
        let mut assets = AssetStorage::new();
        let graphics = GraphicsResources::default();

        Box::new(AvatarSetupEvent::PickMaterialTexture {
            material: "test_material".to_string(),
        })
        .apply(&mut world, &mut assets, &graphics);

        let requests: Vec<DialogRequest> = world
            .resource_mut::<EventQueue<DialogRequest>>()
            .drain()
            .collect();
        assert_eq!(
            requests,
            vec![DialogRequest::PickMaterialTexture {
                material: "test_material".to_string(),
            }]
        );
    }
}
