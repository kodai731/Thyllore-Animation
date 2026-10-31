use std::sync::LazyLock;

use thyllore_math_core::lerp;
use thyllore_scene_core::{RootPath, ScalarParam, SceneFields, UiParam};

/// The strand lag that defines the hair material: a strand of this length trails its root moving at this speed by this angle.
pub const STRAND_LAG_REFERENCE_SPEED_M_PER_S: f32 = 0.5;
pub const STRAND_LAG_REFERENCE_LENGTH_M: f32 = 0.1;
pub const STRAND_LAG_REFERENCE_ANGLE_DEG: f32 = 20.0;

/// Bending relaxation `D` [m²/s]: a strand of length `L` returns at the rate `D / L²`, so its lag is `v·L / D`.
pub fn strand_bending_relaxation() -> f32 {
    STRAND_LAG_REFERENCE_SPEED_M_PER_S * STRAND_LAG_REFERENCE_LENGTH_M
        / STRAND_LAG_REFERENCE_ANGLE_DEG.to_radians().tan()
}

const VRM_DEFAULT_STIFFNESS_M_PER_S: f32 = 1.0;
const VRM_DEFAULT_GRAVITY_M_PER_S: f32 = 0.0;

/// How the joint forces are evaluated: the VRM 1.0 formula, or forces that vanish in the rest pose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpringForceModel {
    Vrm,
    #[default]
    RestRelative,
}

impl SpringForceModel {
    pub const ALL: [SpringForceModel; 2] = [SpringForceModel::Vrm, SpringForceModel::RestRelative];

    pub fn label(&self) -> &'static str {
        match self {
            SpringForceModel::Vrm => "VRM",
            SpringForceModel::RestRelative => "Rest Relative",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            SpringForceModel::Vrm => "vrm",
            SpringForceModel::RestRelative => "rest_relative",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|model| model.key() == key)
    }
}

/// VRM 1.0 joint forces: both are speeds in metres per second applied every step.
#[derive(Clone, Copy, Debug, PartialEq, SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct VrmJointForce {
    /// Speed towards the rest direction (VRM `stiffness`)
    #[persist(ui(min = 0.0, max = 4.0))]
    pub stiffness: f32,
    /// Speed along `gravity_dir` (VRM `gravityPower`)
    #[persist(ui(min = 0.0, max = 2.0))]
    pub gravity_power: f32,
}

impl Default for VrmJointForce {
    fn default() -> Self {
        Self {
            stiffness: VRM_DEFAULT_STIFFNESS_M_PER_S,
            gravity_power: VRM_DEFAULT_GRAVITY_M_PER_S,
        }
    }
}

/// Rest-relative joint forces: zero in the rest pose, so the authored shape is the equilibrium.
#[derive(Clone, Copy, Debug, PartialEq, SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct RestRelativeJointForce {
    /// Bending relaxation D [m²/s]; a strand of length L returns at D / L²
    #[persist(ui(min = 0.0, max = 1.0, format = "%.4f"))]
    pub bending_relaxation: f32,
    /// Multiple of Earth gravity that pulls the tail away from its rest sag when the parent tilts
    #[persist(ui(min = 0.0, max = 5.0))]
    pub gravity_scale: f32,
}

impl Default for RestRelativeJointForce {
    fn default() -> Self {
        Self {
            bending_relaxation: strand_bending_relaxation(),
            gravity_scale: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JointForce {
    Vrm(VrmJointForce),
    RestRelative(RestRelativeJointForce),
}

impl Default for JointForce {
    fn default() -> Self {
        Self::default_for(SpringForceModel::default())
    }
}

impl JointForce {
    pub fn default_for(model: SpringForceModel) -> Self {
        match model {
            SpringForceModel::Vrm => JointForce::Vrm(VrmJointForce::default()),
            SpringForceModel::RestRelative => {
                JointForce::RestRelative(RestRelativeJointForce::default())
            }
        }
    }

    pub fn model(&self) -> SpringForceModel {
        match self {
            JointForce::Vrm(_) => SpringForceModel::Vrm,
            JointForce::RestRelative(_) => SpringForceModel::RestRelative,
        }
    }

    /// The two tunable magnitudes of the model, restoring first, gravity second.
    pub fn magnitudes(&self) -> [f32; 2] {
        match self {
            JointForce::Vrm(force) => [force.stiffness, force.gravity_power],
            JointForce::RestRelative(force) => [force.bending_relaxation, force.gravity_scale],
        }
    }

    pub fn with_magnitudes(&self, magnitudes: [f32; 2]) -> Self {
        match self {
            JointForce::Vrm(_) => JointForce::Vrm(VrmJointForce {
                stiffness: magnitudes[0],
                gravity_power: magnitudes[1],
            }),
            JointForce::RestRelative(_) => JointForce::RestRelative(RestRelativeJointForce {
                bending_relaxation: magnitudes[0],
                gravity_scale: magnitudes[1],
            }),
        }
    }

    /// None across force models, whose magnitudes are different quantities.
    pub fn lerp(&self, to: &JointForce, weight: f32) -> Option<Self> {
        if self.model() != to.model() {
            return None;
        }
        let from = self.magnitudes();
        let target = to.magnitudes();
        Some(self.with_magnitudes([
            lerp(from[0], target[0], weight),
            lerp(from[1], target[1], weight),
        ]))
    }
}

/// The editing view of a joint force: both models' values side by side with the active one selected.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct JointForceForm {
    pub model: SpringForceModel,
    pub vrm: VrmJointForce,
    pub rest_relative: RestRelativeJointForce,
}

impl From<JointForce> for JointForceForm {
    fn from(force: JointForce) -> Self {
        let mut form = JointForceForm {
            model: force.model(),
            ..Default::default()
        };
        match force {
            JointForce::Vrm(vrm) => form.vrm = vrm,
            JointForce::RestRelative(rest_relative) => form.rest_relative = rest_relative,
        }
        form
    }
}

impl From<JointForceForm> for JointForce {
    fn from(form: JointForceForm) -> Self {
        match form.model {
            SpringForceModel::Vrm => JointForce::Vrm(form.vrm),
            SpringForceModel::RestRelative => JointForce::RestRelative(form.rest_relative),
        }
    }
}

pub static VRM_JOINT_FORCE_SCALAR_PARAMS: LazyLock<Vec<ScalarParam<VrmJointForce>>> =
    LazyLock::new(|| {
        let mut out = Vec::new();
        VrmJointForce::collect_scalars::<VrmJointForce, RootPath<VrmJointForce>>("", &mut out);
        out
    });

pub static VRM_JOINT_FORCE_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    VrmJointForce::collect_ui("", "", &mut out);
    out
});

pub static REST_RELATIVE_JOINT_FORCE_SCALAR_PARAMS: LazyLock<
    Vec<ScalarParam<RestRelativeJointForce>>,
> = LazyLock::new(|| {
    let mut out = Vec::new();
    RestRelativeJointForce::collect_scalars::<
        RestRelativeJointForce,
        RootPath<RestRelativeJointForce>,
    >("", &mut out);
    out
});

pub static REST_RELATIVE_JOINT_FORCE_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    RestRelativeJointForce::collect_ui("", "", &mut out);
    out
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_round_trips_the_active_model() {
        let force = JointForce::Vrm(VrmJointForce {
            stiffness: 2.0,
            gravity_power: 0.5,
        });
        let form = JointForceForm::from(force);
        assert_eq!(form.model, SpringForceModel::Vrm);
        assert_eq!(JointForce::from(form), force);
    }

    #[test]
    fn lerp_refuses_to_cross_models() {
        let from = JointForce::default_for(SpringForceModel::RestRelative);
        let to = JointForce::default_for(SpringForceModel::Vrm);
        assert_eq!(from.lerp(&to, 0.5), None);
    }

    #[test]
    fn lerp_interpolates_both_magnitudes() {
        let from = JointForce::Vrm(VrmJointForce {
            stiffness: 0.0,
            gravity_power: 0.0,
        });
        let to = JointForce::Vrm(VrmJointForce {
            stiffness: 2.0,
            gravity_power: 1.0,
        });
        assert_eq!(
            from.lerp(&to, 0.5).map(|force| force.magnitudes()),
            Some([1.0, 0.5])
        );
    }

    #[test]
    fn force_ui_params_follow_declaration_order() {
        let vrm: Vec<&str> = VRM_JOINT_FORCE_UI_PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(vrm, ["stiffness", "gravity_power"]);
        let rest: Vec<&str> = REST_RELATIVE_JOINT_FORCE_UI_PARAMS
            .iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(rest, ["bending_relaxation", "gravity_scale"]);
    }
}
