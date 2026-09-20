use crate::ecs::component::{FlameBaked, FlameEffect, FlameTemporalAccum, FlameTrail};
use crate::ecs::resource::{FlameDumpSink, FlameHistorySnapshotState};
use crate::ecs::FrameContext;

use super::{
    accumulate_flame_history, field_manifest_sync, flame_bone_attach_sync, flame_dump_system,
    flame_time_advance, flame_trail_advance,
};

crate::frame_prep_hook!("flame", Advance, flame_advance);
crate::frame_prep_hook!("flame", Accumulate, flame_accumulate);

fn flame_advance(ctx: &mut FrameContext) {
    flame_bone_attach_sync(ctx);
    flame_time_advance(ctx);
    field_manifest_sync(ctx);
    flame_trail_advance(ctx);
}

fn flame_accumulate(ctx: &mut FrameContext) {
    dump_flame_frame(ctx);
    accumulate_flame_history(&mut ctx.world);
}

fn dump_flame_frame(ctx: &mut FrameContext) {
    let (Some(mut sink), Some(temporal)) = (
        ctx.world.get_resource_mut::<FlameDumpSink>(),
        ctx.world.get_resource::<FlameHistorySnapshotState>(),
    ) else {
        return;
    };

    let flame_entities = ctx.world.entities_with::<FlameEffect>();
    let effects: Vec<(FlameEffect, FlameBaked, FlameTemporalAccum)> = flame_entities
        .iter()
        .filter_map(|e| {
            let effect = ctx.world.get_component::<FlameEffect>(*e).cloned()?;
            let baked = ctx
                .world
                .get_component::<FlameBaked>(*e)
                .cloned()
                .unwrap_or_default();
            let temporal_accum = ctx
                .world
                .get_component::<FlameTemporalAccum>(*e)
                .cloned()
                .unwrap_or_default();
            Some((effect, baked, temporal_accum))
        })
        .collect();
    let trails: Vec<Option<FlameTrail>> = flame_entities
        .iter()
        .map(|e| ctx.world.get_component::<FlameTrail>(*e).cloned())
        .collect();

    flame_dump_system(&mut sink, &*temporal, &effects, &trails);
}
