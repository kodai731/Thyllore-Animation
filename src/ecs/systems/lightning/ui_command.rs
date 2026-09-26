use crate::ecs::component::LightningEffect;
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::world::Entity;

#[derive(Clone)]
pub enum LightningUiCommand {
    UpdateEffect {
        entity: Entity,
        effect: Box<LightningEffect>,
    },
    ApplyPreset(String),
    AddTarget,
    ClearTarget,
    AddWaypoint,
    RemoveWaypoint(usize),
    UpdateRenderSettings(LightningRenderSettings),
}
