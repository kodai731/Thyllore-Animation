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

/// Puff train: the characteristic solution of the density advection equation
/// along the axis. Parcels of unburnt density leave the base every `period`,
/// rise at `rise`, widen by entrainment and burn out; gain 0 = off.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "puff")]
pub struct FlamePuff {
    /// How far the medium between puffs thins, in [0, 1]: the puff cores keep
    /// the full density, the gaps drop to 1 - gain; 0 = off.
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub gain: f32,
    /// Spawn period in seconds (puffing frequency 1 / period).
    #[persist(ui(min = 0.05, max = 3.0, format = "%.2f"))]
    pub period: f32,
    /// Rise velocity in local height units per second.
    #[persist(ui(min = 0.01, max = 3.0, format = "%.2f"))]
    pub rise: f32,
    /// Puff radius at spawn as a ratio of the base trunk radius.
    #[persist(ui(min = 0.05, max = 2.0, format = "%.2f"))]
    pub radius: f32,
    /// Radius growth per unit height (entrainment), in spawn radii.
    #[persist(ui(min = 0.0, max = 4.0, format = "%.2f"))]
    pub spread: f32,
    /// Height over which the puff density e-folds; 0 = no burnout.
    #[persist(ui(min = 0.0, max = 4.0, format = "%.2f"))]
    pub decay: f32,
    /// Vertical over lateral radius of a puff (isotropic units); below 1 = flat lumps.
    #[persist(ui(min = 0.1, max = 2.0, format = "%.2f"))]
    pub aspect: f32,
    /// Height at which puffs are spawned, in local height units [0, 1].
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub spawn_height: f32,
    /// Density of the static root puff; 0 = off.
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub root_gain: f32,
    /// Center height of the static root puff, in local height units [0, 1].
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub root_height: f32,
}

impl Default for FlamePuff {
    fn default() -> Self {
        Self {
            gain: 0.0,
            period: 0.5,
            rise: 0.3,
            radius: 0.6,
            spread: 0.5,
            decay: 0.8,
            aspect: 1.0,
            spawn_height: 0.0,
            root_gain: 0.0,
            root_height: 0.0,
        }
    }
}

/// Fluid motion of the column: a Lagrangian marker column (centre and width
/// per height) carried by a 2D vortex-pair flow with a gust, so the silhouette
/// lobes form, deform and sway instead of being advected rigidly; gain 0 = off.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "flow")]
pub struct FlameFlow {
    /// Scale of the flow's effect on the column (centre offset and width), 0 = off.
    #[persist(ui(min = 0.0, max = 2.0, format = "%.2f"))]
    pub gain: f32,
    /// Vortex pair spawn period in seconds.
    #[persist(ui(min = 0.1, max = 5.0, format = "%.2f"))]
    pub period: f32,
    /// Vortex pair rise speed in height units per second.
    #[persist(ui(min = 0.0, max = 2.0, format = "%.2f"))]
    pub rise: f32,
    /// Circulation of each vortex in base radii squared per second.
    #[persist(ui(min = 0.0, max = 5.0, format = "%.2f"))]
    pub strength: f32,
    /// Gaussian core radius of a vortex in base radii.
    #[persist(ui(min = 0.1, max = 2.0, format = "%.2f"))]
    pub core: f32,
    /// Gust velocity amplitude at the tip in base radii per second.
    #[persist(ui(min = 0.0, max = 3.0, format = "%.2f"))]
    pub gust: f32,
    /// Base gust frequency in Hz (three incommensurate components around it).
    #[persist(ui(min = 0.0, max = 3.0, format = "%.2f"))]
    pub gust_frequency: f32,
    /// Burst (whip) velocity amplitude in base radii per second; 0 = no bursts.
    #[persist(ui(min = 0.0, max = 5.0, format = "%.2f"))]
    pub burst: f32,
    /// Restoring rate of the markers toward the rest column, per second.
    #[persist(ui(min = 0.0, max = 5.0, format = "%.2f"))]
    pub damping: f32,
    /// Linear reduction of the damping with height: damping at the tip = damping * (1 - damping_slope); 0 = uniform (legacy).
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub damping_slope: f32,
    /// Upstream transport speed of the marker column in height units per second; 0 = off (bit-match).
    #[persist(ui(min = 0.0, max = 5.0, format = "%.2f"))]
    pub transport_speed: f32,
    /// Transport speed increase with height (multiplied by y/aspect); 0 = uniform transport.
    #[persist(ui(min = -5.0, max = 5.0, format = "%.2f"))]
    pub transport_accel: f32,
    /// Height01 up to which the gust injects lateral displacement at the root (1 at the foot, 0 at this height); 0 = tip-weighted y/aspect (legacy).
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub inject_height: f32,
}

impl Default for FlameFlow {
    fn default() -> Self {
        Self {
            gain: 0.0,
            period: 1.0,
            rise: 0.3,
            strength: 1.0,
            core: 0.6,
            gust: 0.3,
            gust_frequency: 0.4,
            burst: 0.0,
            damping: 0.5,
            damping_slope: 0.0,
            transport_speed: 0.0,
            transport_accel: 0.0,
            inject_height: 0.0,
        }
    }
}

/// Lobe train on the silhouette: one-sided bulges that spawn near the foot,
/// rise, swell and fade, riding the flow marker table (needs `flow.gain` > 0);
/// gain 0 = off. Mirrors the round puffs stacked along the reference column.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "lobe")]
pub struct FlameLobe {
    /// Peak lateral bulge of one lobe in base radii; 0 = off.
    #[persist(ui(min = 0.0, max = 3.0, format = "%.2f"))]
    pub gain: f32,
    /// Spawn period in seconds.
    #[persist(ui(min = 0.01, max = 5.0, format = "%.3f"))]
    pub period: f32,
    /// Lifetime of one lobe in seconds (swells over the first half, fades over the second).
    #[persist(ui(min = 0.01, max = 10.0, format = "%.3f"))]
    pub life: f32,
    /// Rise speed in height units per second.
    #[persist(ui(min = 0.0, max = 50.0, format = "%.2f"))]
    pub rise: f32,
    /// Vertical half-extent of one lobe in height units.
    #[persist(ui(min = 0.01, max = 0.5, format = "%.3f"))]
    pub size: f32,
    /// Centre of the spawn height band in height units.
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub spawn_height: f32,
    /// Width of the uniform spawn height band above `spawn_height`; 0 keeps the single band.
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub spawn_range: f32,
    /// Exponential rise rate in 1/s: the spawn height grows by exp(accel * age), so
    /// higher lobes rise faster (the reference column accelerates with height); 0 = off.
    #[persist(ui(min = 0.0, max = 20.0, format = "%.2f"))]
    pub accel: f32,
    /// Scatter of spawn time, height and size in [0, 1].
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub spread: f32,
    /// Centre shift per unit bulge in [0, 1]: 1 keeps the far side still (a
    /// one-sided tongue), 0 swells both sides (a symmetric puff).
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub shift: f32,
    /// 1 = inject each lobe once at spawn into the simulated marker column so the flow transport carries it and damping fades it (rise/accel/life unused); 0 = legacy overlay added after the simulation
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub transport: f32,
}

impl Default for FlameLobe {
    fn default() -> Self {
        Self {
            gain: 0.0,
            period: 0.5,
            life: 2.0,
            rise: 0.1,
            size: 0.08,
            spawn_height: 0.2,
            spawn_range: 0.0,
            accel: 0.0,
            spread: 0.5,
            shift: 1.0,
            transport: 0.0,
        }
    }
}

/// Coarse 2D buoyancy grid driving the column (trunk-local x in [-1, 1],
/// height in [0, GRID_HEIGHT_EXTENT]): fuel and heat injected at the root rise
/// under buoyancy, curl under vorticity confinement and burn out; enabled 0 = off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlameGrid {
    /// 1 = the grid replaces the marker column, puffs and lobes; 0 = off.
    pub enabled: f32,
    /// Height01 of the injection band at the root.
    pub inject_height: f32,
    /// Gaussian half width of the injection in x units (flame width = 1).
    pub inject_width: f32,
    /// Mean injection rate of fuel and heat per second.
    pub inject_rate: f32,
    /// Relative amplitude of the periodic injection pulse (puffing) in [0, 1].
    pub puff_amp: f32,
    /// Puffing frequency in Hz.
    pub puff_hz: f32,
    /// Upward acceleration per unit temperature (Nguyen 2002 eq. 14 alpha).
    pub buoyancy_heat: f32,
    /// Downward acceleration per unit fuel density (Nguyen 2002 eq. 14 beta).
    pub buoyancy_density: f32,
    /// Height01 below which the lateral gust accelerates the flow (1 at the foot, 0 there).
    pub gust_height: f32,
    /// Vorticity confinement strength (Nguyen 2002 eq. 15 epsilon).
    pub confinement: f32,
    /// Fuel burn-out rate per second (Mantaflow reaction speed).
    pub burn_rate: f32,
    /// Heat loss rate per second.
    pub cool_rate: f32,
    /// Gauss-Seidel iterations of the pressure projection.
    pub pressure_iters: f32,
}

impl Default for FlameGrid {
    fn default() -> Self {
        Self {
            enabled: 0.0,
            inject_height: 0.08,
            inject_width: 0.18,
            inject_rate: 1.0,
            puff_amp: 0.6,
            puff_hz: 12.5,
            buoyancy_heat: 2000.0,
            buoyancy_density: 0.0,
            gust_height: 0.25,
            confinement: 0.5,
            burn_rate: 12.0,
            cool_rate: 10.0,
            pressure_iters: 32.0,
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
