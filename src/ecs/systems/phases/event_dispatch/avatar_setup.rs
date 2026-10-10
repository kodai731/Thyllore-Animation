use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::vrchat::rank::Platform;

use crate::asset::AssetStorage;
use crate::ecs::events::{send_dialog_request, DialogRequest, UiCommand};
use crate::ecs::systems::{
    add_spring_chains_by_prefix, export_unity_avatar, invalidate_humanoid_rig, open_avatar_setup,
    save_humanoid_mapping, save_material_textures, set_avatar_rank_platform, set_humanoid_role,
    set_material_texture,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

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
    AddSpringChainsByPrefix {
        prefix: String,
    },
    PickMaterialTexture {
        material: String,
    },
    ClearMaterialTexture {
        material: String,
    },
    SaveMaterialTextures,
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
            AvatarSetupEvent::AddSpringChainsByPrefix { prefix } => {
                add_spring_chains_by_prefix(world, assets, &prefix)
            }
            AvatarSetupEvent::PickMaterialTexture { material } => {
                send_dialog_request(world, DialogRequest::PickMaterialTexture { material })
            }
            AvatarSetupEvent::ClearMaterialTexture { material } => {
                set_material_texture(world, &material, None)
            }
            AvatarSetupEvent::SaveMaterialTextures => save_material_textures(world),
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
