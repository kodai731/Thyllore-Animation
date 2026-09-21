#[cfg(feature = "ml")]
use crate::ecs::component::InferenceActorSetup;
#[cfg(feature = "ml")]
use crate::ecs::resource::{CurveSuggestionState, InferenceActorState};
#[cfg(feature = "ml")]
use crate::ecs::systems::curve_copilot::curve_suggestion_poll_results;
#[cfg(feature = "ml")]
use crate::ecs::systems::inference_actor_systems::{
    inference_actor_initialize, inference_actor_poll,
};
#[cfg(feature = "ml")]
use crate::ml::FeedbackSenderHandle;

use crate::ecs::resource::{ClipLibrary, FrameClock, HierarchyState, TimelineState};
use crate::ecs::systems::clip_library_systems::clip_library_sync_dirty;
use crate::ecs::systems::timeline_systems::{schedule_extent_seconds, timeline_update};
use crate::ecs::world::Animator;
use crate::ecs::FrameContext;

pub fn run_timeline_phase(ctx: &mut FrameContext) {
    update_timeline(ctx);

    #[cfg(feature = "ml")]
    run_inference_actor_phase(ctx);
}

fn update_timeline(ctx: &mut FrameContext) {
    if !ctx.world.contains_resource::<TimelineState>() {
        return;
    }
    if !ctx.world.contains_resource::<ClipLibrary>() {
        return;
    }

    let selected_entity = {
        let hierarchy_state = ctx.world.resource::<HierarchyState>();
        hierarchy_state.selected_entity
    };

    {
        let mut timeline_state = ctx.world.resource_mut::<TimelineState>();
        timeline_state.target_entity = selected_entity;
    }

    let schedule_extent = schedule_extent_seconds(ctx.world);
    let mut timeline_state = ctx.world.resource_mut::<TimelineState>();
    timeline_state.schedule_extent_seconds = schedule_extent;
    let clip_library = ctx.world.resource::<ClipLibrary>();
    let timeline_delta = ctx
        .world
        .resource::<FrameClock>()
        .fixed_delta_seconds()
        .unwrap_or(ctx.delta_time);
    timeline_update(&mut timeline_state, &*clip_library, timeline_delta);
    drop(clip_library);
    drop(timeline_state);

    sync_timeline_to_all_animators(ctx);

    sync_editable_clips_to_registry(ctx);
}

fn sync_timeline_to_all_animators(ctx: &mut FrameContext) {
    let timeline_snapshot = {
        let timeline = ctx.world.resource::<TimelineState>();
        (
            timeline.current_time,
            timeline.playing,
            timeline.speed,
            timeline.looping,
        )
    };

    let animated_entities = ctx.world.query_animated();

    for entity in animated_entities {
        if let Some(animator) = ctx.world.get_component_mut::<Animator>(entity) {
            animator.time = timeline_snapshot.0;
            animator.playing = timeline_snapshot.1;
            animator.speed = timeline_snapshot.2;
            animator.looping = timeline_snapshot.3;
        }
    }
}

fn sync_editable_clips_to_registry(ctx: &mut FrameContext) {
    let mut clip_library = ctx.world.resource_mut::<ClipLibrary>();
    clip_library_sync_dirty(&mut clip_library, ctx.assets);
}

#[cfg(feature = "ml")]
fn run_inference_actor_phase(ctx: &mut FrameContext) {
    if !ctx.world.contains_resource::<InferenceActorState>() {
        return;
    }

    let setups: Vec<_> = ctx
        .world
        .iter_components::<InferenceActorSetup>()
        .map(|(_, setup)| setup.clone())
        .collect();

    let mut state = ctx.world.resource_mut::<InferenceActorState>();
    for setup in &setups {
        inference_actor_initialize(setup, &mut state);
    }
    inference_actor_poll(&mut state);

    if ctx.world.contains_resource::<CurveSuggestionState>() {
        let feedback_sender = ctx.world.get_resource::<FeedbackSenderHandle>();
        let mut suggestion_state = ctx.world.resource_mut::<CurveSuggestionState>();
        curve_suggestion_poll_results(
            &mut suggestion_state,
            &mut state,
            feedback_sender.as_ref().map(|sender| &**sender),
        );
    }
}
