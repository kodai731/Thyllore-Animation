use crate::vrchat::sidecar_spring::SidecarJointForce;

/// The two VRM 1.0 joint speeds, in metres per second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VrmJointMagnitudes {
    pub stiffness: f32,
    pub gravity_power: f32,
}

/// A `Vrm` force is carried over as is; a `RestRelative` force maps `D / L` to the stiffness and has no gravity at rest.
pub fn vrm_magnitudes(force: SidecarJointForce, strand_length_m: f32) -> VrmJointMagnitudes {
    match force {
        SidecarJointForce::Vrm {
            stiffness,
            gravity_power,
        } => VrmJointMagnitudes {
            stiffness,
            gravity_power,
        },
        SidecarJointForce::RestRelative {
            bending_relaxation,
            gravity_scale: _,
        } => VrmJointMagnitudes {
            stiffness: if strand_length_m > 0.0 {
                bending_relaxation / strand_length_m
            } else {
                0.0
            },
            gravity_power: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vrm_force_is_identity() {
        let magnitudes = vrm_magnitudes(
            SidecarJointForce::Vrm {
                stiffness: 0.8,
                gravity_power: 0.2,
            },
            0.3,
        );
        assert_eq!(
            magnitudes,
            VrmJointMagnitudes {
                stiffness: 0.8,
                gravity_power: 0.2
            }
        );
    }

    #[test]
    fn rest_relative_divides_relaxation_by_strand_length_without_gravity() {
        let magnitudes = vrm_magnitudes(
            SidecarJointForce::RestRelative {
                bending_relaxation: 0.06,
                gravity_scale: 1.0,
            },
            0.3,
        );
        assert!((magnitudes.stiffness - 0.2).abs() < 1e-6);
        assert_eq!(magnitudes.gravity_power, 0.0);
    }

    #[test]
    fn zero_length_strand_has_no_stiffness() {
        let magnitudes = vrm_magnitudes(
            SidecarJointForce::RestRelative {
                bending_relaxation: 0.06,
                gravity_scale: 1.0,
            },
            0.0,
        );
        assert_eq!(magnitudes.stiffness, 0.0);
    }
}
