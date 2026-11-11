use crate::ecs::component::InferenceActorSetup;
use crate::ecs::world::{EntityBuilder, World};
use crate::ml::{
    resolve_curve_copilot_model_path, CurveCopilotMode, FeedbackSenderHandle, InferenceModelKind,
    CURVE_COPILOT_ACTOR_ID,
};

crate::startup_hook!("CurveCopilotActor", Ml, spawn_curve_copilot_actor);

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
