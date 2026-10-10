use thyllore_math_core::mix;

pub const LIGHTNING_MAX_SEGMENTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub r0: f32,
    pub r1: f32,
    /// Relative envelope (branch dimming x stroke x flicker); the shader applies
    /// `core_intensity` / `rim_intensity` on top, so this must stay O(1).
    pub intensity: f32,
    pub arrival_start: f32,
    pub arrival_end: f32,
}

pub(super) fn push_segment(segments: &mut Vec<Segment>, seg: Segment) -> bool {
    if segments.len() >= LIGHTNING_MAX_SEGMENTS {
        return false;
    }
    segments.push(seg);
    true
}

pub(super) fn compute_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

pub(crate) fn segment_length(seg: &Segment) -> f32 {
    compute_distance(seg.a, seg.b)
}

pub(super) fn clip_to_growth_arrival(segments: &[Segment], front_distance: f32) -> Vec<Segment> {
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        if seg.arrival_end <= front_distance {
            out.push(*seg);
        } else if seg.arrival_start < front_distance {
            let frac = (front_distance - seg.arrival_start) / (seg.arrival_end - seg.arrival_start);
            let new_b = [
                mix(seg.a[0], seg.b[0], frac),
                mix(seg.a[1], seg.b[1], frac),
                mix(seg.a[2], seg.b[2], frac),
            ];
            let new_r1 = mix(seg.r0, seg.r1, frac);
            out.push(Segment {
                a: seg.a,
                b: new_b,
                r0: seg.r0,
                r1: new_r1,
                intensity: seg.intensity,
                arrival_start: seg.arrival_start,
                arrival_end: front_distance,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{difference, length};
    use super::*;

    #[test]
    fn test_clip_to_growth_arrival_spanning_segment_interpolation() {
        let segments = vec![
            Segment {
                a: [0.0, 0.0, 0.0],
                b: [0.0, -4.0, 0.0],
                r0: 1.0,
                r1: 0.5,
                intensity: 1.0,
                arrival_start: 0.0,
                arrival_end: 4.0,
            },
            Segment {
                a: [0.0, -4.0, 0.0],
                b: [0.0, -8.0, 0.0],
                r0: 0.5,
                r1: 0.3,
                intensity: 1.0,
                arrival_start: 4.0,
                arrival_end: 8.0,
            },
        ];

        let clipped = clip_to_growth_arrival(&segments, 2.0);

        assert_eq!(clipped.len(), 1, "the segment past the front is discarded");
        let seg = &clipped[0];
        assert_eq!(seg.a, [0.0, 0.0, 0.0]);
        assert!(
            length(difference(seg.b, [0.0, -2.0, 0.0])) < 1e-6,
            "b = {:?}",
            seg.b
        );
        assert!((seg.r0 - 1.0).abs() < 1e-6);
        assert!((seg.r1 - 0.75).abs() < 1e-6, "r1 = {}", seg.r1);
        assert_eq!(seg.arrival_end, 2.0);
    }

    #[test]
    fn test_clip_to_growth_arrival_all_before_front() {
        let segments = vec![Segment {
            a: [0.0, 0.0, 0.0],
            b: [0.0, -4.0, 0.0],
            r0: 1.0,
            r1: 0.5,
            intensity: 1.0,
            arrival_start: 0.0,
            arrival_end: 4.0,
        }];

        let clipped = clip_to_growth_arrival(&segments, 5.6);
        assert_eq!(clipped, segments);
    }

    #[test]
    fn test_clip_to_growth_arrival_all_after_front() {
        let segments = vec![Segment {
            a: [0.0, -4.0, 0.0],
            b: [0.0, -8.0, 0.0],
            r0: 0.5,
            r1: 0.3,
            intensity: 1.0,
            arrival_start: 4.0,
            arrival_end: 8.0,
        }];

        let clipped = clip_to_growth_arrival(&segments, 2.4);
        assert!(
            clipped.is_empty(),
            "all segments after front should be discarded"
        );
    }

    #[test]
    fn test_clip_to_growth_arrival_keeps_a_near_segment_after_a_far_one() {
        let far = Segment {
            a: [0.0, -4.0, 0.0],
            b: [0.0, -8.0, 0.0],
            r0: 0.5,
            r1: 0.5,
            intensity: 1.0,
            arrival_start: 4.0,
            arrival_end: 8.0,
        };
        let near = Segment {
            a: [0.0, -1.0, 0.0],
            b: [1.0, -1.0, 0.0],
            r0: 0.2,
            r1: 0.2,
            intensity: 0.5,
            arrival_start: 1.0,
            arrival_end: 2.0,
        };

        assert_eq!(clip_to_growth_arrival(&[far, near], 3.0), vec![near]);
    }
}
