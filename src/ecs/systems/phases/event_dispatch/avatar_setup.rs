use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::systems::{
    add_spring_chains_by_prefix, export_unity_avatar, open_avatar_setup, save_humanoid_mapping,
    save_material_textures, set_avatar_rank_platform, set_humanoid_role, set_material_texture,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn dispatch_avatar_setup_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    for event in events {
        match event {
            UIEvent::OpenAvatarSetup => open_avatar_setup(world),
            UIEvent::SetHumanoidRole { role, bone } => set_humanoid_role(world, *role, *bone),
            UIEvent::SaveHumanoidMapping => save_humanoid_mapping(world),
            UIEvent::SetAvatarRankPlatform(platform) => set_avatar_rank_platform(world, *platform),
            UIEvent::ExportUnityAvatar => export_unity_avatar(world, assets, graphics),
            UIEvent::AddSpringChainsByPrefix { prefix } => {
                add_spring_chains_by_prefix(world, assets, prefix)
            }
            UIEvent::ClearMaterialTexture { material } => {
                set_material_texture(world, material, None)
            }
            UIEvent::SaveMaterialTextures => save_material_textures(world),
            _ => {}
        }
    }
}
