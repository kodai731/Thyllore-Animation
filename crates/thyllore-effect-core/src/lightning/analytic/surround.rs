use crate::lightning::{build_lightning_model_matrix, LightningEffect};
use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};

#[derive(Clone, Copy, Debug)]
pub struct DischargeLight {
    pub position: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct ImpactGlow {
    pub position: [f32; 3],
    pub strength: f32,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct DischargeEvent {
    pub burst_index: u32,
    pub start_time: f32,
    pub origin: [f32; 3],
    pub target: [f32; 3],
}

fn bolt_start_world(model: Matrix4<f32>) -> Vector3<f32> {
    let p = model * cgmath::Vector4::new(0.0, 0.0, 0.0, 1.0);
    Vector3::new(p.x / p.w, p.y / p.w, p.z / p.w)
}

fn bolt_end_world(effect: &LightningEffect, model: Matrix4<f32>) -> Vector3<f32> {
    let end = effect.shape.end_offset;
    let p = model * cgmath::Vector4::new(end[0], end[1], end[2], 1.0);
    Vector3::new(p.x / p.w, p.y / p.w, p.z / p.w)
}

pub fn discharge_light(effect: &LightningEffect, t: f32) -> Option<DischargeLight> {
    if effect.surround.light_gain <= 0.0 {
        return None;
    }

    let (burst_index, tau) = crate::lightning::analytic::active_burst(effect, t)?;
    let intensity = crate::lightning::analytic::stroke_intensity(effect, tau)
        * crate::lightning::analytic::flicker_factor(effect, effect.timing.seed, burst_index, tau);
    if intensity <= 0.0 {
        return None;
    }

    let model = build_lightning_model_matrix(effect);
    let start = bolt_start_world(model);
    let end = bolt_end_world(effect, model);
    let fraction = effect.surround.light_height_fraction;
    let position = start + (end - start) * fraction;

    Some(DischargeLight {
        position: [position.x, position.y, position.z],
        intensity: effect.surround.light_gain * intensity,
        color: effect.surround.light_color,
    })
}

pub fn impact_glow(effect: &LightningEffect, t: f32) -> Option<ImpactGlow> {
    if effect.surround.impact_radius <= 0.0 {
        return None;
    }

    let burst_count = if effect.timing.burst_count > 0 {
        effect.timing.burst_count
    } else {
        ((t - effect.timing.burst_start) / effect.timing.burst_interval).floor() as u32 + 1
    };

    let mut last_burst: Option<u32> = None;
    for i in (0..burst_count).rev() {
        let start_time = crate::lightning::analytic::burst_start_time(effect, i);
        if start_time <= t {
            last_burst = Some(i);
            break;
        }
    }

    let burst_index = last_burst?;
    let age = t - crate::lightning::analytic::burst_start_time(effect, burst_index);
    if age < 0.0 {
        return None;
    }

    let model = build_lightning_model_matrix(effect);
    let end = bolt_end_world(effect, model);
    let strength = (-age / effect.surround.impact_decay).exp();

    Some(ImpactGlow {
        position: [end.x, end.y, end.z],
        strength,
        radius: effect.surround.impact_radius,
    })
}

pub fn discharge_started_between(
    effect: &LightningEffect,
    previous_t: f32,
    t: f32,
) -> Option<DischargeEvent> {
    let burst_count = if effect.timing.burst_count > 0 {
        effect.timing.burst_count
    } else {
        ((t - effect.timing.burst_start) / effect.timing.burst_interval).floor() as u32 + 1
    };

    for i in 0..burst_count {
        let start_time = crate::lightning::analytic::burst_start_time(effect, i);
        if start_time > previous_t && start_time <= t {
            let model = build_lightning_model_matrix(effect);
            let origin = bolt_start_world(model);
            let target = bolt_end_world(effect, model);
            return Some(DischargeEvent {
                burst_index: i,
                start_time,
                origin: [origin.x, origin.y, origin.z],
                target: [target.x, target.y, target.z],
            });
        }
    }

    None
}

pub fn thunder_delay(effect: &LightningEffect, listener: [f32; 3], event: &DischargeEvent) -> f32 {
    let dx = event.target[0] - listener[0];
    let dy = event.target[1] - listener[1];
    let dz = event.target[2] - listener[2];
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    distance / effect.surround.thunder_speed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lightning::{LightningShape, LightningSource};

    fn make_effect() -> LightningEffect {
        LightningEffect {
            position: Vector3::new(0.0, 10.0, 0.0),
            shape: LightningShape {
                source: LightningSource::Point,
                end_offset: [0.0, -10.0, 0.0],
                ..LightningShape::default()
            },
            ..LightningEffect::default()
        }
    }

    #[test]
    fn test_discharge_light_none_when_gain_zero() {
        let effect = make_effect();
        let burst_time =
            crate::lightning::analytic::burst_start_time(&effect, 0) + effect.timing.attack_time;
        assert!(discharge_light(&effect, burst_time).is_none());
    }

    #[test]
    fn test_discharge_light_position_and_intensity() {
        let mut effect = make_effect();
        effect.surround.light_gain = 1.0;
        effect.surround.light_height_fraction = 0.5;

        let burst_time =
            crate::lightning::analytic::burst_start_time(&effect, 0) + effect.timing.attack_time;

        let light = discharge_light(&effect, burst_time);
        assert!(light.is_some(), "should have light during burst");
        let light = light.unwrap();

        let expected_mid = Vector3::new(0.0, 5.0, 0.0);
        assert!(
            (light.position[0] - expected_mid.x).abs() < 1e-6,
            "x: {} vs {}",
            light.position[0],
            expected_mid.x
        );
        assert!(
            (light.position[1] - expected_mid.y).abs() < 1e-6,
            "y: {} vs {}",
            light.position[1],
            expected_mid.y
        );
        assert!(
            (light.position[2] - expected_mid.z).abs() < 1e-6,
            "z: {} vs {}",
            light.position[2],
            expected_mid.z
        );
        assert!(light.intensity > 0.0, "intensity should be positive");

        let after = burst_time
            + effect.timing.attack_time
            + effect.timing.sustain_time
            + effect.timing.release_time
            + 1.0;
        assert!(
            discharge_light(&effect, after).is_none(),
            "should be None outside burst"
        );
    }

    #[test]
    fn test_impact_glow_strength_decay() {
        let mut effect = make_effect();
        effect.surround.impact_radius = 1.0;
        effect.surround.impact_decay = 0.5;

        let burst_start = crate::lightning::analytic::burst_start_time(&effect, 0);

        let glow = impact_glow(&effect, burst_start);
        assert!(glow.is_some(), "should have glow at burst start");
        let glow = glow.unwrap();
        assert!(
            (glow.strength - 1.0).abs() < 1e-6,
            "strength at t=0 should be ~1.0, got {}",
            glow.strength
        );

        let glow = impact_glow(&effect, burst_start + 0.5);
        assert!(glow.is_some());
        let glow = glow.unwrap();
        let expected = (-1.0_f32).exp();
        assert!(
            (glow.strength - expected).abs() < 1e-6,
            "strength at decay time should be ~{}, got {}",
            expected,
            glow.strength
        );
    }

    #[test]
    fn test_discharge_started_between() {
        let effect = make_effect();
        let burst_start = crate::lightning::analytic::burst_start_time(&effect, 0);

        let event = discharge_started_between(&effect, burst_start - 0.1, burst_start + 0.1);
        assert!(event.is_some(), "should detect burst crossing");
        let event = event.unwrap();
        assert_eq!(event.burst_index, 0);
        assert!((event.start_time - burst_start).abs() < 1e-6);

        let none = discharge_started_between(&effect, 0.0, burst_start - 0.01);
        assert!(none.is_none(), "should be None before first burst");
    }

    #[test]
    fn test_thunder_delay() {
        let effect = make_effect();
        let event = DischargeEvent {
            burst_index: 0,
            start_time: 0.0,
            origin: [0.0, 10.0, 0.0],
            target: [0.0, 0.0, 0.0],
        };
        let listener = [0.0, 0.0, 343.0];
        let delay = thunder_delay(&effect, listener, &event);
        assert!(
            (delay - 1.0).abs() < 1e-6,
            "delay for 343m at 343 m/s should be 1.0s, got {}",
            delay
        );
    }
}
