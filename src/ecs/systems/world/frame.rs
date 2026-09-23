use anyhow::Result;

use cgmath::Matrix4;

use crate::animation::SkeletonId;
use crate::ecs::context::EcsContext;
use crate::ecs::resource::{AnimationType, UpdatePhaseTimings};
use crate::ecs::systems::phases::{
    collect_mesh_positions, run_animation_phase_ecs, run_animation_phase_gpu, run_first_phase,
    run_input_phase, run_onion_skin_phase, run_render_prep_phase, run_timeline_phase,
    run_transform_propagate_phase, run_view_phase_ecs, run_view_phase_gpu,
};
use crate::ecs::FrameContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePhase {
    EventDispatch,
    First,
    Input,
    View,
    Timeline,
    Animation,
    TransformPropagate,
    OnionSkin,
    RenderPrep,
    Last,
}

impl FramePhase {
    pub fn name(self) -> &'static str {
        match self {
            Self::EventDispatch => "event_dispatch",
            Self::First => "first",
            Self::Input => "input",
            Self::View => "view",
            Self::Timeline => "timeline",
            Self::Animation => "animation",
            Self::TransformPropagate => "transform_propagate",
            Self::OnionSkin => "onion_skin",
            Self::RenderPrep => "render_prep",
            Self::Last => "last",
        }
    }

    pub fn runs_in_update(self) -> bool {
        !matches!(self, Self::EventDispatch | Self::Last)
    }
}

pub const FRAME_SCHEDULE: [FramePhase; 10] = [
    FramePhase::EventDispatch,
    FramePhase::First,
    FramePhase::Input,
    FramePhase::View,
    FramePhase::Timeline,
    FramePhase::Animation,
    FramePhase::TransformPropagate,
    FramePhase::OnionSkin,
    FramePhase::RenderPrep,
    FramePhase::Last,
];

pub fn update_phases() -> impl Iterator<Item = FramePhase> {
    FRAME_SCHEDULE
        .iter()
        .copied()
        .filter(|phase| phase.runs_in_update())
}

struct Carry {
    updated_meshes: Vec<usize>,
    bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)>,
}

impl Carry {
    fn new() -> Self {
        Self {
            updated_meshes: Vec::new(),
            bone_transforms: None,
        }
    }
}

pub unsafe fn run_frame(ctx: &mut FrameContext) -> Result<()> {
    let mut stages: Vec<(String, f32)> = Vec::new();
    let mut carry = Carry::new();

    for phase in update_phases() {
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
        FramePhase::View => {
            let mut ecs_ctx = EcsContext {
                time: ctx.time,
                delta_time: ctx.delta_time,
                image_index: ctx.image_index,
                swapchain_extent: ctx.swapchain_extent,
                world: ctx.world,
                assets: ctx.assets,
                mesh_positions: Vec::new(),
            };
            run_view_phase_ecs(&mut ecs_ctx);
        }
        FramePhase::Timeline => {
            run_timeline_phase(ctx);
        }
        FramePhase::Animation => {
            let animation_updates = run_animation_phase_ecs(ctx);
            run_animation_phase_gpu(ctx, &animation_updates)?;
            carry.updated_meshes = animation_updates.updated_meshes;
            carry.bone_transforms = animation_updates.bone_transforms;
        }
        FramePhase::TransformPropagate => {
            run_transform_propagate_phase(ctx, carry.bone_transforms.take());
        }
        FramePhase::OnionSkin => {
            run_onion_skin_phase(ctx, &carry.updated_meshes)?;
        }
        FramePhase::RenderPrep => {
            run_view_phase_gpu(ctx)?;
            run_render_prep_phase(ctx)?;
        }
        FramePhase::EventDispatch | FramePhase::Last => {
            unreachable!("App::drive_frame runs these around update");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_dispatch_opens_the_frame_and_last_closes_it() {
        assert_eq!(FRAME_SCHEDULE.first(), Some(&FramePhase::EventDispatch));
        assert_eq!(FRAME_SCHEDULE.last(), Some(&FramePhase::Last));
    }

    #[test]
    fn update_phases_are_the_schedule_between_event_dispatch_and_last() {
        let update: Vec<FramePhase> = update_phases().collect();
        assert_eq!(update, FRAME_SCHEDULE[1..FRAME_SCHEDULE.len() - 1]);
        assert!(update.iter().all(|phase| phase.runs_in_update()));
    }

    #[test]
    fn transform_propagate_runs_right_after_animation() {
        let animation_index = FRAME_SCHEDULE
            .iter()
            .position(|phase| *phase == FramePhase::Animation)
            .unwrap();
        assert_eq!(
            FRAME_SCHEDULE[animation_index + 1],
            FramePhase::TransformPropagate
        );
    }
}
