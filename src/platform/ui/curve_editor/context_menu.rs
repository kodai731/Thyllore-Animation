use std::collections::HashSet;

use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use super::track_list::{collect_humanoid_bone_roles, find_bone_role, is_role_curve_allowed};
use super::window::{get_current_clip, ALL_PROPERTY_TYPES};
use crate::animation::editable::{
    CurveExtrapolation, InterpolationType, PropertyCurve, PropertyType, TangentType,
    TangentWeightMode,
};
use crate::animation::BoneId;
use crate::ecs::component::scalar_channel_for_property;
use crate::ecs::resource::{
    ClipLibrary, CurveEditorState, CurveSelectedKeyframe, CurveTrackRef, TimelineState,
};
use crate::ecs::systems::phases::event_dispatch::bone_track::EnsureBoneTrack;
use crate::ecs::systems::phases::event_dispatch::timeline::{ExtrapolationEnd, TimelineEvent};
use crate::ecs::world::World;
use crate::platform::ui::theme::shadow::draw_window_shadow;

pub(super) fn build_keyframe_context_menu(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    track_ref: CurveTrackRef,
) {
    ui.popup("keyframe_context_menu", || {
        draw_window_shadow(ui, ui.clone_style().popup_rounding);

        let ctx_kf = match editor_state.context_menu_keyframe.clone() {
            Some(kf) => kf,
            None => return,
        };

        if ui.selectable_config("Delete Key").build() {
            if editor_state.selected_keyframes.len() > 1 {
                for sel in &editor_state.selected_keyframes {
                    world.send_command(TimelineEvent::DeleteKeyframe {
                        track: track_ref,
                        property_type: sel.property_type.clone(),
                        keyframe_id: sel.keyframe_id,
                    });
                }
                editor_state.selected_keyframes.clear();
                editor_state.selection_anchor = None;
            } else {
                world.send_command(TimelineEvent::DeleteKeyframe {
                    track: track_ref,
                    property_type: ctx_kf.property_type.clone(),
                    keyframe_id: ctx_kf.keyframe_id,
                });
                editor_state.selected_keyframes.clear();
                editor_state.selection_anchor = None;
            }
        }

        let section_color = [0.6, 0.8, 1.0, 1.0];

        ui.separator();
        ui.text_colored(section_color, "Interpolation");
        ui.separator();

        if ui.selectable_config("  Linear").build() {
            world.send_command(TimelineEvent::SetKeyframeInterpolation {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                interpolation: InterpolationType::Linear,
            });
        }

        if ui.selectable_config("  Bezier").build() {
            world.send_command(TimelineEvent::SetKeyframeInterpolation {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                interpolation: InterpolationType::Bezier,
            });
        }

        if ui.selectable_config("  Stepped").build() {
            world.send_command(TimelineEvent::SetKeyframeInterpolation {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                interpolation: InterpolationType::Stepped,
            });
        }

        ui.spacing();
        ui.text_colored(section_color, "Tangent");
        ui.separator();

        let tangent_options = [
            ("  Spline", TangentType::Spline),
            ("  Linear", TangentType::Linear),
            ("  Flat", TangentType::Flat),
            ("  Clamped", TangentType::Clamped),
            ("  Plateau", TangentType::Plateau),
            ("  Manual", TangentType::Manual),
        ];

        for (label, tangent_type) in &tangent_options {
            if ui.selectable_config(label).build() {
                world.send_command(TimelineEvent::SetTangentType {
                    track: track_ref,
                    property_type: ctx_kf.property_type,
                    keyframe_id: ctx_kf.keyframe_id,
                    tangent_type: *tangent_type,
                });
            }
        }

        ui.spacing();
        ui.text_colored(section_color, "Weight");
        ui.separator();

        if ui.selectable_config("  Non-Weighted").build() {
            world.send_command(TimelineEvent::SetTangentWeightMode {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                weight_mode: TangentWeightMode::NonWeighted,
            });
        }

        if ui.selectable_config("  Weighted").build() {
            world.send_command(TimelineEvent::SetTangentWeightMode {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                weight_mode: TangentWeightMode::Weighted,
            });
        }
    });
}

pub(super) fn build_curve_editor_context_menu(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &CurveEditorState,
    track_ref: CurveTrackRef,
) {
    ui.popup("curve_editor_context_menu", || {
        draw_window_shadow(ui, ui.clone_style().popup_rounding);

        if ui.selectable_config("Add Key").build() {
            let bone_role = track_ref
                .bone_id()
                .and_then(|bone_id| find_bone_role(&collect_humanoid_bone_roles(world), bone_id));
            if let Some(property_type) = add_key_target_property(editor_state, track_ref, bone_role)
            {
                if let CurveTrackRef::Bone(bone_id) = track_ref {
                    if !current_clip_has_track(world, bone_id) {
                        world.send_command(EnsureBoneTrack { bone_id });
                    }
                }
                world.send_command(TimelineEvent::AddKeyframe {
                    track: track_ref,
                    property_type,
                    time: editor_state.context_menu_click_time.max(0.0),
                    value: editor_state.context_menu_click_value,
                });
            }
        }
    });
}

pub(super) const CURVE_EXTRAPOLATION_MENU: &str = "curve_extrapolation_menu";

const EXTRAPOLATION_ENDS: [(&str, ExtrapolationEnd); 3] = [
    ("Pre", ExtrapolationEnd::Pre),
    ("Post", ExtrapolationEnd::Post),
    ("Both", ExtrapolationEnd::Both),
];

const EXTRAPOLATION_MODES: [(&str, CurveExtrapolation); 4] = [
    ("Constant", CurveExtrapolation::Constant),
    ("Linear", CurveExtrapolation::Linear),
    ("Cycle", CurveExtrapolation::Cycle),
    ("Cycle with Offset", CurveExtrapolation::CycleWithOffset),
];

pub(super) fn build_curve_extrapolation_menu(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &CurveEditorState,
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    track_ref: CurveTrackRef,
) {
    ui.popup(CURVE_EXTRAPOLATION_MENU, || {
        draw_window_shadow(ui, ui.clone_style().popup_rounding);

        for (end_label, end) in EXTRAPOLATION_ENDS {
            ui.menu(end_label, || {
                for (mode_label, mode) in EXTRAPOLATION_MODES {
                    if ui.menu_item(mode_label) {
                        for property_type in extrapolation_target_properties(
                            &editor_state.selected_keyframes,
                            curves,
                        ) {
                            world.send_command(TimelineEvent::SetCurveExtrapolation {
                                track: track_ref,
                                property_type,
                                end,
                                mode,
                            });
                        }
                    }
                }
            });
        }
    });
}

pub(super) fn extrapolation_target_properties(
    selected_keyframes: &[CurveSelectedKeyframe],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
) -> Vec<PropertyType> {
    let mut targets: Vec<PropertyType> = if selected_keyframes.is_empty() {
        curves
            .iter()
            .map(|(curve, _, _)| curve.property_type)
            .collect()
    } else {
        selected_keyframes
            .iter()
            .map(|selected| selected.property_type)
            .collect()
    };
    let mut seen = HashSet::new();
    targets.retain(|property_type| seen.insert(*property_type));
    targets
}

pub(super) fn current_clip_has_track(world: &World, bone_id: BoneId) -> bool {
    let timeline_state = world.resource::<TimelineState>();
    let clip_library = world.resource::<ClipLibrary>();
    get_current_clip(&timeline_state, &clip_library)
        .is_some_and(|clip| clip.tracks.contains_key(&bone_id))
}

/// The scalar target only accepts registered channels: the visible set can
/// still hold bone property types from a previous bone target, and letting one
/// through would create a curve no channel answers to (grey "Custom", never
/// sampled). Lowest code wins so the choice is deterministic; the bone target
/// picks by declaration order for the same reason. A role bone only lists the
/// curves its role allows, so a translation left visible by the default set
/// must not win over the rotation the user checked.
pub(super) fn add_key_target_property(
    editor_state: &CurveEditorState,
    track_ref: CurveTrackRef,
    bone_role: Option<HumanoidRole>,
) -> Option<PropertyType> {
    match track_ref {
        CurveTrackRef::Scalar => editor_state
            .visible_curves
            .iter()
            .copied()
            .filter(|p| scalar_channel_for_property(*p).is_some())
            .min_by_key(|p| match p {
                PropertyType::Custom(code) => *code,
                _ => u16::MAX,
            }),
        CurveTrackRef::Bone(_) => ALL_PROPERTY_TYPES
            .iter()
            .map(|(property_type, _, _)| *property_type)
            .filter(|property_type| {
                bone_role.is_none_or(|role| is_role_curve_allowed(role, *property_type))
            })
            .find(|property_type| editor_state.visible_curves.contains(property_type)),
        CurveTrackRef::Morph(_) => Some(PropertyType::MorphWeight),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_key_on_a_role_bone_skips_translations_left_visible_by_default() {
        let mut editor_state = CurveEditorState::default();
        editor_state.visible_curves.remove(&PropertyType::RotationX);
        editor_state.visible_curves.remove(&PropertyType::RotationZ);

        let property = add_key_target_property(
            &editor_state,
            CurveTrackRef::Bone(3),
            Some(HumanoidRole::Neck),
        );

        assert_eq!(property, Some(PropertyType::RotationY));
    }

    #[test]
    fn add_key_on_a_plain_bone_keeps_declaration_order() {
        let editor_state = CurveEditorState::default();

        let property = add_key_target_property(&editor_state, CurveTrackRef::Bone(3), None);

        assert_eq!(property, Some(PropertyType::TranslationX));
    }

    #[test]
    fn extrapolation_targets_selected_curves_or_every_visible_curve() {
        let curve_x = PropertyCurve::new(0, PropertyType::TranslationX);
        let curve_y = PropertyCurve::new(1, PropertyType::TranslationY);
        let curves = [
            (&curve_x, [1.0, 0.0, 0.0, 1.0], "x"),
            (&curve_y, [0.0, 1.0, 0.0, 1.0], "y"),
        ];
        let selected_on_y = |keyframe_id| CurveSelectedKeyframe {
            property_type: PropertyType::TranslationY,
            keyframe_id,
            original_time: 0.0,
            original_value: 0.0,
        };

        let without_selection = extrapolation_target_properties(&[], &curves);
        let with_selection =
            extrapolation_target_properties(&[selected_on_y(0), selected_on_y(1)], &curves);

        assert_eq!(
            without_selection,
            vec![PropertyType::TranslationX, PropertyType::TranslationY]
        );
        assert_eq!(with_selection, vec![PropertyType::TranslationY]);
    }
}
