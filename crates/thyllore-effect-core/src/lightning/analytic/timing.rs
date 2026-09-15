use crate::lightning::{hash_f32, HashChannel, LightningEffect};

pub fn envelope_duration(effect: &LightningEffect) -> f32 {
    effect.attack_time + effect.sustain_time + effect.release_time
}

pub fn burst_start_time(effect: &LightningEffect, burst_index: u32) -> f32 {
    let jitter = if effect.burst_jitter > 0.0 {
        let noise = hash_f32(&[effect.seed, burst_index, HashChannel::Jitter.into()]);
        effect.burst_jitter * (2.0 * noise - 1.0)
    } else {
        0.0
    };

    effect.burst_start + burst_index as f32 * effect.burst_interval + jitter
}

/// Newest burst covering `t`, with its local time; `None` while no burst is alive.
pub fn active_burst(effect: &LightningEffect, t: f32) -> Option<(u32, f32)> {
    let duration = envelope_duration(effect);
    if duration <= 0.0 {
        return None;
    }

    let nearest_index = if effect.burst_interval > 0.0 {
        ((t - effect.burst_start) / effect.burst_interval).floor() as i64
    } else {
        0
    };

    for candidate in [nearest_index + 1, nearest_index, nearest_index - 1] {
        if candidate < 0 || (effect.burst_count > 0 && candidate >= effect.burst_count as i64) {
            continue;
        }

        let burst_index = candidate as u32;
        let tau = t - burst_start_time(effect, burst_index);
        if tau >= 0.0 && tau < duration {
            return Some((burst_index, tau));
        }
    }

    None
}

pub fn envelope(effect: &LightningEffect, tau: f32) -> f32 {
    if tau < 0.0 {
        return 0.0;
    }

    if tau < effect.attack_time {
        return tau / effect.attack_time;
    }

    let sustain_end = effect.attack_time + effect.sustain_time;
    if tau < sustain_end {
        return 1.0;
    }

    let release_end = sustain_end + effect.release_time;
    if tau < release_end {
        return (release_end - tau) / effect.release_time;
    }

    0.0
}

pub fn stroke_intensity(effect: &LightningEffect, tau: f32) -> f32 {
    let burst_envelope = envelope(effect, tau);
    if effect.stroke_count <= 1 || burst_envelope <= 0.0 {
        return burst_envelope;
    }

    let mut strongest_stroke = 0.0f32;
    let mut decay = 1.0f32;
    for stroke in 0..effect.stroke_count {
        let stroke_tau = tau - stroke as f32 * effect.stroke_interval;
        strongest_stroke = strongest_stroke.max(decay * envelope(effect, stroke_tau));
        decay *= effect.stroke_decay;
    }

    burst_envelope * strongest_stroke
}

pub fn flicker_factor(effect: &LightningEffect, seed: u32, burst_index: u32, tau: f32) -> f32 {
    if effect.flicker_period <= 0.0 {
        return 1.0;
    }

    let tick = (tau / effect.flicker_period).floor().max(0.0) as u32;
    let noise = hash_f32(&[seed, burst_index, tick, HashChannel::Flicker.into()]);
    1.0 - effect.flicker_amplitude * noise
}

pub fn reseed_index(effect: &LightningEffect, tau: f32) -> u32 {
    if effect.reseed_period <= 0.0 {
        return 0;
    }

    (tau / effect.reseed_period).floor().max(0.0) as u32
}

pub fn charge_alive_strikes(effect: &LightningEffect, tau: f32) -> u32 {
    if effect.charge_ramp <= 0.0 {
        return effect.strikes_per_burst;
    }

    let charged = (tau / effect.charge_ramp).clamp(0.0, 1.0);
    (effect.strikes_per_burst as f32 * charged) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_effect() -> LightningEffect {
        LightningEffect {
            seed: 17,
            burst_start: 0.5,
            burst_interval: 2.0,
            burst_jitter: 0.0,
            burst_count: 4,
            attack_time: 0.01,
            sustain_time: 0.04,
            release_time: 0.12,
            stroke_count: 1,
            stroke_interval: 0.05,
            stroke_decay: 0.6,
            ..LightningEffect::default()
        }
    }

    #[test]
    fn test_envelope_is_continuous_at_every_segment_boundary() {
        let effect = test_effect();
        let epsilon = 1.0e-5;

        assert_eq!(envelope(&effect, 0.0), 0.0);
        assert!((envelope(&effect, effect.attack_time - epsilon) - 1.0).abs() < 1.0e-2);
        assert_eq!(envelope(&effect, effect.attack_time), 1.0);

        let sustain_end = effect.attack_time + effect.sustain_time;
        assert_eq!(envelope(&effect, sustain_end - epsilon), 1.0);
        assert!((envelope(&effect, sustain_end + epsilon) - 1.0).abs() < 1.0e-2);

        let release_end = sustain_end + effect.release_time;
        assert!(envelope(&effect, release_end - epsilon) < 1.0e-2);
        assert_eq!(envelope(&effect, release_end), 0.0);
        assert_eq!(envelope(&effect, release_end + 1.0), 0.0);
    }

    #[test]
    fn test_zero_attack_reaches_full_intensity_immediately() {
        let mut effect = test_effect();
        effect.attack_time = 0.0;
        assert_eq!(envelope(&effect, 0.0), 1.0);
    }

    #[test]
    fn test_bursts_are_evenly_spaced_without_jitter() {
        let effect = test_effect();
        for burst_index in 0..effect.burst_count {
            let expected = effect.burst_start + burst_index as f32 * effect.burst_interval;
            assert!((burst_start_time(&effect, burst_index) - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn test_jitter_shifts_bursts_within_its_bound() {
        let mut effect = test_effect();
        effect.burst_jitter = 0.2;
        for burst_index in 0..effect.burst_count {
            let unjittered = effect.burst_start + burst_index as f32 * effect.burst_interval;
            let shift = burst_start_time(&effect, burst_index) - unjittered;
            assert!(shift.abs() <= effect.burst_jitter, "{shift}");
        }
    }

    #[test]
    fn test_active_burst_selects_the_burst_that_owns_the_time() {
        let effect = test_effect();
        assert_eq!(active_burst(&effect, 0.0), None);

        let (burst_index, tau) = active_burst(&effect, 0.52).expect("first burst is alive");
        assert_eq!(burst_index, 0);
        assert!((tau - 0.02).abs() < 1.0e-6);

        let (burst_index, tau) = active_burst(&effect, 2.5).expect("second burst is alive");
        assert_eq!(burst_index, 1);
        assert!(tau.abs() < 1.0e-6);

        assert_eq!(active_burst(&effect, 1.5), None);
    }

    #[test]
    fn test_bursts_stop_after_the_count_and_repeat_forever_when_zero() {
        let mut effect = test_effect();
        let after_last = effect.burst_start + effect.burst_count as f32 * effect.burst_interval;
        assert_eq!(active_burst(&effect, after_last), None);

        effect.burst_count = 0;
        assert_eq!(active_burst(&effect, after_last).map(|(k, _)| k), Some(4));
        assert_eq!(
            active_burst(&effect, after_last + 200.0).map(|(k, _)| k),
            Some(104)
        );
    }

    #[test]
    fn test_strokes_decay_monotonically() {
        let mut effect = test_effect();
        effect.stroke_count = 4;

        let mut previous = f32::INFINITY;
        for stroke in 0..effect.stroke_count {
            let tau = effect.attack_time + stroke as f32 * effect.stroke_interval;
            let intensity = stroke_intensity(&effect, tau);
            assert!(intensity < previous, "stroke {stroke} gave {intensity}");
            previous = intensity;
        }
    }

    #[test]
    fn test_single_stroke_is_the_bare_envelope() {
        let effect = test_effect();
        for step in 0..32 {
            let tau = step as f32 * 0.01;
            assert_eq!(stroke_intensity(&effect, tau), envelope(&effect, tau));
        }
    }

    #[test]
    fn test_flicker_stays_within_its_amplitude_and_holds_over_a_period() {
        let mut effect = test_effect();
        effect.flicker_amplitude = 0.25;
        effect.flicker_period = 0.03;

        let inside_first_period = flicker_factor(&effect, 17, 0, 0.001);
        assert_eq!(flicker_factor(&effect, 17, 0, 0.029), inside_first_period);
        assert_ne!(flicker_factor(&effect, 17, 0, 0.031), inside_first_period);

        for step in 0..64 {
            let factor = flicker_factor(&effect, 17, 0, step as f32 * 0.005);
            assert!(
                (1.0 - effect.flicker_amplitude..=1.0).contains(&factor),
                "{factor}"
            );
        }

        effect.flicker_period = 0.0;
        assert_eq!(flicker_factor(&effect, 17, 0, 0.05), 1.0);
    }

    #[test]
    fn test_reseed_index_advances_once_per_period() {
        let mut effect = test_effect();
        effect.reseed_period = 0.25;
        assert_eq!(reseed_index(&effect, 0.0), 0);
        assert_eq!(reseed_index(&effect, 0.24), 0);
        assert_eq!(reseed_index(&effect, 0.26), 1);
        assert_eq!(reseed_index(&effect, 1.1), 4);

        effect.reseed_period = 0.0;
        assert_eq!(reseed_index(&effect, 10.0), 0);
    }

    #[test]
    fn test_charge_ramp_reveals_strikes_progressively() {
        let mut effect = test_effect();
        effect.strikes_per_burst = 4;
        effect.charge_ramp = 0.0;
        assert_eq!(charge_alive_strikes(&effect, 0.0), 4);

        effect.charge_ramp = 0.4;
        assert_eq!(charge_alive_strikes(&effect, 0.0), 0);
        assert_eq!(charge_alive_strikes(&effect, 0.2), 2);
        assert_eq!(charge_alive_strikes(&effect, 0.4), 4);
        assert_eq!(charge_alive_strikes(&effect, 10.0), 4);
    }

    #[test]
    fn test_timing_is_bit_reproducible() {
        let effect = {
            let mut effect = test_effect();
            effect.burst_jitter = 0.2;
            effect
        };

        for burst_index in 0..effect.burst_count {
            let first = burst_start_time(&effect, burst_index);
            assert_eq!(
                first.to_bits(),
                burst_start_time(&effect, burst_index).to_bits()
            );
        }

        let flicker = flicker_factor(&effect, effect.seed, 2, 0.07);
        assert_eq!(
            flicker.to_bits(),
            flicker_factor(&effect, effect.seed, 2, 0.07).to_bits()
        );
        assert_eq!(
            stroke_intensity(&effect, 0.03).to_bits(),
            stroke_intensity(&effect, 0.03).to_bits()
        );
    }
}
