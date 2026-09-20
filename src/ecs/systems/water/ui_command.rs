use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::WaterRenderSettings;

#[derive(Clone)]
pub enum WaterUiCommand {
    UpdateEffect(Box<WaterTorusEffect>),
    ApplyPreset(String),
    UpdateRenderSettings(WaterRenderSettings),
}
