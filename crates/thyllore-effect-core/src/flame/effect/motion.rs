use crate::flame::*;

/// Azimuthal swirl-shear of the RTE medium (differential rotation).
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style)]
pub struct FlameSwirl {
    /// Medium swirl share: strain budget spent on azimuthal shear (0 = off; raising it thins the carve warp)
    #[persist(ui(min = 0.0, max = 1.5, group = "noise"))]
    pub gain: f32,
    /// How fast the swirl layers counter-rotate against the rise (time-only: costs no strain budget)
    #[persist(ui(min = 0.0, max = 4.0, group = "motion"))]
    pub speed: f32,
}

impl Default for FlameSwirl {
    fn default() -> Self {
        Self {
            gain: 0.0,
            speed: 1.0,
        }
    }
}

/// Node-frozen azimuthal rotation of the noise coordinate (V design).
#[derive(Clone, Copy, Debug, Default, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "motion")]
pub struct FlameTwist {
    /// Azimuthal twist of the noise pattern around the axis (radians at the tip; a rotation never folds, so any amplitude is structurally safe; 0 = off)
    #[persist(ui(min = 0.0, max = 8.0))]
    pub gain: f32,
    /// Twist rotation rate scale (0 = follow Swirl Speed; > 0 gives the twist its own rate so depth and speed tune independently)
    #[persist(ui(min = 0.0, max = 4.0))]
    pub speed: f32,
}

/// Animated horizontal displacement of the centerline.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "motion")]
pub struct FlameMeander {
    /// Horizontal meandering motion of the flame (0 = off)
    #[persist(ui(min = 0.0, max = 2.0))]
    pub amp: f32,
    /// Wavenumber multiplier of the meander modes: 1 = two long bends over the height, ~12 folds the column into a snake with ~4 bends (pillar reference)
    #[persist(ui(min = 0.2, max = 30.0, format = "%.1f"))]
    pub frequency: f32,
}

impl Default for FlameMeander {
    fn default() -> Self {
        Self {
            amp: 0.0,
            frequency: 1.0,
        }
    }
}

/// Sinusoidal displacement of the density boundary; amp 0 = off.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner)]
pub struct FlameBoundary {
    #[runtime]
    pub amp: f32,
    #[runtime]
    pub freq: f32,
    #[runtime]
    pub speed: f32,
    #[runtime]
    pub radius_ratio: f32,
}

impl Default for FlameBoundary {
    fn default() -> Self {
        Self {
            amp: 0.2,
            freq: 1.6,
            speed: 0.8,
            radius_ratio: 0.2,
        }
    }
}

/// Discrete branch element layer: a deterministic spawner of vortex transport
/// elements that wrap the RTE medium around a rising core; period 0 or gain 0 = off.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "branch")]
pub struct FlameBranch {
    /// Spawn period [s] of the branch elements (vortex lines that roll the medium into side tongues); raised to life/31 when the table is full, so shorter periods never shrink the tongues; 0 = off
    #[persist(ui(min = 0.0, max = 2.0))]
    pub period: f32,
    /// Lifetime [s] of one element (wind out fast, hold, burn out); the spawn period is raised to life/31 when the element table is full
    #[persist(ui(min = 0.1, max = 6.0))]
    pub life: f32,
    /// Rotation angle [rad] the core reaches at the end of winding; the medium flows along the arcs toward the tongue tip at a constant rate. Positive rolls trunk material down-out-up (cap curling inward), negative rolls it up-out-down (KH crest leaning upward). A compact rotation never folds; 0 = off
    #[persist(ui(min = -8.0, max = 8.0))]
    pub gain: f32,
    /// Vortex core radius as a ratio of the local trunk radius: small values shear the medium into thin spirals, near 1 the whole disc turns together and tongues keep the trunk's thickness
    #[persist(ui(min = 0.05, max = 3.0))]
    pub core_radius: f32,
    /// Lateral position of the core at spawn as a ratio of the local trunk radius: 0 on the axis tilts the whole slab, 1 on the shear layer rolls trunk material outward as a billow
    #[persist(ui(min = 0.0, max = 3.0))]
    pub core_offset: f32,
    /// Compact reach of one element at the end of its life as a ratio of the local trunk radius; nothing beyond it moves, so it bounds how far tongues can extend sideways
    #[persist(ui(min = 0.5, max = 8.0))]
    pub reach: f32,
    /// Scatter of azimuth, timing jitter, left/right alternation, element size, line tilt and window shift (0 = identical elements strictly alternating in one plane)
    #[persist(ui(min = 0.0, max = 1.0))]
    pub spread: f32,
    /// Center of the spawn height band (0 = base, 1 = top)
    #[persist(ui(min = 0.0, max = 1.0))]
    pub spawn_height: f32,
    /// Full width of the spawn height band; 1.0 with center 0.5 spawns elements over the whole trunk
    #[persist(ui(min = 0.0, max = 1.0))]
    pub spawn_range: f32,
    #[persist(owner = Frame)]
    pub seed: u32,
}

impl Default for FlameBranch {
    fn default() -> Self {
        Self {
            period: 0.0,
            life: 2.5,
            gain: 0.0,
            core_radius: 0.35,
            core_offset: 0.0,
            reach: 1.5,
            spread: 0.3,
            spawn_height: 0.35,
            spawn_range: 0.4,
            seed: 0,
        }
    }
}

/// Stateless write-through of the Vortex macro knob onto (twist gain, twist
/// speed); the two parameters stay the single source of truth.
pub fn vortex_macro_parameters(v: f32) -> (f32, f32) {
    let v = v.clamp(0.0, 1.0);
    (VORTEX_MACRO_MAX_GAIN * v, VORTEX_MACRO_MAX_SPEED * v)
}

/// The twist rate scale: twist speed owns the rate when positive, otherwise
/// the rate delegates to the swirl speed.
pub fn twist_rate_scale(twist: &FlameTwist, swirl: &FlameSwirl) -> f32 {
    if twist.speed > 0.0 {
        twist.speed
    } else {
        swirl.speed
    }
}

pub fn build_twist_field(twist: &FlameTwist, swirl: &FlameSwirl) -> FlameTwistField {
    let rate_scale = twist_rate_scale(twist, swirl);
    FlameTwistField {
        modes: std::array::from_fn(|j| FlameTwistMode {
            kappa: TWIST_MODE_KAPPA[j],
            omega: TWIST_MODE_SPIN[j] * rate_scale * twist_mode_phase_rate(TWIST_MODE_KAPPA[j]),
            phase: TWIST_MODE_PHASE[j],
            amp: TWIST_MODE_AMP[j],
        }),
        core_radius_sq: TWIST_CORE_RADIUS_SQ,
        _padding: [0.0; 3],
    }
}

pub fn build_meander_modes(meander: &FlameMeander, swirl: &FlameSwirl) -> [FlameMeanderMode; 2] {
    std::array::from_fn(|j| FlameMeanderMode {
        direction: MEANDER_MODE_DIRECTION[j],
        kappa: MEANDER_MODE_KAPPA[j] * meander.frequency.max(0.0),
        omega: swirl.speed * MEANDER_MODE_RATE_SCALE[j],
        phase: MEANDER_MODE_PHASE[j],
        _padding: [0.0; 3],
    })
}

pub fn build_boundary_params(boundary: &FlameBoundary) -> FlameBoundaryParams {
    FlameBoundaryParams {
        amp: boundary.amp,
        freq: boundary.freq,
        speed: boundary.speed,
        radius_ratio: boundary.radius_ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vortex_macro_is_monotone_and_off_at_zero() {
        assert_eq!(vortex_macro_parameters(0.0), (0.0, 0.0));
        assert_eq!(
            vortex_macro_parameters(1.0),
            (VORTEX_MACRO_MAX_GAIN, VORTEX_MACRO_MAX_SPEED)
        );
        assert_eq!(vortex_macro_parameters(2.0), vortex_macro_parameters(1.0));
        assert_eq!(vortex_macro_parameters(-1.0), vortex_macro_parameters(0.0));
        let mut previous = vortex_macro_parameters(0.0);
        for step in 1..=10 {
            let current = vortex_macro_parameters(step as f32 / 10.0);
            assert!(current.0 > previous.0 && current.1 > previous.1);
            previous = current;
        }
    }
}
