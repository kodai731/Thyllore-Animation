/// Converts the metre-unit spring parameters into the skeleton space the solver integrates in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringSpace {
    world_units_per_skeleton_unit: f32,
}

impl SpringSpace {
    pub const IDENTITY: Self = Self {
        world_units_per_skeleton_unit: 1.0,
    };

    pub fn from_world_scale(world_units_per_skeleton_unit: f32) -> Option<Self> {
        let is_valid =
            world_units_per_skeleton_unit.is_finite() && world_units_per_skeleton_unit > 0.0;
        is_valid.then_some(Self {
            world_units_per_skeleton_unit,
        })
    }

    pub fn world_scale(&self) -> f32 {
        self.world_units_per_skeleton_unit
    }

    pub fn skeleton_length(&self, world_length: f32) -> f32 {
        world_length / self.world_units_per_skeleton_unit
    }
}

impl Default for SpringSpace {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_length_divides_by_the_world_scale() {
        let space = SpringSpace::from_world_scale(0.01).expect("valid scale");
        assert!((space.skeleton_length(0.02) - 2.0).abs() < 1e-6);
        assert_eq!(SpringSpace::IDENTITY.skeleton_length(0.02), 0.02);
    }

    #[test]
    fn rejects_non_positive_or_non_finite_scales() {
        assert!(SpringSpace::from_world_scale(0.0).is_none());
        assert!(SpringSpace::from_world_scale(-1.0).is_none());
        assert!(SpringSpace::from_world_scale(f32::NAN).is_none());
    }
}
