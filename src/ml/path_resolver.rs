pub fn resolve_curve_copilot_model_path() -> Option<String> {
    if let Some(path) = thyllore_ml_core::model_path::resolve_v2_curve_copilot_model_path() {
        log!("Using v2 curve_copilot model: {}", path);
        return Some(path);
    }

    log_warn!(
        "v2 curve_copilot model not found. For development, set {} to your SharedData \
         directory so that {}/{} exists. HuggingFace download from {} is not yet implemented.",
        thyllore_ml_core::model_path::SHARED_DATA_ENV_VAR,
        thyllore_ml_core::model_path::EXPORTS_SUBDIR,
        thyllore_ml_core::model_path::V2_CURVE_COPILOT_FILENAME,
        thyllore_ml_core::model_path::HUGGINGFACE_CURVE_COPILOT_REPO
    );
    None
}
