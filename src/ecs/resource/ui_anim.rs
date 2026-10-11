use std::collections::HashMap;

use super::MotionPreference;

pub const DEFAULT_ANIM_DURATION_SECS: f32 = 0.15;

#[derive(Clone, Debug)]
struct AnimValue {
    current: f32,
    target: f32,
}

#[derive(Clone, Debug, Default)]
pub struct UiAnimState {
    values: HashMap<u32, AnimValue>,
}

#[inline]
pub fn advance_anim(current: f32, target: f32, dt: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return target;
    }
    current + (target - current) * (1.0 - (-dt * 4.6 / duration).exp())
}

impl UiAnimState {
    pub fn animate(&mut self, id: u32, target: f32, dt: f32, motion: MotionPreference) -> f32 {
        let entry = self.values.entry(id).or_insert_with(|| AnimValue {
            current: target,
            target,
        });

        if matches!(motion, MotionPreference::Reduced) {
            entry.current = target;
            entry.target = target;
            return target;
        }

        let prev = entry.current;
        entry.target = target;
        entry.current = advance_anim(prev, target, dt, DEFAULT_ANIM_DURATION_SECS);
        entry.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advance_anim_reaches_99_percent_within_duration() {
        let current = 0.0;
        let target = 1.0;
        let duration = DEFAULT_ANIM_DURATION_SECS;
        let value = advance_anim(current, target, duration, duration);
        assert!(
            (value - 0.99).abs() < 0.001,
            "advance_anim({current}, {target}, {duration}, {duration}) = {value} not close to 0.99",
        );
    }

    #[test]
    fn test_advance_anim_dt_zero_is_identity() {
        let current = 0.3;
        let target = 1.0;
        let value = advance_anim(current, target, 0.0, DEFAULT_ANIM_DURATION_SECS);
        assert!(
            (value - current).abs() < 1e-6,
            "advance_anim with dt=0 changed value: {current} -> {value}",
        );
    }

    #[test]
    fn test_advance_anim_duration_zero_is_instant() {
        let current = 0.0;
        let target = 1.0;
        let value = advance_anim(current, target, 0.01, 0.0);
        assert!(
            (value - target).abs() < 1e-6,
            "advance_anim with duration=0 did not reach target: {value}",
        );
    }

    #[test]
    fn test_animate_reduced_motion_is_instant() {
        let mut state = UiAnimState::default();
        let value = state.animate(1, 1.0, 0.016, MotionPreference::Reduced);
        assert!(
            (value - 1.0).abs() < 1e-6,
            "Reduced motion did not reach target immediately: {value}",
        );
    }

    #[test]
    fn test_animate_first_call_returns_target() {
        let mut state = UiAnimState::default();
        let value = state.animate(1, 0.5, 0.016, MotionPreference::Full);
        assert!(
            (value - 0.5).abs() < 1e-6,
            "First animate call did not return target: {value}",
        );
    }
}
