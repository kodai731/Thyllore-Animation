use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use super::bone_label::{format_bone_label, order_bone_ids_by_role};
use super::window::{get_current_clip, ALL_PROPERTY_TYPES};
use crate::animation::editable::{EditableAnimationClip, PropertyType};
use crate::animation::BoneId;
use crate::ecs::component::{scalar_channel_for_property, ScalarChannelDomain};
use crate::ecs::resource::{
    ClipLibrary, CurveEditorState, CurveEditorTarget, HumanoidRigState, TimelineState,
};
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::world::World;

pub(super) fn collect_humanoid_bone_roles(world: &World) -> Vec<(BoneId, HumanoidRole)> {
    world
        .get_resource::<HumanoidRigState>()
        .and_then(|state| {
            state.rig.as_ref().map(|rig| {
                rig.mapping
                    .by_role
                    .iter()
                    .map(|(role, &bone_index)| (bone_index as BoneId, *role))
                    .collect()
            })
        })
        .unwrap_or_default()
}
pub(super) fn find_bone_role(
    bone_roles: &[(BoneId, HumanoidRole)],
    bone_id: BoneId,
) -> Option<HumanoidRole> {
    bone_roles
        .iter()
        .find(|(role_bone_id, _)| *role_bone_id == bone_id)
        .map(|(_, role)| *role)
}
pub(super) fn collect_listed_bone_ids(
    clip: &EditableAnimationClip,
    bone_roles: &[(BoneId, HumanoidRole)],
) -> Vec<BoneId> {
    let mut bone_ids: Vec<BoneId> = clip.tracks.keys().copied().collect();
    let untracked_role_bones = bone_roles
        .iter()
        .map(|(bone_id, _)| *bone_id)
        .filter(|bone_id| !clip.tracks.contains_key(bone_id));
    bone_ids.extend(untracked_role_bones);
    order_bone_ids_by_role(&bone_ids, bone_roles)
}
pub(super) fn build_track_list(
    ui: &imgui::Ui,
    world: &World,
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
    editor_state: &mut CurveEditorState,
    scalar_domain: Option<&'static ScalarChannelDomain>,
    bone_roles: &[(BoneId, HumanoidRole)],
) {
    let Some(clip) = get_current_clip(timeline_state, clip_library) else {
        ui.text("No clip selected");
        return;
    };

    // A scalar-domain clip animates one component's channels: showing the
    // (unrelated) bone list next to it only invites confusion, so the track
    // list is either the domain's channels or the bone tracks, never both.
    if scalar_domain.is_some() || !clip.scalar_curves.is_empty() {
        if editor_state.selected_target != Some(CurveEditorTarget::Scalars) {
            editor_state.select_scalars();
            for curve in &clip.scalar_curves {
                editor_state.visible_curves.insert(curve.property_type);
            }
            editor_state.view_initialized = false;
        }
        let label = scalar_domain.map(|d| d.name).unwrap_or("Scalars");
        ui.text(format!("{label}:"));
        ui.separator();
        if let Some(domain) = scalar_domain {
            build_scalar_curve_selector_inline(ui, world, clip, editor_state, domain);
        }
        return;
    }

    ui.text("Bones:");
    ui.separator();

    let rig_state = world.get_resource::<HumanoidRigState>();
    let rig_track_names = rig_state
        .as_ref()
        .and_then(|state| state.rig.as_ref())
        .map(|rig| &rig.track_names);

    for bone_id in collect_listed_bone_ids(clip, bone_roles) {
        let bone_name = match clip.tracks.get(&bone_id) {
            Some(track) => &track.bone_name,
            None => match rig_track_names.and_then(|names| names.get(&bone_id)) {
                Some(name) => name,
                None => continue,
            },
        };
        let is_selected = editor_state.selected_bone_id() == Some(bone_id);
        let is_spring_bone = timeline_state.baked_bone_ids.contains(&bone_id);
        let role = find_bone_role(bone_roles, bone_id);
        let label = if role.is_some() {
            let role_label = format_bone_label(bone_name, role);
            if is_spring_bone {
                format!("[SB] {}", role_label)
            } else {
                role_label
            }
        } else if is_spring_bone {
            let name = if bone_name.len() > 13 {
                &bone_name[..10]
            } else {
                bone_name
            };
            format!("[SB] {}", name)
        } else if bone_name.len() > 18 {
            format!("{}...", &bone_name[..15])
        } else {
            bone_name.clone()
        };

        if ui.selectable_config(&label).selected(is_selected).build() {
            editor_state.select_bone(bone_id);
            editor_state.view_initialized = false;
        }

        if is_selected {
            build_curve_selector_inline(ui, clip, bone_roles, bone_id, editor_state);
        }
    }

    if !clip.morph_tracks.is_empty() {
        ui.separator();
        ui.text("Morphs:");
        ui.separator();
        for (i, morph_track) in clip.morph_tracks.iter().enumerate() {
            let is_selected = editor_state.selected_target == Some(CurveEditorTarget::Morph(i));
            let label = format_morph_track_name(&morph_track.source_mesh, &morph_track.channel);
            if ui.selectable_config(&label).selected(is_selected).build() {
                editor_state.select_morph(i);
                editor_state.view_initialized = false;
            }
        }
    }
}
/// Every domain channel gets a row, keyed or not: `+` inserts a key at the
/// playhead with the component's current value (creating the curve on first
/// use), so an empty clip opened in the editor still offers a keying path.
pub(super) fn build_scalar_curve_selector_inline(
    ui: &imgui::Ui,
    world: &World,
    clip: &EditableAnimationClip,
    editor_state: &mut CurveEditorState,
    domain: &'static ScalarChannelDomain,
) {
    ui.indent();

    for (index, channel) in domain.channels().iter().enumerate() {
        let property_type = domain.property_type_at(index);
        let key_count = clip
            .get_scalar_curve(property_type)
            .map(|curve| curve.keyframes.len())
            .unwrap_or(0);

        let (color, name) = scalar_curve_style(property_type);
        let mut visible = editor_state.visible_curves.contains(&property_type);
        draw_curve_color_swatch(ui, color);
        ui.same_line();
        let label = if key_count > 0 {
            format!("{name} ({key_count})")
        } else {
            name.to_string()
        };
        if ui.checkbox(&label, &mut visible) {
            if visible {
                editor_state.visible_curves.insert(property_type);
            } else {
                editor_state.visible_curves.remove(&property_type);
            }
        }
        ui.same_line();
        if ui.small_button(&format!("+##key_{}", channel.cli_name)) {
            world.send_command(ScalarCurveEvent::InsertScalarKeyAtPlayhead { property_type });
            editor_state.visible_curves.insert(property_type);
        }
        if ui.is_item_hovered() {
            ui.tooltip_text("Insert key at playhead (current value)");
        }
    }

    if ui.small_button("All##scalar") {
        for index in 0..domain.channels().len() {
            editor_state
                .visible_curves
                .insert(domain.property_type_at(index));
        }
    }
    ui.same_line();
    if ui.small_button("None##scalar") {
        for index in 0..domain.channels().len() {
            editor_state
                .visible_curves
                .remove(&domain.property_type_at(index));
        }
    }

    ui.unindent();
    ui.spacing();
}
pub(super) fn is_curve_listed(
    clip: &EditableAnimationClip,
    bone_roles: &[(BoneId, HumanoidRole)],
    bone_id: BoneId,
    property_type: PropertyType,
) -> bool {
    match find_bone_role(bone_roles, bone_id) {
        Some(role) => is_role_curve_allowed(role, property_type),
        None => clip
            .tracks
            .get(&bone_id)
            .is_some_and(|track| !track.get_curve(property_type).is_empty()),
    }
}
pub(super) fn is_role_curve_allowed(role: HumanoidRole, property_type: PropertyType) -> bool {
    match property_type {
        PropertyType::RotationX | PropertyType::RotationY | PropertyType::RotationZ => true,
        PropertyType::TranslationX | PropertyType::TranslationY | PropertyType::TranslationZ => {
            role.allows_translation()
        }
        _ => false,
    }
}
pub(super) fn build_curve_selector_inline(
    ui: &imgui::Ui,
    clip: &EditableAnimationClip,
    bone_roles: &[(BoneId, HumanoidRole)],
    bone_id: BoneId,
    editor_state: &mut CurveEditorState,
) {
    ui.indent();

    for (prop_type, color, name) in ALL_PROPERTY_TYPES {
        if !is_curve_listed(clip, bone_roles, bone_id, *prop_type) {
            continue;
        }

        let mut visible = editor_state.visible_curves.contains(prop_type);
        draw_curve_color_swatch(ui, *color);
        ui.same_line();
        if ui.checkbox(name, &mut visible) {
            if visible {
                editor_state.visible_curves.insert(*prop_type);
            } else {
                editor_state.visible_curves.remove(prop_type);
            }
        }
    }

    if ui.small_button("All") {
        for (prop_type, _, _) in ALL_PROPERTY_TYPES {
            if is_curve_listed(clip, bone_roles, bone_id, *prop_type) {
                editor_state.visible_curves.insert(*prop_type);
            }
        }
    }
    ui.same_line();
    if ui.small_button("None") {
        editor_state.visible_curves.clear();
    }

    ui.unindent();
    ui.spacing();
}
pub(super) fn format_morph_track_name(source_mesh: &str, channel: &str) -> String {
    let label = format!("{}/{}", source_mesh, channel);
    if label.chars().count() > 20 {
        let tail: String = label.chars().skip(label.chars().count() - 17).collect();
        format!("...{tail}")
    } else {
        label
    }
}
pub(super) fn scalar_curve_style(property_type: PropertyType) -> ([f32; 4], &'static str) {
    match scalar_channel_for_property(property_type) {
        Some((domain, channel)) => (
            scalar_channel_color(domain, property_type),
            channel.display_name,
        ),
        None => ([0.6, 0.6, 0.6, 1.0], "Custom"),
    }
}
pub(super) fn scalar_channel_color(
    domain: &ScalarChannelDomain,
    property_type: PropertyType,
) -> [f32; 4] {
    // Evenly spaced hues over the domain's channels, alternating brightness
    // for neighbor separability.
    let index = domain.channel_index(property_type).unwrap_or(0);
    let hue = index as f32 / domain.channels().len().max(1) as f32;
    let value = if index % 2 == 0 { 1.0 } else { 0.75 };
    hsv_to_rgba(hue, 0.75, value)
}
pub(super) fn hsv_to_rgba(h: f32, s: f32, v: f32) -> [f32; 4] {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r, g, b, 1.0]
}
pub(super) fn draw_curve_color_swatch(ui: &imgui::Ui, color: [f32; 4]) {
    let side = ui.text_line_height() * 0.6;
    let top_left = ui.cursor_screen_pos();
    let offset_y = (ui.text_line_height() - side) * 0.5;
    let min = [top_left[0], top_left[1] + offset_y];
    let max = [min[0] + side, min[1] + side];
    ui.get_window_draw_list()
        .add_rect(min, max, color)
        .filled(true)
        .build();
    ui.dummy([side, ui.text_line_height()]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_morph_track_name_short() {
        assert_eq!(format_morph_track_name("mesh", "smile"), "mesh/smile");
    }

    #[test]
    fn test_format_morph_track_name_truncated() {
        let result = format_morph_track_name("very_long_source_mesh", "very_long_channel");
        assert_eq!(result.len(), 20);
        assert!(result.starts_with("..."));
        assert!(result.ends_with("channel"));
    }

    #[test]
    fn test_format_morph_track_name_truncates_multibyte_on_char_boundary() {
        let result = format_morph_track_name("顔メッシュ", "まばたき左目を閉じる強め表情差分");
        assert_eq!(result.chars().count(), 20);
        assert!(result.starts_with("..."));
        assert!(result.ends_with("表情差分"));
    }

    #[test]
    fn mapped_bones_list_rotation_curves_before_any_key_exists() {
        use crate::ecs::systems::{
            build_humanoid_rig, copy_test_humanoid_fixture, find_first_skeleton,
            test_humanoid_world,
        };

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (world, assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets)
            .expect("fixture has a skeleton")
            .clone();
        world.resource_mut::<HumanoidRigState>().rig =
            Some(build_humanoid_rig(&fbx_path, &skeleton, None).expect("fixture is humanoid"));

        let bone_roles = collect_humanoid_bone_roles(&world);
        let role_bone = |role: HumanoidRole| {
            bone_roles
                .iter()
                .find(|(_, bone_role)| *bone_role == role)
                .map(|(bone_id, _)| *bone_id)
                .expect("role is mapped")
        };
        let hips = role_bone(HumanoidRole::Hips);
        let head = role_bone(HumanoidRole::Head);
        let clip = EditableAnimationClip::new(0, "empty".to_string());

        assert!(collect_listed_bone_ids(&clip, &bone_roles).contains(&head));
        assert!(is_curve_listed(
            &clip,
            &bone_roles,
            head,
            PropertyType::RotationZ
        ));
        assert!(!is_curve_listed(
            &clip,
            &bone_roles,
            head,
            PropertyType::TranslationY
        ));
        assert!(is_curve_listed(
            &clip,
            &bone_roles,
            hips,
            PropertyType::TranslationY
        ));
        assert!(!is_curve_listed(
            &clip,
            &bone_roles,
            hips,
            PropertyType::ScaleX
        ));
    }

    #[test]
    fn unmapped_bones_list_only_keyed_curves() {
        let mut clip = EditableAnimationClip::new(0, "bone".to_string());
        let track = clip.add_track(0, "Spine".to_string());
        crate::animation::editable::curve_add_keyframe(&mut track.rotation_x, 0.0, 1.0);

        assert!(is_curve_listed(&clip, &[], 0, PropertyType::RotationX));
        assert!(!is_curve_listed(&clip, &[], 0, PropertyType::RotationY));
    }
}
