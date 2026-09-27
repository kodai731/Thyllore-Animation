use thyllore_anim_core::editable::components::morph_track::MorphTrack;
use thyllore_anim_core::editable::EditableAnimationClip;

use crate::components::fbx::FbxBlendShapeExport;
use crate::fbx_animation::{
    build_channel_exports, build_curve_export, FbxChannel, FbxCurveExport, FbxCurveNodeExport,
    UidAllocator,
};

const MORPH_WEIGHT_TO_PERCENT: f32 = 100.0;

pub(crate) fn build_animation_curves(
    clip: Option<&EditableAnimationClip>,
    bone_name_to_model_uid: &std::collections::HashMap<String, i64>,
    blend_shapes: &[FbxBlendShapeExport],
    uid_alloc: &mut UidAllocator,
    inv_unit_scale: f32,
) -> (Vec<FbxCurveNodeExport>, Vec<FbxCurveExport>) {
    let mut curve_nodes = Vec::new();
    let mut curves = Vec::new();

    let Some(clip) = clip else {
        return (curve_nodes, curves);
    };

    for track in clip.tracks.values() {
        let bone_model_uid = match bone_name_to_model_uid.get(track.bone_name.as_str()) {
            Some(&uid) => uid,
            None => continue,
        };

        if let Some((node, node_curves)) = build_channel_exports(
            [
                &track.translation_x,
                &track.translation_y,
                &track.translation_z,
            ],
            bone_model_uid,
            FbxChannel::Translation,
            uid_alloc,
            inv_unit_scale,
        ) {
            curve_nodes.push(node);
            curves.extend(node_curves);
        }

        if let Some((node, node_curves)) = build_channel_exports(
            [&track.rotation_x, &track.rotation_y, &track.rotation_z],
            bone_model_uid,
            FbxChannel::Rotation,
            uid_alloc,
            inv_unit_scale,
        ) {
            curve_nodes.push(node);
            curves.extend(node_curves);
        }

        if let Some((node, node_curves)) = build_channel_exports(
            [&track.scale_x, &track.scale_y, &track.scale_z],
            bone_model_uid,
            FbxChannel::Scale,
            uid_alloc,
            inv_unit_scale,
        ) {
            curve_nodes.push(node);
            curves.extend(node_curves);
        }
    }

    for morph_track in &clip.morph_tracks {
        build_morph_track_exports(
            morph_track,
            blend_shapes,
            uid_alloc,
            &mut curve_nodes,
            &mut curves,
        );
    }

    (curve_nodes, curves)
}

fn build_morph_track_exports(
    morph_track: &MorphTrack,
    blend_shapes: &[FbxBlendShapeExport],
    uid_alloc: &mut UidAllocator,
    curve_nodes: &mut Vec<FbxCurveNodeExport>,
    curves: &mut Vec<FbxCurveExport>,
) {
    if morph_track.curve.is_empty() {
        return;
    }

    let target_channel_uids = blend_shapes
        .iter()
        .filter(|blend_shape| blend_shape.source_mesh == morph_track.source_mesh)
        .flat_map(|blend_shape| &blend_shape.channels)
        .filter(|channel| channel.name == morph_track.channel)
        .map(|channel| channel.channel_uid);

    for channel_uid in target_channel_uids {
        let curve_node_uid = uid_alloc.allocate();
        let curve_uid = uid_alloc.allocate();
        let curve = build_curve_export(&morph_track.curve, curve_uid, MORPH_WEIGHT_TO_PERCENT);

        curve_nodes.push(FbxCurveNodeExport {
            uid: curve_node_uid,
            bone_model_uid: channel_uid,
            channel: FbxChannel::DeformPercent,
            default_values: [curve.default_value, 0.0, 0.0],
            curve_uids: [Some(curve_uid), None, None],
        });
        curves.push(curve);
    }
}
