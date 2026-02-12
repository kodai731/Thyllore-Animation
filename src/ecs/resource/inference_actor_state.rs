use std::collections::HashMap;

use crate::ml::{
    InferenceActorId, InferenceRequestId, InferenceResult, InferenceThreadHandle,
};

pub struct ActorRuntime {
    pub thread_handle: InferenceThreadHandle,
    pub enabled: bool,
}

pub struct InferenceActorState {
    pub actors: HashMap<InferenceActorId, ActorRuntime>,
    pub pending_results: Vec<InferenceResult>,
    pub next_request_id: InferenceRequestId,
}

impl Default for InferenceActorState {
    fn default() -> Self {
        Self {
            actors: HashMap::new(),
            pending_results: Vec::new(),
            next_request_id: 1,
        }
    }
}
