use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::animation::editable::PropertyType;
use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{CurveEditorState, CurveEditorTarget, TimelineState};
use crate::ecs::systems::animation_debug_dump::{
    build_animation_debug_dump, resolve_animation_debug_target,
};
use crate::ecs::systems::batch_run_systems::batch_anim_dump_json;
use crate::ecs::World;
use crate::hooks::external_command::ExternalCommandSender;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub const LIVE_DUMP_DIRECTORY: &str = "log/live_dump";
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

crate::external_command_source!("live_dump", start_live_dump_listener);

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiveDumpRequest {
    Anim {
        #[serde(default)]
        include_tracks: bool,
    },
    Pose {
        times: Vec<f32>,
    },
}

/// Applied on the main thread in the dispatch phase; the reply goes back to the listener thread.
#[derive(Debug)]
pub struct LiveDumpCommand {
    pub request: LiveDumpRequest,
    pub reply: mpsc::Sender<serde_json::Value>,
}

impl UiCommand for LiveDumpCommand {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        let response = match serve_request(world, assets, &self.request) {
            Ok(data) => serde_json::json!({ "ok": true, "data": data }),
            Err(error) => serde_json::json!({ "ok": false, "error": format!("{error:#}") }),
        };
        if self.reply.send(response).is_err() {
            log_warn!("live dump: client went away before the reply");
        }
    }
}

pub fn live_dump_socket_path() -> PathBuf {
    Path::new(LIVE_DUMP_DIRECTORY).join("engine.sock")
}

#[cfg(unix)]
fn start_live_dump_listener(sender: ExternalCommandSender) -> Result<()> {
    use std::os::unix::net::UnixListener;

    let socket_path = live_dump_socket_path();
    std::fs::create_dir_all(LIVE_DUMP_DIRECTORY)
        .with_context(|| format!("failed to create {LIVE_DUMP_DIRECTORY}"))?;
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)
            .with_context(|| format!("failed to remove stale {}", socket_path.display()))?;
    }
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind {}", socket_path.display()))?;

    std::thread::Builder::new()
        .name("live_dump".to_string())
        .spawn(move || {
            for connection in listener.incoming() {
                match connection {
                    Ok(stream) => serve_connection(stream, &sender),
                    Err(error) => log_warn!("live dump: accept failed: {error}"),
                }
            }
        })
        .context("failed to spawn the live dump thread")?;
    log!("live dump listening on {}", socket_path.display());
    Ok(())
}

#[cfg(not(unix))]
fn start_live_dump_listener(_: ExternalCommandSender) -> Result<()> {
    anyhow::bail!("live dump needs a Unix socket; not available on this platform")
}

fn serve_connection<S: std::io::Read + Write>(mut stream: S, sender: &ExternalCommandSender) {
    let response = match handle_request_line(&mut stream, sender) {
        Ok(response) => response,
        Err(error) => serde_json::json!({ "ok": false, "error": format!("{error:#}") }),
    };
    if let Err(error) = stream.write_all(response.to_string().as_bytes()) {
        log_warn!("live dump: cannot write the reply: {error}");
    }
}

fn handle_request_line<S: std::io::Read + Write>(
    stream: &mut S,
    sender: &ExternalCommandSender,
) -> Result<serde_json::Value> {
    let mut line = String::new();
    BufReader::new(&mut *stream)
        .read_line(&mut line)
        .context("failed to read the request line")?;
    let request: LiveDumpRequest =
        serde_json::from_str(line.trim()).context("invalid live dump request")?;

    let (reply, reply_rx) = mpsc::channel();
    sender.send(Box::new(LiveDumpCommand { request, reply }))?;
    reply_rx
        .recv_timeout(REPLY_TIMEOUT)
        .context("the engine did not answer in time")
}

fn serve_request(
    world: &World,
    assets: &AssetStorage,
    request: &LiveDumpRequest,
) -> Result<serde_json::Value> {
    match request {
        LiveDumpRequest::Anim { include_tracks } => {
            let mut data = batch_anim_dump_json(world, *include_tracks);
            data["curve_editor"] = curve_editor_json(world);
            Ok(data)
        }
        LiveDumpRequest::Pose { times } => {
            let target = resolve_animation_debug_target(world, assets)
                .context("animation debug target not found (no clip/skeleton/rig)")?;
            let looping = world.resource::<TimelineState>().looping;
            let dump = build_animation_debug_dump(&target, times, looping);
            Ok(serde_json::to_value(dump)?)
        }
    }
}

pub fn curve_editor_json(world: &World) -> serde_json::Value {
    let Some(editor) = world.get_resource::<CurveEditorState>() else {
        return serde_json::Value::Null;
    };

    let target = match editor.selected_target {
        Some(CurveEditorTarget::Bone(bone_id)) => {
            serde_json::json!({ "kind": "bone", "bone_id": bone_id })
        }
        Some(CurveEditorTarget::Scalars) => serde_json::json!({ "kind": "scalars" }),
        Some(CurveEditorTarget::Morph(index)) => {
            serde_json::json!({ "kind": "morph", "index": index })
        }
        None => serde_json::Value::Null,
    };

    let mut visible: Vec<PropertyType> = editor.visible_curves.iter().copied().collect();
    visible.sort_by_key(|property| format!("{property:?}"));

    let selected: Vec<serde_json::Value> = editor
        .selected_keyframes
        .iter()
        .map(|keyframe| {
            serde_json::json!({
                "property": keyframe.property_type,
                "keyframe_id": keyframe.keyframe_id,
                "original_time": keyframe.original_time,
                "original_value": keyframe.original_value,
            })
        })
        .collect();

    serde_json::json!({
        "is_open": editor.is_open,
        "target": target,
        "visible_curves": visible,
        "selected_keyframes": selected,
        "view": {
            "time_offset": editor.view_time_offset,
            "duration": editor.view_duration,
            "value_offset": editor.view_value_offset,
            "value_range": editor.view_val_range,
            "zoom_x": editor.zoom_x,
            "zoom_y": editor.zoom_y,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_defaults_to_anim_without_tracks() {
        let request: LiveDumpRequest = serde_json::from_str(r#"{"kind": "anim"}"#).unwrap();
        assert_eq!(
            request,
            LiveDumpRequest::Anim {
                include_tracks: false
            }
        );
    }

    #[test]
    fn request_parses_pose_times() {
        let request: LiveDumpRequest =
            serde_json::from_str(r#"{"kind": "pose", "times": [0.0, 1.5]}"#).unwrap();
        assert_eq!(
            request,
            LiveDumpRequest::Pose {
                times: vec![0.0, 1.5]
            }
        );
    }

    #[test]
    fn curve_editor_json_reports_the_selected_bone_and_visible_curves() {
        let mut world = World::new();
        let mut editor = CurveEditorState::default();
        editor.select_bone(7);
        editor.visible_curves.clear();
        editor.visible_curves.insert(PropertyType::RotationY);
        world.insert_resource(editor);

        let json = curve_editor_json(&world);

        assert_eq!(json["target"]["bone_id"], 7);
        assert_eq!(json["visible_curves"], serde_json::json!(["RotationY"]));
    }

    #[test]
    fn command_replies_through_its_channel() {
        let mut world = World::new();
        world.insert_resource(TimelineState::default());
        world.insert_resource(CurveEditorState::default());
        let mut assets = AssetStorage::default();
        let (reply, reply_rx) = mpsc::channel();

        Box::new(LiveDumpCommand {
            request: LiveDumpRequest::Anim {
                include_tracks: false,
            },
            reply,
        })
        .apply(&mut world, &mut assets, &GraphicsResources::default());

        let response = reply_rx.recv().unwrap();
        assert_eq!(response["ok"], true);
        assert!(response["data"]["curve_editor"]["is_open"].is_boolean());
    }
}
