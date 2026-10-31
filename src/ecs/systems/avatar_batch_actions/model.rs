use crate::ecs::resource::{
    AssetEditCommand, AssetEditQueue, OutputCommand, OutputQueue, ScaleHistory,
};
use crate::ecs::systems::avatar_export_systems::dump_morph_track_samples;
use crate::ecs::systems::batch_run_systems::{unit_action_parse, BatchAction};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::systems::validate_scale_factor;
use crate::ecs::world::World;
use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ApplyModelScale {
    factor: f32,
}

impl BatchAction for ApplyModelScale {
    fn name(&self) -> &'static str {
        "apply_model_scale"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<AssetEditQueue>()
            .push(AssetEditCommand::ApplyModelScale {
                factor: self.factor,
                history: ScaleHistory::Record,
            });
    }
}

pub(super) fn apply_model_scale_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("apply_model_scale=")?.trim();
    Some(
        spec.parse::<f32>()
            .map_err(|_| anyhow!("apply_model_scale expects apply_model_scale=<factor>"))
            .and_then(|factor| {
                validate_scale_factor(factor)?;
                Ok(Box::new(ApplyModelScale { factor }) as Box<dyn BatchAction>)
            }),
    )
}

#[derive(Debug, Default)]
pub struct ExportUnityAvatar;

impl BatchAction for ExportUnityAvatar {
    fn name(&self) -> &'static str {
        "export_unity_avatar"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::ExportUnityAvatar);
    }
}

#[derive(Debug, Default)]
pub struct DumpMorphTrackSamples;

impl BatchAction for DumpMorphTrackSamples {
    fn name(&self) -> &'static str {
        "dump_morph_track_samples"
    }
    fn apply(&self, world: &mut World) {
        dump_morph_track_samples(world);
    }
}

#[derive(Debug)]
pub struct ExportModelFbx {
    path: PathBuf,
}

impl BatchAction for ExportModelFbx {
    fn name(&self) -> &'static str {
        "export_model_fbx"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<OutputQueue>()
            .push(OutputCommand::ExportModelFbx {
                path: self.path.clone(),
            });
    }
}

pub(super) fn export_model_fbx_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("export_model_fbx=")?.trim();
    if spec.is_empty() {
        return Some(Err(anyhow!(
            "export_model_fbx expects export_model_fbx=<path.fbx>"
        )));
    }
    Some(Ok(Box::new(ExportModelFbx {
        path: PathBuf::from(spec),
    })))
}
crate::batch_action!("apply_model_scale", apply_model_scale_parse);
crate::batch_action!(
    "export_unity_avatar",
    unit_action_parse::<ExportUnityAvatar>
);
crate::batch_action!(
    "dump_morph_track_samples",
    unit_action_parse::<DumpMorphTrackSamples>
);
crate::batch_action!("export_model_fbx", export_model_fbx_parse);
