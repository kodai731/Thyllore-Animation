use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use thyllore_anim_core::editable::{
    EditableAnimationClip, PropertyCurve, PropertyType, SourceClipId,
};
use thyllore_anim_core::BoneId;

use crate::asset::AssetStorage;
use crate::ecs::component::{
    scalar_channel_domains, scalar_channel_for_cli_name, scalar_channel_for_property,
    scalar_cli_names_joined, AnimationMeta, ClipSchedule,
};
use crate::ecs::resource::{
    AnimationType, AvatarSetupState, BakedRoleClips, ClipLibrary, RecipeClipSources, TimelineState,
};
use crate::ecs::systems::phases::event_dispatch::clip_instance::ClipInstanceEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::world::{Entity, World};

use super::cli_resolve::BATCH_ANIM_EDIT_FLAG;

#[derive(Clone, Debug, PartialEq)]
pub enum BatchAnimEdit {
    DebugKeys {
        seed: u64,
    },
    Key {
        property_type: PropertyType,
        time: f32,
        value: f32,
    },
    RoleKey {
        role: thyllore_avatar_core::humanoid::components::role::HumanoidRole,
        axis: RoleAxis,
        time: f32,
        value: f32,
    },
    KeyAtPlayhead {
        property_type: PropertyType,
    },
    TrimEnd {
        seconds: f32,
    },
    Recipe {
        path: PathBuf,
    },
    NewRoleClip {
        name: String,
    },
    Template {
        path: PathBuf,
    },
    Save {
        path: PathBuf,
    },
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoleAxis {
    RotationX,
    RotationY,
    RotationZ,
    TranslationX,
    TranslationY,
    TranslationZ,
}

/// Parse repeated `--batch-anim-edit <spec>` flags. Specs:
/// `debug_keys=<seed>` | `key=<param>@<time>=<value>` | `key=<Role>.<x|y|z|tx|ty|tz>@<time>=<value>` | `clear`.
pub(super) fn anim_edits_resolve_from_args(args: &[String]) -> Result<Vec<BatchAnimEdit>> {
    let mut edits = Vec::new();
    for i in 0..args.len() {
        if args[i] != BATCH_ANIM_EDIT_FLAG {
            continue;
        }
        let Some(spec) = args.get(i + 1).filter(|v| !v.starts_with("--")) else {
            bail!("{BATCH_ANIM_EDIT_FLAG} requires a spec: debug_keys=<seed> | key=<param>@<time>=<value> | key=<Role>.<x|y|z|tx|ty|tz>@<time>=<value> | key_at_playhead=<param> | trim_end=<seconds> | recipe=<path.json> | new_role_clip=<name> | template=<path> | save=<path> | clear");
        };
        edits.push(anim_edit_parse_spec(spec)?);
    }
    Ok(edits)
}

pub(super) fn anim_edit_parse_spec(spec: &str) -> Result<BatchAnimEdit> {
    let spec = spec.trim();
    if spec == "clear" {
        return Ok(BatchAnimEdit::Clear);
    }
    if let Some(seed_str) = spec.strip_prefix("debug_keys=") {
        let seed: u64 = seed_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid debug_keys seed '{seed_str}': expected u64"))?;
        return Ok(BatchAnimEdit::DebugKeys { seed });
    }
    if let Some(param_str) = spec.strip_prefix("key_at_playhead=") {
        return Ok(BatchAnimEdit::KeyAtPlayhead {
            property_type: scalar_property_for_cli_name(param_str)?,
        });
    }
    if let Some(seconds_str) = spec.strip_prefix("trim_end=") {
        let seconds: f32 = seconds_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid trim_end seconds '{seconds_str}'"))?;
        if !seconds.is_finite() || seconds < 0.0 {
            bail!("trim_end seconds must be >= 0 and finite: '{spec}'");
        }
        return Ok(BatchAnimEdit::TrimEnd { seconds });
    }
    if let Some(rest) = spec.strip_prefix("key=") {
        let (param_str, rest) = rest.split_once('@').ok_or_else(|| {
            anyhow::anyhow!("key spec must be key=<param>@<time>=<value>, got '{spec}'")
        })?;
        let (time_str, value_str) = rest.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("key spec must be key=<param>@<time>=<value>, got '{spec}'")
        })?;
        let time: f32 = time_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid key time '{time_str}'"))?;
        let value: f32 = value_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid key value '{value_str}'"))?;
        if !time.is_finite() || time < 0.0 || !value.is_finite() {
            bail!("key time must be >= 0 and value finite: '{spec}'");
        }
        if let Some(dot) = param_str.find('.') {
            let role_name = &param_str[..dot];
            let axis_str = &param_str[dot + 1..];
            let role = parse_humanoid_role(role_name)?;
            let axis = parse_role_axis(axis_str, role)?;
            return Ok(BatchAnimEdit::RoleKey {
                role,
                axis,
                time,
                value,
            });
        }
        let property_type = scalar_property_for_cli_name(param_str)?;
        return Ok(BatchAnimEdit::Key {
            property_type,
            time,
            value,
        });
    }
    if let Some(path_str) = spec.strip_prefix("recipe=") {
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            bail!("recipe path must not be empty: '{spec}'");
        }
        return Ok(BatchAnimEdit::Recipe { path });
    }
    if let Some(name) = spec.strip_prefix("new_role_clip=") {
        let name = name.trim();
        if name.is_empty() {
            bail!("new_role_clip name must not be empty: '{spec}'");
        }
        return Ok(BatchAnimEdit::NewRoleClip {
            name: name.to_string(),
        });
    }
    if let Some(path_str) = spec.strip_prefix("template=") {
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            bail!("template path must not be empty: '{spec}'");
        }
        return Ok(BatchAnimEdit::Template { path });
    }
    if let Some(path_str) = spec.strip_prefix("save=") {
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            bail!("save path must not be empty: '{spec}'");
        }
        return Ok(BatchAnimEdit::Save { path });
    }
    bail!("unknown anim edit spec '{spec}'. Expected debug_keys=<seed> | key=<param>@<time>=<value> | key_at_playhead=<param> | trim_end=<seconds> | recipe=<path.json> | new_role_clip=<name> | template=<path> | save=<path> | clear")
}

fn scalar_property_for_cli_name(name: &str) -> Result<PropertyType> {
    let (domain, channel) = scalar_channel_for_cli_name(name.trim()).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown scalar channel '{}'. Valid channels: {}",
            name,
            scalar_cli_names_joined()
        )
    })?;
    domain
        .property_type_of(channel)
        .ok_or_else(|| anyhow::anyhow!("scalar channel '{name}' is not in its domain table"))
}

fn parse_humanoid_role(
    name: &str,
) -> Result<thyllore_avatar_core::humanoid::components::role::HumanoidRole> {
    use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
    HumanoidRole::ALL
        .iter()
        .find(|r| r.unity_name() == name)
        .copied()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "unknown role '{}'. Valid roles: {}",
                name,
                HumanoidRole::ALL
                    .iter()
                    .map(|r| r.unity_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

fn parse_role_axis(
    axis_str: &str,
    role: thyllore_avatar_core::humanoid::components::role::HumanoidRole,
) -> Result<RoleAxis> {
    use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
    match axis_str {
        "x" => Ok(RoleAxis::RotationX),
        "y" => Ok(RoleAxis::RotationY),
        "z" => Ok(RoleAxis::RotationZ),
        "tx" if role == HumanoidRole::Hips => Ok(RoleAxis::TranslationX),
        "ty" if role == HumanoidRole::Hips => Ok(RoleAxis::TranslationY),
        "tz" if role == HumanoidRole::Hips => Ok(RoleAxis::TranslationZ),
        other => Err(anyhow::anyhow!(
            "invalid axis '{}' for role '{}'. Valid axes: x, y, z (tx, ty, tz only for Hips)",
            other,
            role.unity_name()
        )),
    }
}

fn remove_clip_by_name(world: &mut World, assets: &mut AssetStorage, name: &str) {
    let source_id = match world.resource::<ClipLibrary>().find_source_by_name(name) {
        Some(id) => id,
        None => return,
    };
    world
        .resource_mut::<ClipLibrary>()
        .source_clips
        .remove(&source_id);
    if let Some(asset_id) = world
        .resource_mut::<ClipLibrary>()
        .source_to_asset_id
        .remove(&source_id)
    {
        assets.animation_clips.remove(&asset_id);
    }
    if let Some(mut baked) = world.get_resource_mut::<BakedRoleClips>() {
        let invalidated = baked.invalidate_source(source_id);
        for asset_id in invalidated {
            assets.animation_clips.remove(&asset_id);
        }
    }
}

/// Apply anim edits through the production scalar-clip event dispatcher, so batch
/// runs exercise the same path as the UI (clip creation, undo history, schedule
/// extension). Key edits temporarily move the timeline to the key's time because
/// `InsertScalarKey` always keys at `TimelineState::current_time`.
pub fn batch_apply_anim_edits(
    world: &mut World,
    assets: &mut AssetStorage,
    edits: &[BatchAnimEdit],
) {
    use crate::ecs::systems::phases::event_dispatch::scalar_curve::dispatch_scalar_clip_events;
    use crate::ecs::systems::scalar_clip_systems::{
        ensure_entity_clip, resolve_selected_scalar_entity,
    };

    for edit in edits {
        match edit {
            BatchAnimEdit::DebugKeys { seed } => {
                dispatch_scalar_clip_events(
                    &[ScalarCurveEvent::InsertScalarDebugKeys { seed: *seed }],
                    world,
                    assets,
                );
            }
            BatchAnimEdit::Key {
                property_type,
                time,
                value,
            } => {
                let previous_time = {
                    let mut timeline = world.resource_mut::<TimelineState>();
                    let previous = timeline.current_time;
                    timeline.current_time = *time;
                    previous
                };
                dispatch_scalar_clip_events(
                    &[ScalarCurveEvent::InsertScalarKey {
                        property_type: *property_type,
                        value: *value,
                    }],
                    world,
                    assets,
                );
                world.resource_mut::<TimelineState>().current_time = previous_time;
            }
            BatchAnimEdit::RoleKey {
                role,
                axis,
                time,
                value,
            } => {
                let clip_id = match world.resource::<TimelineState>().current_clip_id {
                    Some(id) => id,
                    None => continue,
                };
                let mut lib = world.resource_mut::<ClipLibrary>();
                let clip = match lib.get_mut(clip_id) {
                    Some(c) => c,
                    None => continue,
                };
                if clip.space != thyllore_anim_core::editable::ClipSpace::HumanoidRole {
                    log_warn!(
                        "key edit: current clip is not a role clip (space={:?}), skipping",
                        clip.space
                    );
                    continue;
                }
                let role_index =
                    thyllore_avatar_core::humanoid::components::role::HumanoidRole::ALL
                        .iter()
                        .position(|r| *r == *role)
                        .unwrap();
                let bone_id: BoneId = role_index as u32;
                let track = if let Some(t) = clip.get_track_mut(bone_id) {
                    t
                } else {
                    clip.add_track(bone_id, role.unity_name().to_string())
                };
                let curve = match axis {
                    RoleAxis::RotationX => &mut track.rotation_x,
                    RoleAxis::RotationY => &mut track.rotation_y,
                    RoleAxis::RotationZ => &mut track.rotation_z,
                    RoleAxis::TranslationX => &mut track.translation_x,
                    RoleAxis::TranslationY => &mut track.translation_y,
                    RoleAxis::TranslationZ => &mut track.translation_z,
                };
                use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
                curve_add_keyframe(curve, *time, *value);
                if *time > clip.duration {
                    clip.duration = *time;
                }
                lib.mark_dirty(clip_id);
            }
            BatchAnimEdit::KeyAtPlayhead { property_type } => {
                dispatch_scalar_clip_events(
                    &[ScalarCurveEvent::InsertScalarKeyAtPlayhead {
                        property_type: *property_type,
                    }],
                    world,
                    assets,
                );
            }
            BatchAnimEdit::TrimEnd { seconds } => {
                let Some((entity, domain)) = resolve_selected_scalar_entity(world) else {
                    continue;
                };
                let clip_id = ensure_entity_clip(world, assets, entity, domain);
                let Some(instance_id) = world.get_component::<ClipSchedule>(entity).and_then(|s| {
                    s.instances
                        .iter()
                        .find(|i| i.source_id == clip_id)
                        .map(|i| i.instance_id)
                }) else {
                    continue;
                };
                crate::ecs::systems::timeline_systems::process_clip_instance_events(
                    &[ClipInstanceEvent::TrimEnd {
                        entity,
                        instance_id,
                        new_clip_out: *seconds,
                    }],
                    world,
                );
            }
            BatchAnimEdit::Recipe { path } => {
                world
                    .resource_mut::<crate::ecs::resource::AssetEditQueue>()
                    .push(crate::ecs::resource::AssetEditCommand::LoadRecipeFromFile {
                        path: path.clone(),
                    });
            }
            BatchAnimEdit::NewRoleClip { name } => {
                let clip = crate::ecs::systems::role_clip_systems::new_role_clip(name);
                let id = crate::ecs::systems::clip_library_register_and_activate(
                    &mut world.resource_mut::<ClipLibrary>(),
                    assets,
                    clip,
                );
                world.resource_mut::<TimelineState>().current_clip_id = Some(id);
            }
            BatchAnimEdit::Template { path } => {
                let loaded = match crate::scene::load_animation_clip(path) {
                    Ok(c) => c,
                    Err(e) => {
                        log_warn!("template {} failed: {}", path.display(), e);
                        continue;
                    }
                };
                remove_clip_by_name(world, assets, &loaded.name);
                let id = crate::ecs::systems::clip_library_register_and_activate(
                    &mut world.resource_mut::<ClipLibrary>(),
                    assets,
                    loaded,
                );
                world.resource_mut::<TimelineState>().current_clip_id = Some(id);
            }
            BatchAnimEdit::Save { path } => {
                let current_clip_id = world.resource::<TimelineState>().current_clip_id;
                let Some(clip_id) = current_clip_id else {
                    log_warn!("save {}: no current clip", path.display());
                    continue;
                };
                let lib = world.resource::<ClipLibrary>();
                let Some(clip) = lib.get(clip_id) else {
                    log_warn!("save {}: clip id {} not found", path.display(), clip_id);
                    continue;
                };
                if let Err(e) = crate::scene::save_animation_clip(path, clip) {
                    log_warn!("save {} failed: {}", path.display(), e);
                }
            }
            BatchAnimEdit::Clear => {
                dispatch_scalar_clip_events(&[ScalarCurveEvent::ClearScalarKeys], world, assets);
            }
        }
    }
}

fn schedule_instances_json(world: &World, entity: Entity) -> Vec<serde_json::Value> {
    world
        .get_component::<ClipSchedule>(entity)
        .map(|s| {
            s.instances
                .iter()
                .map(|i| {
                    serde_json::json!({
                        "instance_id": i.instance_id,
                        "source_id": i.source_id,
                        "start_time": i.start_time,
                        "clip_in": i.clip_in,
                        "clip_out": i.clip_out,
                        "speed": i.speed,
                        "muted": i.muted,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_role_names_by_bone(world: &World, clip_id: SourceClipId) -> HashMap<BoneId, String> {
    let mut role_names: HashMap<BoneId, String> = HashMap::new();

    if let Some(setup) = world.get_resource::<AvatarSetupState>() {
        for (role, &bone_index) in &setup.mapping.by_role {
            role_names.insert(bone_index as BoneId, format!("{role:?}"));
        }
    }

    if let Some(sources) = world.get_resource::<RecipeClipSources>() {
        if let Some(source) = sources.by_clip.get(&clip_id) {
            for (bone_id, role) in &source.roles {
                role_names.insert(*bone_id, format!("{role:?}"));
            }
        }
    }

    role_names
}

fn bone_tracks_json(world: &World, clip: &EditableAnimationClip) -> Vec<serde_json::Value> {
    let role_names = collect_role_names_by_bone(world, clip.id);

    let mut tracks: Vec<_> = clip.tracks.values().collect();
    tracks.sort_unstable_by_key(|track| track.bone_id);

    tracks
        .into_iter()
        .map(|track| {
            let role = role_names.get(&track.bone_id);

            let curves = serde_json::json!({
                "rot_x": build_curve_array(&track.rotation_x),
                "rot_y": build_curve_array(&track.rotation_y),
                "rot_z": build_curve_array(&track.rotation_z),
                "pos_x": build_curve_array(&track.translation_x),
                "pos_y": build_curve_array(&track.translation_y),
                "pos_z": build_curve_array(&track.translation_z),
            });

            serde_json::json!({
                "bone_id": track.bone_id,
                "bone_name": track.bone_name,
                "role": role,
                "curves": curves,
            })
        })
        .collect()
}

fn build_curve_array(curve: &PropertyCurve) -> Vec<serde_json::Value> {
    curve
        .keyframes
        .iter()
        .map(|k| serde_json::json!({"time": k.time, "value": k.value}))
        .collect()
}

/// Serialize the animation-facing world state (effects, their scheduled clips,
/// every clip's scalar curves, timeline) so agents can inspect edits without a
/// window. Written once at engine exit; the file is the access surface.
pub fn batch_anim_dump_json(world: &World, include_tracks: bool) -> serde_json::Value {
    use crate::ecs::systems::scalar_clip_systems::find_entity_clip_id;

    let entities: Vec<serde_json::Value> = scalar_channel_domains()
        .iter()
        .flat_map(|domain| {
            (domain.entities)(world).into_iter().map(move |entity| {
                let params: serde_json::Map<String, serde_json::Value> = domain
                    .channels()
                    .iter()
                    .enumerate()
                    .filter_map(|(index, channel)| {
                        (domain.read)(world, entity, domain.property_type_at(index))
                            .map(|value| (channel.cli_name.to_string(), value.into()))
                    })
                    .collect();
                let schedule = schedule_instances_json(world, entity);
                serde_json::json!({
                    "entity": entity,
                    "domain": domain.name,
                    "time": (domain.local_time)(world, entity),
                    "clip_id": find_entity_clip_id(world, entity),
                    "params": params,
                    "schedule": schedule,
                })
            })
        })
        .collect();

    let clips: Vec<serde_json::Value> = world
        .get_resource::<ClipLibrary>()
        .map(|library| {
            let mut ids: Vec<_> = library.all_clip_ids().copied().collect();
            ids.sort_unstable();
            ids.iter()
                .filter_map(|&id| library.get(id))
                .map(|clip| {
                    let curves: Vec<serde_json::Value> = clip
                        .scalar_curves
                        .iter()
                        .map(|curve| {
                            let property = scalar_channel_for_property(curve.property_type)
                                .map(|(_, c)| c.cli_name.to_string())
                                .unwrap_or_else(|| format!("{:?}", curve.property_type));
                            let keyframes: Vec<serde_json::Value> = curve
                                .keyframes
                                .iter()
                                .map(|k| serde_json::json!({"time": k.time, "value": k.value}))
                                .collect();
                            serde_json::json!({"property": property, "keyframes": keyframes})
                        })
                        .collect();
                    let mut clip_obj = serde_json::json!({
                        "id": clip.id,
                        "name": clip.name,
                        "duration": clip.duration,
                        "bone_track_count": clip.tracks.len(),
                        "scalar_curves": curves,
                    });
                    if include_tracks {
                        let bone_tracks = bone_tracks_json(world, clip);
                        clip_obj["bone_tracks"] = serde_json::json!(bone_tracks);
                    }
                    clip_obj
                })
                .collect()
        })
        .unwrap_or_default();

    let drag_preview = world
        .get_resource::<crate::ecs::resource::TimelineInteractionState>()
        .and_then(|s| s.drag_preview)
        .map(|p| {
            serde_json::json!({
                "entity": p.entity,
                "instance_id": p.instance_id,
                "start_time": p.start_time,
                "end_time": p.end_time,
            })
        })
        .unwrap_or(serde_json::Value::Null);

    let timeline = world
        .get_resource::<TimelineState>()
        .map(|t| {
            serde_json::json!({
                "current_time": t.current_time,
                "playing": t.playing,
                "looping": t.looping,
                "current_clip_id": t.current_clip_id,
                "drag_preview": drag_preview,
            })
        })
        .unwrap_or(serde_json::Value::Null);

    let models: Vec<serde_json::Value> = world
        .component_entities::<ClipSchedule>()
        .into_iter()
        .filter(|&entity| {
            world
                .get_component::<AnimationMeta>(entity)
                .is_some_and(|meta| meta.animation_type == AnimationType::Skeletal)
        })
        .map(|entity| {
            serde_json::json!({
                "entity": entity,
                "schedule": schedule_instances_json(world, entity),
            })
        })
        .collect();

    serde_json::json!({"entities": entities, "clips": clips, "timeline": timeline, "models": models})
}

pub fn batch_anim_dump_write(world: &World, path: &str, include_tracks: bool) -> Result<()> {
    let json = batch_anim_dump_json(world, include_tracks);
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&json)?)
        .with_context(|| format!("failed to write anim dump to {path}"))?;
    Ok(())
}
