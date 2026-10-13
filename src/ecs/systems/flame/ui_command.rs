#[derive(Clone, Debug)]
pub enum FlameUiCommand {
    UpdateBaked(Box<thyllore_effect_core::FlameBaked>),
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
    UpdateTrailEnabled(bool),
    UpdateTrailFade(f32),
    DumpWallProbe {
        viewport_size: [f32; 2],
    },
}
