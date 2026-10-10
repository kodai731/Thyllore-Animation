use crate::editable::components::curve::PropertyCurve;
use crate::editable::components::keyframe::KeyframeId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TweenKind {
    BlendToNeighbor,
    Breakdown,
    Smooth,
}

pub fn compute_tween(
    curve: &PropertyCurve,
    selected: &[KeyframeId],
    kind: TweenKind,
    factor: f32,
) -> Vec<(KeyframeId, f32)> {
    let clamped = factor.clamp(-1.0, 1.0);
    let mut results = Vec::new();

    for &sel_id in selected {
        let kf_pos = curve.keyframes.iter().position(|k| k.id == sel_id);
        let Some(sel_pos) = kf_pos else {
            continue;
        };

        let prev_kf = curve.keyframes.get(sel_pos.wrapping_sub(1));
        let next_kf = curve.keyframes.get(sel_pos + 1);

        let new_value = match kind {
            TweenKind::BlendToNeighbor => blend_to_neighbor(curve, sel_pos, clamped),
            TweenKind::Breakdown => breakdown(prev_kf, next_kf, clamped),
            TweenKind::Smooth => smooth(curve, sel_pos, clamped),
        };

        if let Some(new_value) = new_value {
            results.push((sel_id, new_value));
        }
    }

    results
}

fn blend_to_neighbor(curve: &PropertyCurve, sel_pos: usize, factor: f32) -> Option<f32> {
    let current = curve.keyframes[sel_pos].value;
    let prev_kf = curve.keyframes.get(sel_pos.wrapping_sub(1));
    let next_kf = curve.keyframes.get(sel_pos + 1);

    if factor > 0.0 {
        let Some(next) = next_kf else {
            return None;
        };
        let target = current + (next.value - current) * factor;
        Some(target)
    } else if factor < 0.0 {
        let Some(prev) = prev_kf else {
            return None;
        };
        let abs_factor = (-factor).abs();
        let target = current + (prev.value - current) * abs_factor;
        Some(target)
    } else {
        None
    }
}

fn breakdown(
    prev: Option<&crate::editable::components::keyframe::EditableKeyframe>,
    next: Option<&crate::editable::components::keyframe::EditableKeyframe>,
    factor: f32,
) -> Option<f32> {
    let prev = prev?;
    let next = next?;
    let ratio = (factor + 1.0) * 0.5;
    let value = prev.value + (next.value - prev.value) * ratio;
    Some(value)
}

fn smooth(curve: &PropertyCurve, sel_pos: usize, factor: f32) -> Option<f32> {
    let effective_factor = if factor < 0.0 { 0.0 } else { factor };
    if effective_factor == 0.0 {
        return None;
    }

    let current = curve.keyframes[sel_pos].value;
    let prev_kf = curve.keyframes.get(sel_pos.wrapping_sub(1));
    let next_kf = curve.keyframes.get(sel_pos + 1);

    if prev_kf.is_none() && next_kf.is_none() {
        return None;
    }

    let mut weights: Vec<f32> = Vec::new();
    let mut values: Vec<f32> = Vec::new();

    if let Some(prev) = prev_kf {
        weights.push(0.25);
        values.push(prev.value);
    }

    weights.push(0.5);
    values.push(current);

    if let Some(next) = next_kf {
        weights.push(0.25);
        values.push(next.value);
    }

    let total_weight: f32 = weights.iter().sum();
    let smoothed: f32 = weights
        .iter()
        .zip(values.iter())
        .map(|(w, v)| w * v)
        .sum::<f32>()
        / total_weight;

    let new_value = current + (smoothed - current) * effective_factor;
    Some(new_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editable::components::curve::PropertyType;
    use crate::editable::components::keyframe::EditableKeyframe;

    fn make_curve(times_values: &[(f32, f32)]) -> PropertyCurve {
        let mut curve = PropertyCurve::new(0, PropertyType::TranslationX);
        for (i, (t, v)) in times_values.iter().enumerate() {
            curve
                .keyframes
                .push(EditableKeyframe::new(i as KeyframeId + 1, *t, *v));
        }
        curve
    }

    #[test]
    fn test_blend_to_neighbor() {
        let curve = make_curve(&[(0.0, 0.0), (1.0, 10.0), (2.0, 20.0)]);
        let mid_id = curve.keyframes[1].id;

        let result = compute_tween(&curve, &[mid_id], TweenKind::BlendToNeighbor, 0.5);
        assert_eq!(result.len(), 1);
        assert!((result[0].1 - 15.0).abs() < 1e-6);

        let result = compute_tween(&curve, &[mid_id], TweenKind::BlendToNeighbor, -1.0);
        assert!((result[0].1 - 0.0).abs() < 1e-6);

        let first_id = curve.keyframes[0].id;
        let result = compute_tween(&curve, &[first_id], TweenKind::BlendToNeighbor, -1.0);
        assert!(result.is_empty());

        let last_id = curve.keyframes[2].id;
        let result = compute_tween(&curve, &[last_id], TweenKind::BlendToNeighbor, 1.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_breakdown() {
        let curve = make_curve(&[(0.0, 0.0), (1.0, 10.0), (2.0, 20.0)]);
        let mid_id = curve.keyframes[1].id;

        let result = compute_tween(&curve, &[mid_id], TweenKind::Breakdown, 0.0);
        assert!((result[0].1 - 10.0).abs() < 1e-6);

        let result = compute_tween(&curve, &[mid_id], TweenKind::Breakdown, -1.0);
        assert!((result[0].1 - 0.0).abs() < 1e-6);

        let result = compute_tween(&curve, &[mid_id], TweenKind::Breakdown, 1.0);
        assert!((result[0].1 - 20.0).abs() < 1e-6);

        let single = make_curve(&[(0.0, 0.0)]);
        let result = compute_tween(
            &single,
            &[single.keyframes[0].id],
            TweenKind::Breakdown,
            0.0,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_smooth() {
        let curve = make_curve(&[(0.0, 0.0), (1.0, 20.0), (2.0, 0.0)]);
        let mid_id = curve.keyframes[1].id;

        let result = compute_tween(&curve, &[mid_id], TweenKind::Smooth, 1.0);
        assert!((result[0].1 - 10.0).abs() < 1e-6);

        let result = compute_tween(&curve, &[mid_id], TweenKind::Smooth, -1.0);
        assert!(result.is_empty());

        let single = make_curve(&[(0.0, 5.0)]);
        let result = compute_tween(&single, &[single.keyframes[0].id], TweenKind::Smooth, 1.0);
        assert!(result.is_empty());

        let curve = make_curve(&[(0.0, 0.0), (1.0, 10.0), (2.0, 20.0)]);
        let first_id = curve.keyframes[0].id;
        let result = compute_tween(&curve, &[first_id], TweenKind::Smooth, 1.0);
        let expected = (0.5 * 0.0 + 0.25 * 10.0) / (0.5 + 0.25);
        assert!((result[0].1 - expected).abs() < 1e-6);
    }

    #[test]
    fn test_factor_clamped() {
        let curve = make_curve(&[(0.0, 0.0), (1.0, 10.0), (2.0, 20.0)]);
        let mid_id = curve.keyframes[1].id;

        let result_high = compute_tween(&curve, &[mid_id], TweenKind::BlendToNeighbor, 2.0);
        let result_clamped = compute_tween(&curve, &[mid_id], TweenKind::BlendToNeighbor, 1.0);
        assert_eq!(result_high, result_clamped);

        let result_low = compute_tween(&curve, &[mid_id], TweenKind::BlendToNeighbor, -2.0);
        let result_clamped = compute_tween(&curve, &[mid_id], TweenKind::BlendToNeighbor, -1.0);
        assert_eq!(result_low, result_clamped);
    }
}
