use crate::ecs::resource::{TextToMeshState, TextToMeshStatus};
use crate::grpc::{GrpcRequest, GrpcThreadHandle, TextToMeshRequest};

pub fn text_to_mesh_submit(
    state: &mut TextToMeshState,
    handle: &GrpcThreadHandle,
    prompt: String,
    target_faces: u32,
    seed: u32,
) {
    state.status = TextToMeshStatus::Generating;
    state.last_prompt = prompt.clone();
    state.glb_data = None;
    state.error_message = None;
    state.generation_time_ms = None;
    state.vertex_count = None;
    state.face_count = None;
    state.intermediate_image_png = None;

    handle.send(GrpcRequest::GenerateMesh(TextToMeshRequest {
        prompt,
        target_faces,
        seed,
    }));

    log!(
        "TextToMesh: submitted '{}' (faces={}, seed={})",
        state.last_prompt,
        target_faces,
        seed
    );
}

pub fn text_to_mesh_cancel(state: &mut TextToMeshState) {
    state.status = TextToMeshStatus::Idle;
    state.glb_data = None;
    state.error_message = None;
    state.generation_time_ms = None;
    state.vertex_count = None;
    state.face_count = None;
    state.intermediate_image_png = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_to_mesh_cancel() {
        let mut state = TextToMeshState::default();
        state.status = TextToMeshStatus::Generating;
        state.error_message = Some("test error".to_string());

        text_to_mesh_cancel(&mut state);

        assert_eq!(state.status, TextToMeshStatus::Idle);
        assert!(state.glb_data.is_none());
        assert!(state.error_message.is_none());
    }
}
