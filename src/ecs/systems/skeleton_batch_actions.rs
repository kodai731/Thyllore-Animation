use anyhow::{anyhow, Result};
use cgmath::Vector3;

use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{
    AssetEditCommand, AssetEditQueue, HierarchyDisplayMode, HierarchyState,
};
use crate::ecs::systems::phases::event_dispatch::edit_history::EditHistoryEvent;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::batch_run_systems::{unit_action_parse, BatchAction};

#[derive(Debug, Default)]
struct Undo;

impl BatchAction for Undo {
    fn name(&self) -> &'static str {
        "undo"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(EditHistoryEvent::Undo);
    }
}

#[derive(Debug, Default)]
struct Redo;

impl BatchAction for Redo {
    fn name(&self) -> &'static str {
        "redo"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(EditHistoryEvent::Redo);
    }
}

#[derive(Debug)]
struct QueuedSkeletonEdit {
    name: &'static str,
    command: AssetEditCommand,
}

impl BatchAction for QueuedSkeletonEdit {
    fn name(&self) -> &'static str {
        self.name
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<AssetEditQueue>()
            .push(self.command.clone());
    }
}

fn parse_offset(parts: &[&str], usage: &str) -> Result<Vector3<f32>> {
    let [x, y, z] = parts else {
        return Err(anyhow!("{usage}"));
    };
    let parse = |text: &str| -> Result<f32> {
        text.trim()
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| anyhow!("{usage}"))
    };
    Ok(Vector3::new(parse(x)?, parse(y)?, parse(z)?))
}

fn non_empty(text: &str, usage: &str) -> Result<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("{usage}"));
    }
    Ok(trimmed.to_string())
}

const ADD_BONE_USAGE: &str = "add_bone expects add_bone=<parent>,<name>,<x>,<y>,<z>";

fn add_bone_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("add_bone=")?;
    let parts: Vec<&str> = spec.split(',').collect();
    if parts.len() != 5 {
        return Some(Err(anyhow!("{ADD_BONE_USAGE}")));
    }
    let command = (|| -> Result<AssetEditCommand> {
        Ok(AssetEditCommand::AddBone {
            parent: non_empty(parts[0], ADD_BONE_USAGE)?,
            name: non_empty(parts[1], ADD_BONE_USAGE)?,
            local_offset: parse_offset(&parts[2..], ADD_BONE_USAGE)?,
        })
    })();
    Some(command.map(|command| {
        Box::new(QueuedSkeletonEdit {
            name: "add_bone",
            command,
        }) as Box<dyn BatchAction>
    }))
}

const REMOVE_BONE_USAGE: &str = "remove_bone expects remove_bone=<name>";

fn remove_bone_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("remove_bone=")?;
    Some(non_empty(spec, REMOVE_BONE_USAGE).map(|bone| {
        Box::new(QueuedSkeletonEdit {
            name: "remove_bone",
            command: AssetEditCommand::RemoveBone { bone },
        }) as Box<dyn BatchAction>
    }))
}

const SET_BONE_OFFSET_USAGE: &str = "set_bone_offset expects set_bone_offset=<name>,<x>,<y>,<z>";

fn set_bone_offset_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("set_bone_offset=")?;
    let parts: Vec<&str> = spec.split(',').collect();
    if parts.len() != 4 {
        return Some(Err(anyhow!("{SET_BONE_OFFSET_USAGE}")));
    }
    let command = (|| -> Result<AssetEditCommand> {
        Ok(AssetEditCommand::SetBoneOffset {
            bone: non_empty(parts[0], SET_BONE_OFFSET_USAGE)?,
            local_offset: parse_offset(&parts[1..], SET_BONE_OFFSET_USAGE)?,
        })
    })();
    Some(command.map(|command| {
        Box::new(QueuedSkeletonEdit {
            name: "set_bone_offset",
            command,
        }) as Box<dyn BatchAction>
    }))
}

#[derive(Debug)]
struct SelectBoneByName(String);

impl UiCommand for SelectBoneByName {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        let bone_id = assets
            .skeletons
            .values()
            .next()
            .and_then(|asset| asset.skeleton.bone_name_to_id.get(&self.0).copied());
        let Some(bone_id) = bone_id else {
            msg_warn!("select_bone: no bone named '{}'", self.0);
            return;
        };
        world.resource_mut::<HierarchyState>().display_mode = HierarchyDisplayMode::Bones;
        Box::new(HierarchyEvent::SelectBone(bone_id)).apply(world, assets, graphics);
    }
}

#[derive(Debug)]
struct SelectBone(String);

impl BatchAction for SelectBone {
    fn name(&self) -> &'static str {
        "select_bone"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(SelectBoneByName(self.0.clone()));
    }
}

const SELECT_BONE_USAGE: &str = "select_bone expects select_bone=<name>";

fn select_bone_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("select_bone=")?;
    Some(
        non_empty(spec, SELECT_BONE_USAGE)
            .map(|bone| Box::new(SelectBone(bone)) as Box<dyn BatchAction>),
    )
}

crate::batch_action!("add_bone", add_bone_parse);
crate::batch_action!("select_bone", select_bone_parse);
crate::batch_action!("remove_bone", remove_bone_parse);
crate::batch_action!("set_bone_offset", set_bone_offset_parse);
crate::batch_action!("undo", unit_action_parse::<Undo>);
crate::batch_action!("redo", unit_action_parse::<Redo>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_bone_parses_parent_name_and_offset() {
        let action = add_bone_parse("add_bone=Head,hair_root,0,0.1,0")
            .expect("prefix matches")
            .expect("spec is valid");
        assert_eq!(action.name(), "add_bone");
    }

    #[test]
    fn malformed_specs_are_rejected() {
        for spec in [
            "add_bone=Head,hair_root,0,0.1",
            "add_bone=,hair_root,0,0.1,0",
            "add_bone=Head,hair_root,0,nan,0",
            "remove_bone= ",
            "set_bone_offset=Head,1,2",
        ] {
            let parsed = add_bone_parse(spec)
                .or_else(|| remove_bone_parse(spec))
                .or_else(|| set_bone_offset_parse(spec))
                .expect("prefix matches");
            assert!(parsed.is_err(), "{spec}");
        }
    }

    #[test]
    fn remove_and_offset_parse() {
        assert_eq!(
            remove_bone_parse("remove_bone=hair_00_01")
                .unwrap()
                .unwrap()
                .name(),
            "remove_bone"
        );
        assert_eq!(
            set_bone_offset_parse("set_bone_offset=hair_00_01,0,-5,0")
                .unwrap()
                .unwrap()
                .name(),
            "set_bone_offset"
        );
    }
}
