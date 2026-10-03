use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::vrchat::rank::Platform;

use crate::asset::AssetStorage;
use crate::ecs::systems::{
    add_spring_chains_by_prefix, export_unity_avatar, open_avatar_setup, save_humanoid_mapping,
    save_material_textures, set_avatar_rank_platform, set_humanoid_role, set_material_texture,
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

crate::ui_event!(AvatarSetupEvent => dispatch_avatar_setup_events, Normal);

pub fn dispatch_avatar_setup_events(
    events: Vec<AvatarSetupEvent>,
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    for event in events {
        match event {
            AvatarSetupEvent::OpenAvatarSetup => open_avatar_setup(world),
            AvatarSetupEvent::SetHumanoidRole { role, bone } => {
                set_humanoid_role(world, role, bone)
            }
            AvatarSetupEvent::SaveHumanoidMapping => save_humanoid_mapping(world),
            AvatarSetupEvent::SetAvatarRankPlatform(platform) => {
                set_avatar_rank_platform(world, platform)
            }
            AvatarSetupEvent::ExportUnityAvatar => export_unity_avatar(world, assets, graphics),
            AvatarSetupEvent::AddSpringChainsByPrefix { prefix } => {
                add_spring_chains_by_prefix(world, assets, &prefix)
            }
            AvatarSetupEvent::PickMaterialTexture { .. } => {}
            AvatarSetupEvent::ClearMaterialTexture { material } => {
                set_material_texture(world, &material, None)
            }
            AvatarSetupEvent::SaveMaterialTextures => save_material_textures(world),
        }
    }
}
