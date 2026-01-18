use anyhow::Result;

use crate::ecs::context::FrameContext;

use super::phases::{
    run_animation_phase, run_input_phase, run_render_prep_phase, run_transform_phase,
};

pub unsafe fn run_frame(ctx: &mut FrameContext) -> Result<()> {
    run_input_phase(ctx)?;
    run_animation_phase(ctx)?;
    run_transform_phase(ctx)?;
    run_render_prep_phase(ctx)?;
    Ok(())
}
