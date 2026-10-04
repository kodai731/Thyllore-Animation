use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use crate::animation::editable::SourceClipId;
use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::systems::motion_recipe_systems::{rebake_recipe_clip, set_recipe_pose_rotation};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum MotionRecipeEvent {
    SetPoseRotation {
        clip: SourceClipId,
        role: HumanoidRole,
        pose_index: usize,
        euler: [f32; 3],
    },
    RebakeFromRecipe {
        clip: SourceClipId,
    },
}

impl UiCommand for MotionRecipeEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            MotionRecipeEvent::SetPoseRotation {
                clip,
                role,
                pose_index,
                euler,
            } => {
                if let Err(e) =
                    set_recipe_pose_rotation(world, assets, clip, role, pose_index, euler)
                {
                    msg_error!("Recipe edit failed: {:#}", e);
                }
            }
            MotionRecipeEvent::RebakeFromRecipe { clip } => {
                if let Err(e) = rebake_recipe_clip(world, assets, clip) {
                    msg_error!("Recipe edit failed: {:#}", e);
                }
            }
        }
    }
}
