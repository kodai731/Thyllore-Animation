use anyhow::{bail, Result};

use crate::ecs::world::World;

use super::batch_action::{batch_action_registry, BatchAction};
use super::cli_resolve::BATCH_DEBUG_ACTION_FLAG;

pub(super) fn debug_actions_resolve_from_args(
    args: &[String],
) -> Result<Vec<Box<dyn BatchAction>>> {
    let mut actions = Vec::new();
    for i in 0..args.len() {
        if args[i] != BATCH_DEBUG_ACTION_FLAG {
            continue;
        }
        let Some(name) = args.get(i + 1).filter(|v| !v.starts_with("--")) else {
            bail!(
                "{BATCH_DEBUG_ACTION_FLAG} requires an action. Valid actions: {}",
                registered_action_names()
            );
        };
        actions.push(debug_action_parse(name)?);
    }
    Ok(actions)
}

pub(super) fn debug_action_parse(name: &str) -> Result<Box<dyn BatchAction>> {
    let name = name.trim();
    for descriptor in batch_action_registry() {
        if let Some(result) = (descriptor.parse)(name) {
            return result;
        }
    }
    bail!(
        "unknown debug action '{name}'. Valid actions: {}",
        registered_action_names()
    )
}

fn registered_action_names() -> String {
    batch_action_registry()
        .iter()
        .map(|descriptor| descriptor.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Execute debug-window actions headlessly: view-mode radios write the same
/// `DebugViewState` resource the imgui panel edits, buttons enqueue the same
/// `UIEvent`s so they run through the normal dispatch on the first frame.
pub fn batch_apply_debug_actions(world: &World, actions: &[&dyn BatchAction]) {
    for action in actions {
        action.apply(world);
    }
}

pub fn debug_actions_json() -> String {
    let names: Vec<&str> = batch_action_registry()
        .iter()
        .map(|descriptor| descriptor.name)
        .collect();
    serde_json::json!({"ok": true, "actions": names}).to_string()
}
