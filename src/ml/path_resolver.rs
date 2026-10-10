use crate::ecs::component::InferenceActorSetup;
use crate::ecs::world::{EntityBuilder, World};
use crate::ml::{
    CurveCopilotMode, FeedbackSenderHandle, InferenceModelKind, CURVE_COPILOT_ACTOR_ID,
};

#[cfg(feature = "ml")]
fn spawn_curve_copilot_actor(world: &mut World) {
    let Some(model_path) = resolve_curve_copilot_model_path() else {
        return;
    };

    let mode = world
        .get_resource::<CurveCopilotMode>()
        .map(|mode| *mode)
        .unwrap_or_default();
    log!("Curve copilot mode: {:?}", mode);

    if mode.sends_feedback() {
        if let Some(handle) = FeedbackSenderHandle::spawn_from_env(&model_path) {
            world.insert_resource(handle);
        }
    }

    EntityBuilder::new(world).with_inference_actor(InferenceActorSetup {
        actor_id: CURVE_COPILOT_ACTOR_ID,
        model_path,
        model_kind: InferenceModelKind::CurveCopilot,
        enabled: true,
    });
}

#[cfg(feature = "ml")]
crate::startup_hook!("CurveCopilotActor", Ml, spawn_curve_copilot_actor);

pub fn resolve_curve_copilot_model_path() -> Option<String> {
    if let Some(path) = thyllore_ml_core::model_path::resolve_v2_curve_copilot_model_path() {
        log!("Using v2 curve_copilot model: {}", path);
        return Some(path);
    }

    log_warn!(
        "v2 curve_copilot model not found. For development, set {} to your SharedData \
         directory so that {}/{} exists. HuggingFace download from {} is not yet implemented.",
        crate::paths::SHARED_DATA_ENV_VAR,
        crate::paths::EXPORTS_SUBDIR,
        crate::paths::V2_CURVE_COPILOT_FILENAME,
        crate::paths::HUGGINGFACE_CURVE_COPILOT_REPO
    );
    None
}
