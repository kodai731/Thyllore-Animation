use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::WaterRenderSettings;
use crate::ecs::world::Entity;

#[derive(Clone, Debug)]
pub enum WaterUiCommand {
    UpdateEffect {
        entity: Entity,
        effect: Box<WaterTorusEffect>,
    },
    ApplyPreset(String),
    UpdateRenderSettings(WaterRenderSettings),
}
