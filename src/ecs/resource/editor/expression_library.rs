#[derive(Default)]
pub struct ExpressionLibraryState {
    pub library: thyllore_avatar_core::expression::components::preset::ExpressionLibrary,
    pub source_model_path: String,
    pub capture_name: String,
}
