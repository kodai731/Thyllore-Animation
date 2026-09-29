#[derive(Clone, Debug, Default)]
pub struct MaterialTextureState {
    pub source_model_path: String,
    pub slots: Vec<MaterialTextureSlot>,
    pub save_state: MaterialTextureSaveState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialTextureSlot {
    pub material: String,
    pub texture: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MaterialTextureSaveState {
    #[default]
    Saved,
    Edited,
}
