use crate::ml::{InferenceActorId, InferenceModelKind};

#[derive(Clone, Debug)]
pub struct InferenceActorSetup {
    pub actor_id: InferenceActorId,
    pub model_path: String,
    pub model_kind: InferenceModelKind,
    pub enabled: bool,
}
