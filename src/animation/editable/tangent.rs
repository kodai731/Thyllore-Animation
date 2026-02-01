use super::keyframe::{BezierHandle, EditableKeyframe};

pub fn sample_bezier(
    k0_time: f32,
    k0_value: f32,
    k0_out: &BezierHandle,
    k1_time: f32,
    k1_value: f32,
    k1_in: &BezierHandle,
    time: f32,
) -> f32 {
    let p0_t = k0_time;
    let p1_t = k0_time + k0_out.time_offset;
    let p2_t = k1_time + k1_in.time_offset;
    let p3_t = k1_time;

    let p0_v = k0_value;
    let p1_v = k0_value + k0_out.value_offset;
    let p2_v = k1_value + k1_in.value_offset;
    let p3_v = k1_value;

    let u = find_bezier_t_for_time(p0_t, p1_t, p2_t, p3_t, time);

    evaluate_cubic(p0_v, p1_v, p2_v, p3_v, u)
}

fn find_bezier_t_for_time(
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    target: f32,
) -> f32 {
    let mut t = (target - p0) / (p3 - p0).max(0.0001);
    t = t.clamp(0.0, 1.0);

    for _ in 0..8 {
        let current = evaluate_cubic(p0, p1, p2, p3, t);
        let derivative = evaluate_cubic_derivative(p0, p1, p2, p3, t);

        if derivative.abs() < 1e-8 {
            break;
        }

        let correction = (current - target) / derivative;
        t -= correction;
        t = t.clamp(0.0, 1.0);

        if correction.abs() < 1e-6 {
            break;
        }
    }

    t
}

fn evaluate_cubic(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let one_minus_t = 1.0 - t;
    let one_minus_t2 = one_minus_t * one_minus_t;
    let t2 = t * t;

    one_minus_t2 * one_minus_t * p0
        + 3.0 * one_minus_t2 * t * p1
        + 3.0 * one_minus_t * t2 * p2
        + t2 * t * p3
}

fn evaluate_cubic_derivative(
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    t: f32,
) -> f32 {
    let one_minus_t = 1.0 - t;

    3.0 * one_minus_t * one_minus_t * (p1 - p0)
        + 6.0 * one_minus_t * t * (p2 - p1)
        + 3.0 * t * t * (p3 - p2)
}

pub fn apply_auto_tangent(keyframes: &mut [EditableKeyframe], index: usize) {
    let len = keyframes.len();
    if len < 2 || index >= len {
        return;
    }

    let (slope, dt) = if index == 0 {
        let k0 = &keyframes[0];
        let k1 = &keyframes[1];
        let dt = k1.time - k0.time;
        if dt.abs() < 1e-8 {
            return;
        }
        ((k1.value - k0.value) / dt, dt)
    } else if index == len - 1 {
        let k0 = &keyframes[len - 2];
        let k1 = &keyframes[len - 1];
        let dt = k1.time - k0.time;
        if dt.abs() < 1e-8 {
            return;
        }
        ((k1.value - k0.value) / dt, dt)
    } else {
        let prev = &keyframes[index - 1];
        let next = &keyframes[index + 1];
        let dt_total = next.time - prev.time;
        if dt_total.abs() < 1e-8 {
            return;
        }
        let slope = (next.value - prev.value) / dt_total;
        let dt = (next.time - prev.time) * 0.5;
        (slope, dt)
    };

    let handle_time = dt / 3.0;
    let handle_value = slope * handle_time;

    keyframes[index].in_tangent = BezierHandle::new(-handle_time, -handle_value);
    keyframes[index].out_tangent = BezierHandle::new(handle_time, handle_value);
}

pub fn apply_flat_tangent(keyframe: &mut EditableKeyframe, dt: f32) {
    let handle_time = dt / 3.0;
    keyframe.in_tangent = BezierHandle::new(-handle_time, 0.0);
    keyframe.out_tangent = BezierHandle::new(handle_time, 0.0);
}

pub fn apply_linear_tangent(keyframes: &[EditableKeyframe], index: usize) -> (BezierHandle, BezierHandle) {
    let len = keyframes.len();
    if len < 2 || index >= len {
        return (BezierHandle::linear(), BezierHandle::linear());
    }

    let in_handle = if index > 0 {
        let prev = &keyframes[index - 1];
        let curr = &keyframes[index];
        let dt = curr.time - prev.time;
        let dv = curr.value - prev.value;
        let handle_time = dt / 3.0;
        let handle_value = dv / 3.0;
        BezierHandle::new(-handle_time, -handle_value)
    } else {
        BezierHandle::linear()
    };

    let out_handle = if index < len - 1 {
        let curr = &keyframes[index];
        let next = &keyframes[index + 1];
        let dt = next.time - curr.time;
        let dv = next.value - curr.value;
        let handle_time = dt / 3.0;
        let handle_value = dv / 3.0;
        BezierHandle::new(handle_time, handle_value)
    } else {
        BezierHandle::linear()
    };

    (in_handle, out_handle)
}

pub fn apply_auto_tangents_to_all(keyframes: &mut [EditableKeyframe]) {
    for i in 0..keyframes.len() {
        apply_auto_tangent(keyframes, i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::keyframe::InterpolationType;

    #[test]
    fn test_sample_bezier_linear_equivalent() {
        let result = sample_bezier(
            0.0,
            0.0,
            &BezierHandle::linear(),
            1.0,
            1.0,
            &BezierHandle::linear(),
            0.5,
        );
        assert!((result - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_sample_bezier_midpoint() {
        let out_handle = BezierHandle::new(0.33, 0.5);
        let in_handle = BezierHandle::new(-0.33, -0.5);

        let result = sample_bezier(
            0.0, 0.0, &out_handle, 1.0, 1.0, &in_handle, 0.5,
        );
        assert!(result > 0.3 && result < 0.7);
    }

    #[test]
    fn test_apply_flat_tangent() {
        let mut kf = EditableKeyframe::new(1, 1.0, 5.0);
        kf.interpolation = InterpolationType::Bezier;
        apply_flat_tangent(&mut kf, 0.5);

        assert!((kf.in_tangent.value_offset).abs() < 1e-6);
        assert!((kf.out_tangent.value_offset).abs() < 1e-6);
    }

    #[test]
    fn test_apply_auto_tangent_three_kf() {
        let mut keyframes = vec![
            EditableKeyframe::new(1, 0.0, 0.0),
            EditableKeyframe::new(2, 1.0, 2.0),
            EditableKeyframe::new(3, 2.0, 4.0),
        ];

        apply_auto_tangent(&mut keyframes, 1);

        let slope = (4.0 - 0.0) / (2.0 - 0.0);
        let expected_handle_time = (2.0 - 0.0) * 0.5 / 3.0;
        let expected_handle_value = slope * expected_handle_time;

        assert!(
            (keyframes[1].out_tangent.time_offset - expected_handle_time).abs() < 1e-4
        );
        assert!(
            (keyframes[1].out_tangent.value_offset - expected_handle_value).abs()
                < 1e-4
        );
    }

    #[test]
    fn test_stepped_sample() {
        let k0_value: f32 = 3.0;
        assert!((k0_value - 3.0).abs() < 1e-6);
    }
}
