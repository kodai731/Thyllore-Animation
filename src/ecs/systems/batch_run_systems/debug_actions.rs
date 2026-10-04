use anyhow::{bail, Context, Result};

use crate::ecs::resource::{FrameClock, ScheduledBatchAction, ScheduledBatchActions};
use crate::ecs::world::World;

use super::batch_action::{batch_action_registry, BatchAction};
use super::cli_resolve::{BATCH_DEBUG_ACTION_AT_FLAG, BATCH_DEBUG_ACTION_FLAG};

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

/// Parse repeated `--batch-debug-action-at <frame>:<action>` flags; the action is validated now and
/// applied once the frame clock reaches `<frame>`.
pub(super) fn scheduled_actions_resolve_from_args(
    args: &[String],
) -> Result<Vec<ScheduledBatchAction>> {
    let mut scheduled = Vec::new();
    for i in 0..args.len() {
        if args[i] != BATCH_DEBUG_ACTION_AT_FLAG {
            continue;
        }
        let Some(spec) = args.get(i + 1).filter(|v| !v.starts_with("--")) else {
            bail!("{BATCH_DEBUG_ACTION_AT_FLAG} requires <frame>:<action>");
        };
        scheduled.push(scheduled_action_parse(spec)?);
    }
    Ok(scheduled)
}

pub(super) fn scheduled_action_parse(spec: &str) -> Result<ScheduledBatchAction> {
    let (frame_text, action_text) = spec.split_once(':').with_context(|| {
        format!("{BATCH_DEBUG_ACTION_AT_FLAG} expects <frame>:<action>, got '{spec}'")
    })?;
    let frame: u64 = frame_text
        .trim()
        .parse()
        .with_context(|| format!("invalid frame '{frame_text}' in '{spec}'"))?;
    debug_action_parse(action_text)?;

    Ok(ScheduledBatchAction {
        frame,
        action: action_text.trim().to_string(),
    })
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
/// UI commands so they run through the normal dispatch on the first frame.
pub fn batch_apply_debug_actions(world: &mut World, actions: &[&dyn BatchAction]) {
    for action in actions {
        action.apply(world);
    }
}

/// Applies every scheduled action whose frame has come, in the order the CLI gave them.
pub fn batch_apply_scheduled_actions(world: &mut World) {
    let frame = world.resource::<FrameClock>().frame;
    let due_actions: Vec<String> = {
        let Some(mut scheduled) = world.get_resource_mut::<ScheduledBatchActions>() else {
            return;
        };
        let (due, pending): (Vec<_>, Vec<_>) = scheduled
            .pending
            .drain(..)
            .partition(|entry| entry.frame <= frame);
        scheduled.pending = pending;
        due.into_iter().map(|entry| entry.action).collect()
    };

    for action_text in due_actions {
        match debug_action_parse(&action_text) {
            Ok(action) => action.apply(world),
            Err(error) => log_error!("Scheduled batch action failed to parse: {:?}", error),
        }
    }
}

pub fn debug_actions_json() -> String {
    let names: Vec<&str> = batch_action_registry()
        .iter()
        .map(|descriptor| descriptor.name)
        .collect();
    serde_json::json!({"ok": true, "actions": names}).to_string()
}
