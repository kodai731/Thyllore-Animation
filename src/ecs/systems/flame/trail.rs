use crate::app::FrameContext;
use crate::ecs::component::{FlameEffect, FlameTrail};
use crate::ecs::resource::{FrameClock, TimelineState};
use thyllore_effect_core::advance_flame_trail;

pub fn flame_trail_advance(ctx: &mut FrameContext) {
    let flame_entities = ctx.world.query_flames();
    let fixed_step_frame = ctx
        .world
        .get_resource::<FrameClock>()
        .filter(|clock| clock.is_fixed())
        .map(|clock| clock.frame as f32);
    let timeline_current_time = if fixed_step_frame.is_some() {
        None
    } else {
        ctx.world
            .get_resource::<TimelineState>()
            .map(|ts| ts.current_time)
    };

    for &entity in &flame_entities {
        // Read position before mutable borrow
        let position = match ctx.world.get_component::<FlameEffect>(entity) {
            Some(effect) => effect.position,
            None => continue,
        };
        let trail = match ctx.world.get_component_mut::<FlameTrail>(entity) {
            Some(t) => t,
            None => continue,
        };
        if !trail.state.enabled {
            continue;
        }
        let delta = if let Some(current_frame) = fixed_step_frame {
            let last_frame = trail.last_timeline_time.unwrap_or(current_frame);
            current_frame - last_frame
        } else if let Some(timeline_time) = timeline_current_time {
            let last = trail.last_timeline_time.unwrap_or(timeline_time);
            timeline_time - last
        } else {
            ctx.delta_time
        };
        let pos: [f32; 3] = [position.x, position.y, position.z];
        advance_flame_trail(&mut trail.state, pos, delta);
        trail.last_timeline_time = fixed_step_frame.or(timeline_current_time);
    }
}
