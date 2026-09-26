use anyhow::{bail, Result};
use thyllore_effect_core::TextureFitGroups;

use crate::ecs::component::{ClipSchedule, FlameBaked, FlameEffect};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{ClipDragPreview, ClipDragType, TimelineInteractionState};
use crate::ecs::systems::scalar_clip_systems::find_entity_clip_id;
use crate::ecs::systems::timeline_systems::clip_drag_preview_times;
use crate::ecs::systems::{unit_action_parse, BatchAction};
use crate::ecs::world::{Entity, World};

use super::apply_texture_fit_from_path;

#[derive(Debug, Default)]
pub struct AddFlame;

#[derive(Debug, Default)]
pub struct OpenFlameCurves;

#[derive(Debug, Default)]
pub struct TimelineSelectFlameClip;

#[derive(Debug)]
pub struct FlameClipPreview {
    pub end_seconds: f32,
}

#[derive(Debug)]
pub struct ApplyTextureFit {
    pub path: String,
    pub blend: f32,
    pub profile: bool,
}

#[derive(Debug)]
pub struct ApplyTextureFitRoundtrip {
    pub path: String,
    pub blend: f32,
    pub profile: bool,
}

impl BatchAction for AddFlame {
    fn name(&self) -> &'static str {
        "add_flame"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::AddEffect(super::FLAME_SPAWN_HOOK.key));
    }
}

impl BatchAction for OpenFlameCurves {
    fn name(&self) -> &'static str {
        "open_flame_curves"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::OpenScalarCurveEditor);
    }
}

impl BatchAction for TimelineSelectFlameClip {
    fn name(&self) -> &'static str {
        "timeline_select_flame_clip"
    }
    fn apply(&self, world: &mut World) {
        let clip_id = world
            .entities_with::<FlameEffect>()
            .first()
            .and_then(|&flame| find_entity_clip_id(world, flame));
        if let Some(clip_id) = clip_id {
            world
                .resource_mut::<UIEventQueue>()
                .send(UIEvent::TimelineSelectClip(clip_id));
        }
    }
}

impl BatchAction for FlameClipPreview {
    fn name(&self) -> &'static str {
        "flame_clip_preview"
    }
    fn apply(&self, world: &mut World) {
        apply_flame_clip_preview(world, self.end_seconds);
    }
}

impl BatchAction for ApplyTextureFit {
    fn name(&self) -> &'static str {
        "apply_texture_fit"
    }
    fn apply(&self, world: &mut World) {
        apply_texture_fit_to_first_flame(world, &self.path, self.blend, self.profile);
    }
}

impl BatchAction for ApplyTextureFitRoundtrip {
    fn name(&self) -> &'static str {
        "apply_texture_fit_roundtrip"
    }
    fn apply(&self, world: &mut World) {
        let Some((flame, original_effect, original_baked)) = first_flame_effect_and_baked(world)
        else {
            return;
        };
        apply_texture_fit_to_first_flame(world, &self.path, self.blend, self.profile);
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::UpdateFlameEffect {
                entity: flame,
                effect: Box::new(original_effect),
            });
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::UpdateFlameBaked(Box::new(original_baked)));
    }
}

/// Same preview math as a live TrimEnd drag, but no commit event: the instance stays untouched.
fn apply_flame_clip_preview(world: &World, end_seconds: f32) {
    let Some(&flame) = world.entities_with::<FlameEffect>().first() else {
        return;
    };
    let Some(instance) = world
        .get_component::<ClipSchedule>(flame)
        .and_then(|schedule| schedule.first_instance().cloned())
    else {
        return;
    };

    let (start_time, end_time) = clip_drag_preview_times(
        &ClipDragType::TrimEnd,
        instance.clip_out,
        end_seconds - instance.clip_out,
        instance.start_time,
        instance.end_time(),
        instance.clip_in,
        instance.clip_out,
    );
    world
        .resource_mut::<TimelineInteractionState>()
        .drag_preview = Some(ClipDragPreview {
        entity: flame,
        instance_id: instance.instance_id,
        start_time,
        end_time,
    });
}

fn first_flame_effect_and_baked(world: &World) -> Option<(Entity, FlameEffect, FlameBaked)> {
    let &flame = world.entities_with::<FlameEffect>().first()?;
    let effect = world.get_component::<FlameEffect>(flame)?.clone();
    let baked = world
        .get_component::<FlameBaked>(flame)
        .cloned()
        .unwrap_or_default();
    Some((flame, effect, baked))
}

fn apply_texture_fit_to_first_flame(world: &World, path: &str, blend: f32, profile: bool) {
    let Some((flame, mut effect, mut baked)) = first_flame_effect_and_baked(world) else {
        return;
    };
    apply_texture_fit_from_path(
        &mut effect,
        &mut baked,
        path,
        blend,
        TextureFitGroups::default(),
        profile,
        "debug_action",
    );
    world
        .resource_mut::<UIEventQueue>()
        .send(UIEvent::UpdateFlameEffect {
            entity: flame,
            effect: Box::new(effect),
        });
    world
        .resource_mut::<UIEventQueue>()
        .send(UIEvent::UpdateFlameBaked(Box::new(baked)));
}

fn clip_preview_seconds_parse(text: &str) -> Result<f32> {
    let end_seconds: f32 = text
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid flame_clip_preview seconds '{text}'"))?;
    if !end_seconds.is_finite() || end_seconds < 0.0 {
        bail!("flame_clip_preview seconds must be >= 0 and finite: '{text}'");
    }
    Ok(end_seconds)
}

fn flame_clip_preview_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let seconds_text = text.strip_prefix("flame_clip_preview=")?.trim();
    Some(
        clip_preview_seconds_parse(seconds_text)
            .map(|end_seconds| Box::new(FlameClipPreview { end_seconds }) as Box<dyn BatchAction>),
    )
}

fn texture_fit_action_args_parse(rest: &str) -> Result<(String, f32, bool)> {
    let parts: Vec<&str> = rest.rsplit(',').collect();
    if parts.len() != 3 {
        bail!(
            "apply_texture_fit expects <path>,<blend>,<profile|statistics>, got '{}'",
            rest
        );
    }
    let profile_text = parts[0];
    let blend_text = parts[1];
    let path = parts[2].to_string();

    let blend: f32 = blend_text
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid blend value '{}'", blend_text))?;
    if !blend.is_finite() || !(0.0..=1.0).contains(&blend) {
        bail!("blend must be in [0.0, 1.0], got {}", blend);
    }

    let profile = match profile_text {
        "profile" => true,
        "statistics" => false,
        _ => bail!(
            "profile mode must be 'profile' or 'statistics', got '{}'",
            profile_text
        ),
    };
    Ok((path, blend, profile))
}

fn apply_texture_fit_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let rest = text.strip_prefix("apply_texture_fit:")?;
    Some(
        texture_fit_action_args_parse(rest).map(|(path, blend, profile)| {
            Box::new(ApplyTextureFit {
                path,
                blend,
                profile,
            }) as Box<dyn BatchAction>
        }),
    )
}

fn apply_texture_fit_roundtrip_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let rest = text.strip_prefix("apply_texture_fit_roundtrip:")?;
    Some(
        texture_fit_action_args_parse(rest).map(|(path, blend, profile)| {
            Box::new(ApplyTextureFitRoundtrip {
                path,
                blend,
                profile,
            }) as Box<dyn BatchAction>
        }),
    )
}

crate::batch_action!("add_flame", unit_action_parse::<AddFlame>);
crate::batch_action!("open_flame_curves", unit_action_parse::<OpenFlameCurves>);
crate::batch_action!(
    "timeline_select_flame_clip",
    unit_action_parse::<TimelineSelectFlameClip>
);
crate::batch_action!("flame_clip_preview", flame_clip_preview_parse);
crate::batch_action!("apply_texture_fit", apply_texture_fit_parse);
crate::batch_action!(
    "apply_texture_fit_roundtrip",
    apply_texture_fit_roundtrip_parse
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetStorage;
    use crate::ecs::resource::{ClipLibrary, TimelineState};
    use crate::ecs::systems::{
        batch_anim_dump_json, batch_apply_debug_actions, resolve_engine_cli_overrides,
    };

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flame_clip_preview_parses_and_rejects_invalid() {
        let actions = resolve_engine_cli_overrides(&args(&[
            "bin",
            "--batch-debug-action",
            "flame_clip_preview=3.5",
        ]))
        .unwrap()
        .debug_actions;
        assert_eq!(actions[0].name(), "flame_clip_preview");
        assert_eq!(
            format!("{:?}", actions[0]),
            "FlameClipPreview { end_seconds: 3.5 }"
        );
        for bad in ["flame_clip_preview=abc", "flame_clip_preview=-1"] {
            assert!(
                resolve_engine_cli_overrides(&args(&["bin", "--batch-debug-action", bad])).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn texture_fit_action_args_take_path_blend_and_profile() {
        let (path, blend, profile) = texture_fit_action_args_parse("a/b.png,0.5,profile").unwrap();
        assert_eq!(path, "a/b.png");
        assert!((blend - 0.5).abs() < 1e-6);
        assert!(profile);
        assert!(texture_fit_action_args_parse("a.png,1.5,statistics").is_err());
        assert!(texture_fit_action_args_parse("a.png,0.5,bogus").is_err());
        assert!(texture_fit_action_args_parse("a.png,0.5").is_err());
    }

    #[test]
    fn flame_clip_preview_sets_drag_preview_without_touching_instance() {
        let mut world = World::new();
        world.insert_resource(ClipLibrary::new());
        world.insert_resource(TimelineState::new());
        world.insert_resource(TimelineInteractionState::default());
        let mut assets = AssetStorage::new();
        let flame = super::super::spawn_flame_with_clip(
            &mut world,
            &mut assets,
            "Flame",
            FlameEffect::default(),
        );

        batch_apply_debug_actions(
            &mut world,
            &[&FlameClipPreview { end_seconds: 3.0 } as &dyn BatchAction],
        );

        let preview = world
            .resource::<TimelineInteractionState>()
            .drag_preview
            .expect("preview set");
        assert_eq!(preview.entity, flame);
        assert!((preview.start_time - 0.0).abs() < 1e-6);
        assert!((preview.end_time - 3.0).abs() < 1e-6);

        let instance = world
            .get_component::<ClipSchedule>(flame)
            .unwrap()
            .first_instance()
            .cloned()
            .unwrap();
        assert!(
            (instance.clip_out - 0.0).abs() < 1e-6,
            "preview must not commit the trim"
        );

        let dump = batch_anim_dump_json(&world);
        assert!(
            (dump["timeline"]["drag_preview"]["end_time"]
                .as_f64()
                .unwrap()
                - 3.0)
                .abs()
                < 1e-6
        );
    }
}
