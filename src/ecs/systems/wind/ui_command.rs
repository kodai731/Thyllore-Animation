use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::WindRenderSettings;

#[derive(Clone)]
pub enum WindUiCommand {
    UpdateEffect(Box<WindTornadoEffect>),
    ApplyPreset(String),
    UpdateRenderSettings(WindRenderSettings),
}
