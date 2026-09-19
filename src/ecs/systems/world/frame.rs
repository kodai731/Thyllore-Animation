use anyhow::Result;

use crate::ecs::context::EcsContext;
use crate::ecs::resource::UpdatePhaseTimings;
use crate::ecs::systems::phases::{
    collect_mesh_positions, run_animation_phase_ecs, run_animation_phase_gpu, run_first_phase,
    run_input_phase, run_onion_skin_phase, run_render_prep_phase, run_timeline_phase,
    run_transform_phase_ecs, run_transform_phase_gpu,
};
use crate::ecs::FrameContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePhase {
    First,
    Input,
    Transform,
    Timeline,
    Animation,
    OnionSkin,
    RenderPrep,
    EventDispatch,
    Last,
}

impl FramePhase {
    pub fn name(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Input => "input",
            Self::Transform => "transform",
            Self::Timeline => "timeline",
            Self::Animation => "animation",
            Self::OnionSkin => "onion_skin",
            Self::RenderPrep => "render_prep",
            Self::EventDispatch => "event_dispatch",
            Self::Last => "last",
        }
    }
}

pub const FRAME_SCHEDULE: [FramePhase; 9] = [
    FramePhase::First,
    FramePhase::Input,
    FramePhase::Transform,
    FramePhase::Timeline,
    FramePhase::Animation,
    FramePhase::OnionSkin,
    FramePhase::RenderPrep,
    FramePhase::EventDispatch,
    FramePhase::Last,
];

struct Carry {
    updated_meshes: Vec<usize>,
}

impl Carry {
    fn new() -> Self {
        Self {
            updated_meshes: Vec::new(),
        }
    }
}

pub unsafe fn run_frame(ctx: &mut FrameContext) -> Result<()> {
    let mut stages: Vec<(String, f32)> = Vec::new();
    let mut carry = Carry::new();

    for &phase in FRAME_SCHEDULE
        .iter()
        .take_while(|phase| **phase != FramePhase::EventDispatch)
    {
        let t = std::time::Instant::now();
        run_update_phase(&phase, ctx, &mut carry)?;
        stages.push((phase.name().to_string(), t.elapsed().as_secs_f32() * 1000.0));
    }

    ctx.world.insert_resource(UpdatePhaseTimings { stages });
    Ok(())
}

unsafe fn run_update_phase(
    phase: &FramePhase,
    ctx: &mut FrameContext,
    carry: &mut Carry,
) -> Result<()> {
    match phase {
        FramePhase::First => {
            run_first_phase(ctx.world);
        }
        FramePhase::Input => {
            let mesh_positions = collect_mesh_positions(ctx.graphics);
            let mut ecs_ctx = EcsContext {
                time: ctx.time,
                delta_time: ctx.delta_time,
                image_index: ctx.image_index,
                swapchain_extent: ctx.swapchain_extent,
                world: ctx.world,
                assets: ctx.assets,
                mesh_positions,
            };
            run_input_phase(&mut ecs_ctx)?;
        }
        FramePhase::Transform => {
            let mut ecs_ctx = EcsContext {
                time: ctx.time,
                delta_time: ctx.delta_time,
                image_index: ctx.image_index,
                swapchain_extent: ctx.swapchain_extent,
                world: ctx.world,
                assets: ctx.assets,
                mesh_positions: Vec::new(),
            };
            run_transform_phase_ecs(&mut ecs_ctx);
        }
        FramePhase::Timeline => {
            run_timeline_phase(ctx);
        }
        FramePhase::Animation => {
            let animation_updates = run_animation_phase_ecs(ctx);
            run_animation_phase_gpu(ctx, &animation_updates)?;
            carry.updated_meshes = animation_updates.updated_meshes;
        }
        FramePhase::OnionSkin => {
            run_onion_skin_phase(ctx, &carry.updated_meshes)?;
        }
        FramePhase::RenderPrep => {
            run_transform_phase_gpu(ctx)?;
            run_render_prep_phase(ctx)?;
        }
        FramePhase::EventDispatch | FramePhase::Last => {
            unreachable!("platform の UI 後 / App::after_present から入る");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_update_phases_run_before_event_dispatch_and_last() {
        assert_eq!(
            FRAME_SCHEDULE,
            [
                FramePhase::First,
                FramePhase::Input,
                FramePhase::Transform,
                FramePhase::Timeline,
                FramePhase::Animation,
                FramePhase::OnionSkin,
                FramePhase::RenderPrep,
                FramePhase::EventDispatch,
                FramePhase::Last,
            ]
        );
    }
}
