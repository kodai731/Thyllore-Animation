use crate::animation::editable::{
    align_opposite_handle, compute_snap_threshold_time, snap_time, BezierHandle, PropertyCurve,
    SnapSettings, TangentContinuity,
};
use crate::ecs::resource::{
    AxisLock, CurveSelectedKeyframe, CurveViewMode, DraggingTangent, TangentHandleType,
};

use super::view::ViewTransform;

const AXIS_LOCK_THRESHOLD_PX: f32 = 4.0;

#[derive(Clone, Debug)]
pub(super) struct KeyTimeSnap {
    settings: SnapSettings,
    threshold_time: f32,
}

impl KeyTimeSnap {
    #[must_use]
    pub(super) fn new(settings: SnapSettings, pixels_per_second: f32) -> Self {
        let threshold_time =
            compute_snap_threshold_time(settings.snap_threshold_px, pixels_per_second);
        Self {
            settings,
            threshold_time,
        }
    }

    #[must_use]
    fn disabled(settings: &SnapSettings) -> Self {
        Self {
            settings: SnapSettings {
                snap_to_frame: false,
                snap_to_key: false,
                ..settings.clone()
            },
            threshold_time: 0.0,
        }
    }
}

#[must_use]
pub(super) fn effective_axis_lock(
    view_mode: CurveViewMode,
    axis_lock: Option<AxisLock>,
) -> Option<AxisLock> {
    match view_mode {
        CurveViewMode::Curves => axis_lock,
        CurveViewMode::Dopesheet => Some(AxisLock::Time),
    }
}

#[must_use]
pub(super) fn resolve_axis_lock(
    current: Option<AxisLock>,
    shift_held: bool,
    pixel_delta: [f32; 2],
) -> Option<AxisLock> {
    if !shift_held {
        return None;
    }
    if current.is_some() {
        return current;
    }

    let dx = pixel_delta[0].abs();
    let dy = pixel_delta[1].abs();
    if dx.max(dy) <= AXIS_LOCK_THRESHOLD_PX {
        None
    } else if dx >= dy {
        Some(AxisLock::Time)
    } else {
        Some(AxisLock::Value)
    }
}

#[must_use]
pub(super) fn resolve_key_time_snap(
    ui: &imgui::Ui,
    snap_settings: &SnapSettings,
    vt: &ViewTransform,
) -> KeyTimeSnap {
    if ui.io().key_ctrl() {
        KeyTimeSnap::disabled(snap_settings)
    } else {
        KeyTimeSnap::new(snap_settings.clone(), vt.pixels_per_second())
    }
}

#[must_use]
pub(super) fn collect_unselected_key_times(
    curve: &PropertyCurve,
    selected_keyframes: &[CurveSelectedKeyframe],
) -> Vec<f32> {
    curve
        .keyframes
        .iter()
        .filter(|kf| {
            !selected_keyframes
                .iter()
                .any(|sel| sel.property_type == curve.property_type && sel.keyframe_id == kf.id)
        })
        .map(|kf| kf.time)
        .collect()
}

#[must_use]
pub(super) fn dragged_key_position(
    original: [f32; 2],
    delta: [f32; 2],
    axis_lock: Option<AxisLock>,
    time_snap: &KeyTimeSnap,
    nearby_times: &[f32],
) -> [f32; 2] {
    let locked_delta = match axis_lock {
        Some(AxisLock::Time) => [delta[0], 0.0],
        Some(AxisLock::Value) => [0.0, delta[1]],
        None => delta,
    };

    let free_time = original[0] + locked_delta[0];
    let snapped_time = snap_time(
        free_time,
        &time_snap.settings,
        nearby_times,
        time_snap.threshold_time,
    );

    [snapped_time.max(0.0), original[1] + locked_delta[1]]
}

#[must_use]
pub(super) fn dragged_key_delta(
    curve_vt: &ViewTransform,
    drag_start: [f32; 2],
    mouse_pos: [f32; 2],
) -> [f32; 2] {
    [
        curve_vt.x_to_time(mouse_pos[0]) - curve_vt.x_to_time(drag_start[0]),
        curve_vt.y_to_value(mouse_pos[1]) - curve_vt.y_to_value(drag_start[1]),
    ]
}

pub(super) fn compute_dragged_tangent(
    dragging: &DraggingTangent,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> (BezierHandle, BezierHandle) {
    for (curve, _, _) in curves_to_draw {
        if curve.property_type != dragging.property_type {
            continue;
        }

        let kf = match curve.get_keyframe(dragging.keyframe_id) {
            Some(kf) => kf,
            None => break,
        };

        let curve_vt = vt.for_curve(curve);
        let mouse_time = curve_vt.x_to_time(mouse_pos[0]);
        let mouse_value = curve_vt.y_to_value(mouse_pos[1]);
        let time_offset = mouse_time - kf.time;
        let value_offset = mouse_value - kf.value;
        let new_handle = BezierHandle::new(time_offset, value_offset);

        let (in_tangent, out_tangent) = match dragging.handle_type {
            TangentHandleType::In => (new_handle, kf.out_tangent.clone()),
            TangentHandleType::Out => (kf.in_tangent.clone(), new_handle),
        };

        if kf.continuity == TangentContinuity::Unified {
            return match dragging.handle_type {
                TangentHandleType::In => {
                    let aligned_out = align_opposite_handle(&in_tangent, &out_tangent);
                    (in_tangent, aligned_out)
                }
                TangentHandleType::Out => {
                    let aligned_in = align_opposite_handle(&out_tangent, &in_tangent);
                    (aligned_in, out_tangent)
                }
            };
        }

        return (in_tangent, out_tangent);
    }

    (BezierHandle::linear(), BezierHandle::linear())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{PropertyType, TangentContinuity};

    use super::super::test_fixtures::*;

    #[test]
    fn normalized_drag_value_delta_scales_with_each_curve_half_width() {
        let small = build_two_point_curve(PropertyType::TranslationX, [0.0, 10.0]);
        let large = build_two_point_curve(PropertyType::TranslationY, [100.0, -100.0]);
        let curves = [(&small, [1.0; 4], "x"), (&large, [1.0; 4], "y")];
        let vt = build_normalized_pixel_view();
        let drag_start = [50.0, 50.0];
        let mouse_pos = [50.0, 40.0];
        let normalized_delta = 0.2;

        let small_delta = dragged_key_delta(
            &vt.for_property(PropertyType::TranslationX, &curves),
            drag_start,
            mouse_pos,
        );
        let large_delta = dragged_key_delta(
            &vt.for_property(PropertyType::TranslationY, &curves),
            drag_start,
            mouse_pos,
        );

        assert!((small_delta[1] - normalized_delta * 5.0).abs() < 1e-4);
        assert!((large_delta[1] - normalized_delta * 100.0).abs() < 1e-3);
        assert_eq!(small_delta[0], large_delta[0]);
    }

    #[test]
    fn dragging_one_handle_of_a_unified_key_realigns_the_opposite_handle() {
        let mut curve = build_two_point_curve(PropertyType::TranslationX, [0.0, 1.0]);
        curve.keyframes[0].in_tangent = BezierHandle::new(-0.3, -0.2);
        let kf = &curve.keyframes[0];
        assert_eq!(kf.continuity, TangentContinuity::Unified);
        let dragging = DraggingTangent {
            property_type: PropertyType::TranslationX,
            keyframe_id: kf.id,
            handle_type: TangentHandleType::Out,
            original_handle: kf.out_tangent.clone(),
        };
        let vt = build_pixel_view();
        let curves = [(&curve, [1.0; 4], "x")];

        let (in_tangent, out_tangent) =
            compute_dragged_tangent(&dragging, [40.0, 20.0], &curves, &vt);

        let in_length = (in_tangent.time_offset.powi(2) + in_tangent.value_offset.powi(2)).sqrt();
        let original_length: f32 = (0.3f32.powi(2) + 0.2f32.powi(2)).sqrt();
        assert!((in_length - original_length).abs() < 1e-5);
        let cross = in_tangent.time_offset * out_tangent.value_offset
            - in_tangent.value_offset * out_tangent.time_offset;
        assert!(cross.abs() < 1e-5);
        assert!(in_tangent.time_offset * out_tangent.time_offset < 0.0);
    }

    #[test]
    fn axis_lock_stays_unresolved_below_the_threshold() {
        assert_eq!(resolve_axis_lock(None, true, [3.0, -3.0]), None);
    }

    #[test]
    fn axis_lock_picks_time_for_a_horizontal_move() {
        assert_eq!(
            resolve_axis_lock(None, true, [6.0, 1.0]),
            Some(AxisLock::Time)
        );
    }

    #[test]
    fn axis_lock_picks_value_for_a_vertical_move() {
        assert_eq!(
            resolve_axis_lock(None, true, [1.0, -6.0]),
            Some(AxisLock::Value)
        );
    }

    #[test]
    fn axis_lock_keeps_the_first_direction_while_shift_is_held() {
        assert_eq!(
            resolve_axis_lock(Some(AxisLock::Time), true, [6.0, 40.0]),
            Some(AxisLock::Time)
        );
    }

    #[test]
    fn axis_lock_is_released_with_shift() {
        assert_eq!(
            resolve_axis_lock(Some(AxisLock::Time), false, [6.0, 0.0]),
            None
        );
    }

    fn build_snap(snap_settings: SnapSettings) -> KeyTimeSnap {
        KeyTimeSnap::new(snap_settings, 100.0)
    }

    fn free_snap() -> KeyTimeSnap {
        build_snap(SnapSettings {
            snap_to_frame: false,
            ..SnapSettings::default()
        })
    }

    fn frame_snap() -> KeyTimeSnap {
        build_snap(SnapSettings::default())
    }

    fn key_snap() -> KeyTimeSnap {
        build_snap(SnapSettings {
            snap_to_frame: false,
            snap_to_key: true,
            ..SnapSettings::default()
        })
    }

    fn frame_and_key_snap() -> KeyTimeSnap {
        build_snap(SnapSettings {
            snap_to_key: true,
            ..SnapSettings::default()
        })
    }

    #[test]
    fn dragged_key_position_time_lock_keeps_the_value() {
        let position = dragged_key_position(
            [1.0, 2.0],
            [0.5, 3.0],
            Some(AxisLock::Time),
            &free_snap(),
            &[],
        );

        assert_eq!(position, [1.5, 2.0]);
    }

    #[test]
    fn dragged_key_position_value_lock_keeps_the_time() {
        let position = dragged_key_position(
            [1.0, 2.0],
            [0.5, 3.0],
            Some(AxisLock::Value),
            &free_snap(),
            &[],
        );

        assert_eq!(position, [1.0, 5.0]);
    }

    #[test]
    fn dragged_key_position_snaps_time_to_30fps_frames() {
        let [time, value] =
            dragged_key_position([0.0, 1.0], [0.05, 0.25], None, &frame_snap(), &[]);

        assert!((time - 2.0 / 30.0).abs() < 1e-6);
        assert_eq!(value, 1.25);
    }

    #[test]
    fn dragged_key_position_free_keeps_the_raw_time_and_clamps_at_zero() {
        assert_eq!(
            dragged_key_position([1.0, 0.0], [0.013, 0.0], None, &free_snap(), &[]),
            [1.013, 0.0]
        );
        assert_eq!(
            dragged_key_position([0.1, 0.0], [-0.5, 0.0], None, &frame_snap(), &[]),
            [0.0, 0.0]
        );
    }

    #[test]
    fn dragged_key_position_snaps_to_a_nearby_key_within_the_pixel_threshold() {
        let [near_time, _] =
            dragged_key_position([0.0, 0.0], [0.47, 0.0], None, &frame_and_key_snap(), &[0.5]);
        let [far_time, _] = dragged_key_position([0.0, 0.0], [0.3, 0.0], None, &key_snap(), &[0.5]);

        assert_eq!(near_time, 0.5);
        assert_eq!(far_time, 0.3);
    }

    #[test]
    fn unselected_key_times_skip_the_dragged_keys() {
        let curve = build_two_point_curve(PropertyType::TranslationX, [0.0, 1.0]);
        let dragged = CurveSelectedKeyframe {
            property_type: PropertyType::TranslationX,
            keyframe_id: curve.keyframes[0].id,
            original_time: curve.keyframes[0].time,
            original_value: curve.keyframes[0].value,
        };

        assert_eq!(
            collect_unselected_key_times(&curve, &[dragged]),
            vec![curve.keyframes[1].time]
        );
    }

    #[test]
    fn effective_axis_lock_keeps_curves_lock_and_forces_time_in_dopesheet() {
        assert_eq!(effective_axis_lock(CurveViewMode::Curves, None), None);
        assert_eq!(
            effective_axis_lock(CurveViewMode::Curves, Some(AxisLock::Value)),
            Some(AxisLock::Value)
        );
        assert_eq!(
            effective_axis_lock(CurveViewMode::Dopesheet, None),
            Some(AxisLock::Time)
        );
        assert_eq!(
            effective_axis_lock(CurveViewMode::Dopesheet, Some(AxisLock::Value)),
            Some(AxisLock::Time)
        );
    }
}
