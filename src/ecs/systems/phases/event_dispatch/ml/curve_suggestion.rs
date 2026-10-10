#![cfg(feature = "ml")]

use crate::animation::editable::PropertyType;
use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum CurveSuggestionEvent {
    Request {
        bone_id: BoneId,
        property_type: PropertyType,
    },
    Accept,
    Dismiss,
}

impl UiCommand for CurveSuggestionEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        dispatch_curve_suggestion_events(&[*self], world, assets);
    }
}

fn dispatch_curve_suggestion_events(
    events: &[CurveSuggestionEvent],
    world: &mut crate::ecs::world::World,
    _assets: &crate::asset::AssetStorage,
) {
    use crate::ecs::resource::{
        ClipLibrary, CurveSuggestionState, InferenceActorState, TimelineState,
    };
    use crate::ecs::systems::{
        curve_suggestion_apply, curve_suggestion_dismiss, curve_suggestion_submit,
        CurveSuggestionInputs,
    };
    use crate::ml::{CurveCopilotMode, CURVE_COPILOT_ACTOR_ID};

    for event in events {
        match event {
            CurveSuggestionEvent::Request {
                bone_id,
                property_type,
            } => {
                let timeline_state = world.resource::<TimelineState>();
                let clip_id = timeline_state.current_clip_id;
                let current_time = timeline_state.current_time;
                drop(timeline_state);

                let Some(clip_id) = clip_id else {
                    continue;
                };

                let clip = {
                    let clip_library = world.resource::<ClipLibrary>();
                    clip_library.get(clip_id).cloned()
                };
                let Some(clip) = clip else {
                    continue;
                };

                let mode = world
                    .get_resource::<CurveCopilotMode>()
                    .map(|mode| *mode)
                    .unwrap_or_default();
                let mut suggestion_state = world.resource_mut::<CurveSuggestionState>();
                let mut inference_state = world.resource_mut::<InferenceActorState>();
                curve_suggestion_submit(
                    &mut suggestion_state,
                    &mut inference_state,
                    CURVE_COPILOT_ACTOR_ID,
                    CurveSuggestionInputs { clip: &clip },
                    *property_type,
                    *bone_id,
                    current_time,
                    mode,
                );
            }

            CurveSuggestionEvent::Accept => {
                let suggestions = {
                    let state = world.resource::<CurveSuggestionState>();
                    state.suggestions.clone()
                };

                if !suggestions.is_empty() {
                    let timeline_state = world.resource::<TimelineState>();
                    let clip_id = timeline_state.current_clip_id;
                    drop(timeline_state);

                    if let Some(cid) = clip_id {
                        let mut clip_library = world.resource_mut::<ClipLibrary>();
                        if let Some(clip) = clip_library.get_mut(cid) {
                            for suggestion in &suggestions {
                                if let Some(track) = clip.tracks.get_mut(&suggestion.bone_id) {
                                    let curve = track.get_curve_mut(suggestion.property_type);
                                    curve_suggestion_apply(suggestion, curve);
                                }
                            }
                        }
                    }

                    let mut state = world.resource_mut::<CurveSuggestionState>();
                    curve_suggestion_dismiss(&mut state);
                }
            }

            CurveSuggestionEvent::Dismiss => {
                let mut state = world.resource_mut::<CurveSuggestionState>();
                curve_suggestion_dismiss(&mut state);
            }

            _ => {}
        }
    }
}
