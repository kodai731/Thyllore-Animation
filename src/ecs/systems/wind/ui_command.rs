use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::WindRenderSettings;
use crate::ecs::world::Entity;

#[derive(Clone)]
pub enum WindUiCommand {
    UpdateEffect {
        entity: Entity,
        effect: Box<WindTornadoEffect>,
    },
    ApplyPreset(String),
    UpdateRenderSettings(WindRenderSettings),
}
