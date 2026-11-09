#[derive(Clone, Debug, Default, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct BakeForm {
    #[persist(ui(min = 1.0, max = 120.0, format = "%.0f"))]
    pub sample_fps: f32,
}

#[derive(Clone)]
pub struct ConstraintEditorState {
    pub add_type_index: usize,
    pub bake: BakeForm,
}

impl Default for ConstraintEditorState {
    fn default() -> Self {
        Self {
            add_type_index: 3,
            bake: BakeForm { sample_fps: 30.0 },
        }
    }
}
