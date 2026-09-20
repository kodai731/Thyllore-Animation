use crate::ecs::component::FlameEffect;
use crate::ecs::resource::FlameRenderSettings;

#[derive(Clone)]
pub enum FlameUiCommand {
    UpdateEffect(Box<FlameEffect>),
    UpdateBaked(Box<thyllore_effect_core::FlameBaked>),
    ApplyPreset(String),
    ApplyTextureFit {
        path: String,
        blend: f32,
        groups: [bool; 4],
        profile: bool,
    },
    ApplyStyle {
        path: String,
        groups: [bool; 3],
    },
    SaveStyle {
        name: String,
    },
    UpdateRenderSettings(FlameRenderSettings),
    UpdateTrailEnabled(bool),
    UpdateTrailFade(f32),
    DumpWallProbe {
        viewport_size: [f32; 2],
    },
}
