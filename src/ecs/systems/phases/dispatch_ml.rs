#[cfg(feature = "ml")]
pub fn dispatch_curve_suggestion_events(
    events: &[crate::ecs::events::UIEvent],
    world: &mut crate::ecs::world::World,
) {
    use crate::ecs::events::UIEvent;
    use crate::ecs::resource::{
        BoneNameTokenCache, BoneTopologyCache, ClipLibrary, CurveSuggestionState,
        InferenceActorState, TimelineState,
    };
    use crate::ecs::systems::{
        curve_suggestion_apply, curve_suggestion_dismiss, curve_suggestion_submit,
    };
    use crate::ml::CURVE_COPILOT_ACTOR_ID;

    for event in events {
        match event {
            UIEvent::CurveSuggestionRequest {
                bone_id,
                property_type,
            } => {
                let timeline_state = world.resource::<TimelineState>();
                let clip_id = timeline_state.current_clip_id;
                let current_time = timeline_state.current_time;
                drop(timeline_state);

                let clip_library = world.resource::<ClipLibrary>();
                let clip_info = clip_id
                    .and_then(|id| clip_library.get(id))
                    .and_then(|clip| {
                        clip.tracks
                            .get(bone_id)
                            .map(|track| (track.get_curve(*property_type).clone(), clip.duration))
                    });
                drop(clip_library);

                if let Some((curve, clip_duration)) = clip_info {
                    let topology_cache = world.resource::<BoneTopologyCache>();
                    let name_token_cache = world.resource::<BoneNameTokenCache>();
                    let mut suggestion_state = world.resource_mut::<CurveSuggestionState>();
                    let mut inference_state = world.resource_mut::<InferenceActorState>();
                    curve_suggestion_submit(
                        &mut suggestion_state,
                        &mut inference_state,
                        CURVE_COPILOT_ACTOR_ID,
                        &curve,
                        *property_type,
                        *bone_id,
                        clip_duration,
                        current_time,
                        &topology_cache,
                        &name_token_cache,
                    );
                }
            }

            UIEvent::CurveSuggestionAccept => {
                let suggestion = {
                    let state = world.resource::<CurveSuggestionState>();
                    state.suggestions.first().cloned()
                };

                if let Some(suggestion) = suggestion {
                    let timeline_state = world.resource::<TimelineState>();
                    let clip_id = timeline_state.current_clip_id;
                    drop(timeline_state);

                    if let Some(cid) = clip_id {
                        let mut clip_library = world.resource_mut::<ClipLibrary>();
                        if let Some(clip) = clip_library.get_mut(cid) {
                            if let Some(track) = clip.tracks.get_mut(&suggestion.bone_id) {
                                let curve = track.get_curve_mut(suggestion.property_type);
                                curve_suggestion_apply(&suggestion, curve);
                            }
                        }
                    }

                    let mut state = world.resource_mut::<CurveSuggestionState>();
                    curve_suggestion_dismiss(&mut state);
                }
            }

            UIEvent::CurveSuggestionDismiss => {
                let mut state = world.resource_mut::<CurveSuggestionState>();
                curve_suggestion_dismiss(&mut state);
            }

            _ => {}
        }
    }
}

#[cfg(feature = "text-to-motion")]
pub fn dispatch_text_to_motion_events(
    events: &[crate::ecs::events::UIEvent],
    world: &mut crate::ecs::world::World,
    assets: &mut crate::asset::AssetStorage,
) {
    use crate::ecs::events::UIEvent;
    use crate::ecs::resource::{ClipLibrary, TextToMotionState, TimelineState};
    use crate::ecs::systems::{text_to_motion_cancel, text_to_motion_submit};
    use crate::grpc::GrpcThreadHandle;

    const DEFAULT_ENDPOINT: &str = "http://localhost:50051";

    for event in events {
        match event {
            UIEvent::TextToMotionGenerate {
                prompt,
                duration_seconds,
            } => {
                if !world.contains_resource::<GrpcThreadHandle>() {
                    let handle = GrpcThreadHandle::spawn(DEFAULT_ENDPOINT);
                    world.insert_resource(handle);
                    crate::log!("TextToMotion: spawned gRPC thread ({})", DEFAULT_ENDPOINT);
                }

                let handle = world.get_resource::<GrpcThreadHandle>();
                let mut state = world.resource_mut::<TextToMotionState>();

                if let Some(handle) = handle {
                    text_to_motion_submit(&mut state, &*handle, prompt, *duration_seconds);
                }
            }

            UIEvent::TextToMotionApply => {
                let clip = {
                    let mut state = world.resource_mut::<TextToMotionState>();
                    state.generated_clip.take()
                };

                if let Some(clip) = clip {
                    let mut clip_library = world.resource_mut::<ClipLibrary>();
                    let new_id =
                        crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                            &mut clip_library,
                            assets,
                            clip,
                        );
                    drop(clip_library);

                    let mut timeline = world.resource_mut::<TimelineState>();
                    timeline.current_clip_id = Some(new_id);

                    let mut state = world.resource_mut::<TextToMotionState>();
                    text_to_motion_cancel(&mut state);

                    crate::log!("TextToMotion: applied clip (id={})", new_id);
                }
            }

            UIEvent::TextToMotionCancel => {
                let mut state = world.resource_mut::<TextToMotionState>();
                text_to_motion_cancel(&mut state);
                crate::log!("TextToMotion: cancelled");
            }

            _ => {}
        }
    }
}
