use crate::animation::editable::EditableAnimationClip;
use crate::app::FrameContext;
use crate::ecs::resource::{BatchRun, ClipLibrary, TimelineState};
use crate::ecs::storage::Component;
use crate::ecs::systems::scalar_clip_systems::{find_entity_clip_id, sampled_scalar_values};
use crate::ecs::world::{Entity, Transform, World};
use thyllore_anim_core::editable::PropertyType;

/// An effect component whose local time, placement and animated scalars are refreshed once per
/// frame by `advance_effect_time`.
pub trait TimedEffect: Component {
    /// Per-frame values read from `World` resources before the components are borrowed mutably.
    type WorldInputs: Copy;

    fn entities(world: &World) -> Vec<Entity>;
    fn time_sources(world: &World, delta_time: f32) -> EffectTimeSources;
    fn collect_world_inputs(world: &World) -> Self::WorldInputs;

    fn time(&self) -> f32;
    fn time_mut(&mut self) -> &mut f32;
    fn time_scale(&self) -> f32;
    fn time_offset(&self) -> f32;
    fn apply_world_inputs(&mut self, inputs: Self::WorldInputs);
    fn place(&mut self, transform: &Transform);
    fn apply_scalar(&mut self, property_type: PropertyType, value: f32);
}

pub fn advance_effect_time<E: TimedEffect>(ctx: &mut FrameContext) {
    let entities = E::entities(ctx.world);
    let inputs = EffectEntityInputs::collect(ctx.world, &entities);
    let time_sources = E::time_sources(ctx.world, ctx.delta_time);
    let world_inputs = E::collect_world_inputs(ctx.world);

    for &entity in &entities {
        let Some(effect) = ctx.world.get_component_mut::<E>(entity) else {
            continue;
        };
        let (time_scale, time_offset) = (effect.time_scale(), effect.time_offset());
        resolve_effect_time(effect.time_mut(), time_scale, time_offset, time_sources);
        effect.apply_world_inputs(world_inputs);
        if let Some(transform) = inputs.transform_of(entity) {
            effect.place(transform);
        }

        for (property_type, value) in inputs.sampled_scalars_of(entity, effect.time()) {
            effect.apply_scalar(property_type, value);
        }
    }
}

const BATCH_FRAME_DURATION: f32 = 1.0 / 60.0;

#[derive(Clone, Copy)]
pub struct TimelineSample {
    pub current_time: f32,
    pub playing: bool,
}

#[derive(Clone, Copy)]
pub struct EffectTimeSources {
    pub batch_fixed_time: Option<f32>,
    pub batch_frames_rendered: Option<f32>,
    pub timeline: Option<TimelineSample>,
    pub delta_time: f32,
    pub free_run_when_paused: bool,
}

impl EffectTimeSources {
    pub fn collect(
        world: &World,
        delta_time: f32,
        batch_fixed_time: Option<f32>,
        free_run_when_paused: bool,
    ) -> Self {
        let batch_frames_rendered = world
            .get_resource::<BatchRun>()
            .map(|batch| batch.frames_rendered as f32);
        let timeline = if batch_frames_rendered.is_some() {
            None
        } else {
            world
                .get_resource::<TimelineState>()
                .map(|timeline| TimelineSample {
                    current_time: timeline.current_time,
                    playing: timeline.playing,
                })
        };
        Self {
            batch_fixed_time,
            batch_frames_rendered,
            timeline,
            delta_time,
            free_run_when_paused,
        }
    }
}

pub fn resolve_effect_time(
    time: &mut f32,
    time_scale: f32,
    time_offset: f32,
    sources: EffectTimeSources,
) {
    if let Some(fixed_time) = sources.batch_fixed_time {
        *time = fixed_time;
        return;
    }
    if let Some(frames_rendered) = sources.batch_frames_rendered {
        *time = frames_rendered * BATCH_FRAME_DURATION;
        return;
    }

    let timeline_time = |timeline: TimelineSample| timeline.current_time * time_scale + time_offset;
    match sources.timeline {
        Some(timeline) if timeline.playing => *time = timeline_time(timeline),
        Some(timeline) if !sources.free_run_when_paused => *time = timeline_time(timeline),
        _ => *time += sources.delta_time.max(0.0),
    }
}

pub struct EffectEntityInputs {
    pub transforms: Vec<(Entity, Transform)>,
    pub clips: Vec<(Entity, EditableAnimationClip)>,
}

impl EffectEntityInputs {
    pub fn collect(world: &World, entities: &[Entity]) -> Self {
        let transforms = entities
            .iter()
            .filter_map(|&entity| {
                world
                    .get_component::<Transform>(entity)
                    .map(|transform| (entity, transform.clone()))
            })
            .collect();

        let clip_library = world.get_resource::<ClipLibrary>();
        let clips = entities
            .iter()
            .filter_map(|&entity| {
                let clip_id = find_entity_clip_id(world, entity)?;
                let clip = clip_library.as_ref()?.get(clip_id)?;
                Some((entity, clip.clone()))
            })
            .collect();

        Self { transforms, clips }
    }

    pub fn transform_of(&self, entity: Entity) -> Option<&Transform> {
        self.transforms
            .iter()
            .find(|(candidate, _)| *candidate == entity)
            .map(|(_, transform)| transform)
    }

    pub fn sampled_scalars_of(&self, entity: Entity, time: f32) -> Vec<(PropertyType, f32)> {
        self.clips
            .iter()
            .find(|(candidate, _)| *candidate == entity)
            .map(|(_, clip)| sampled_scalar_values(clip, time))
            .unwrap_or_default()
    }
}
