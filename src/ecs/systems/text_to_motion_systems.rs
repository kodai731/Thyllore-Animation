use std::collections::HashMap;

use crate::animation::BoneId;
use crate::ecs::resource::{TextToMotionState, TextToMotionStatus};
use crate::grpc::{
    convert_motion_response_to_clip, GrpcRequest, GrpcResponse,
    GrpcThreadHandle, TextToMotionRequest,
};

pub fn text_to_motion_submit(
    state: &mut TextToMotionState,
    handle: &GrpcThreadHandle,
    prompt: &str,
    duration_seconds: f32,
) {
    state.status = TextToMotionStatus::Generating;
    state.last_prompt = prompt.to_string();
    state.last_duration = duration_seconds;
    state.error_message = None;
    state.generated_clip = None;
    state.generation_time_ms = None;
    state.model_used = None;

    let request = TextToMotionRequest {
        prompt: prompt.to_string(),
        duration_seconds,
        target_fps: 30,
    };

    handle.send(GrpcRequest::GenerateMotion(request));
    crate::log!(
        "TextToMotion: submitted '{}' (duration={}s)",
        prompt,
        duration_seconds
    );
}

pub fn text_to_motion_poll(
    state: &mut TextToMotionState,
    handle: &GrpcThreadHandle,
    bone_name_to_id: Option<&HashMap<String, BoneId>>,
) {
    if state.status != TextToMotionStatus::Generating {
        return;
    }

    let response = match handle.try_recv() {
        Some(r) => r,
        None => return,
    };

    match response {
        GrpcResponse::MotionGenerated {
            curves,
            generation_time_ms,
            model_used,
        } => {
            let bone_map = bone_name_to_id
                .cloned()
                .unwrap_or_default();

            let clip_name =
                format!("T2M: {}", truncate_prompt(&state.last_prompt, 30));

            let clip = convert_motion_response_to_clip(
                &curves,
                &clip_name,
                state.last_duration,
                &bone_map,
            );

            crate::log!(
                "TextToMotion: generated clip '{}' with {} tracks in {:.0}ms (model: {})",
                clip_name,
                clip.tracks.len(),
                generation_time_ms,
                model_used
            );

            state.status = TextToMotionStatus::Generated;
            state.generated_clip = Some(clip);
            state.generation_time_ms = Some(generation_time_ms);
            state.model_used = Some(model_used);
        }

        GrpcResponse::ServerStatus {
            ready,
            active_model,
            ..
        } => {
            state.server_ready = ready;
            state.model_used = Some(active_model);
        }

        GrpcResponse::Error { message } => {
            crate::log!("TextToMotion: error - {}", message);
            state.status = TextToMotionStatus::Error;
            state.error_message = Some(message);
        }
    }
}

pub fn text_to_motion_cancel(state: &mut TextToMotionState) {
    state.status = TextToMotionStatus::Idle;
    state.generated_clip = None;
    state.error_message = None;
    state.generation_time_ms = None;
    state.model_used = None;
}

fn truncate_prompt(prompt: &str, max_len: usize) -> String {
    if prompt.len() <= max_len {
        prompt.to_string()
    } else {
        format!("{}...", &prompt[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc::{RawAnimationCurve, RawCurveKeyframe};

    #[test]
    fn test_text_to_motion_cancel() {
        let mut state = TextToMotionState::default();
        state.status = TextToMotionStatus::Generating;
        state.error_message = Some("test error".to_string());

        text_to_motion_cancel(&mut state);

        assert_eq!(state.status, TextToMotionStatus::Idle);
        assert!(state.generated_clip.is_none());
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_truncate_prompt_short() {
        assert_eq!(truncate_prompt("hello", 30), "hello");
    }

    #[test]
    fn test_truncate_prompt_long() {
        let long = "a".repeat(50);
        let result = truncate_prompt(&long, 30);
        assert_eq!(result.len(), 33);
        assert!(result.ends_with("..."));
    }
}
