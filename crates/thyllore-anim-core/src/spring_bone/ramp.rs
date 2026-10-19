use crate::spring_bone::{SpringChain, SpringJointParam};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RampShape {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
}

impl RampShape {
    pub const ALL: [RampShape; 3] = [RampShape::Linear, RampShape::EaseIn, RampShape::EaseOut];

    pub fn label(&self) -> &'static str {
        match self {
            RampShape::Linear => "Linear",
            RampShape::EaseIn => "Ease In",
            RampShape::EaseOut => "Ease Out",
        }
    }

    pub fn weight(&self, t: f32) -> f32 {
        match self {
            RampShape::Linear => t,
            RampShape::EaseIn => t * t,
            RampShape::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ChainRamp {
    pub root: SpringJointParam,
    pub tip: SpringJointParam,
    pub shape: RampShape,
}

fn lerp(from: f32, to: f32, weight: f32) -> f32 {
    from + (to - from) * weight
}

pub fn apply_ramp(chain: &SpringChain, ramp: &ChainRamp) -> SpringChain {
    let n = chain.joints.len();
    let mut out = chain.clone();

    for (i, joint) in out.joints.iter_mut().enumerate() {
        let t = if n <= 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let w = ramp.shape.weight(t);

        joint.stiffness = lerp(ramp.root.stiffness, ramp.tip.stiffness, w);
        joint.drag_force = lerp(ramp.root.drag_force, ramp.tip.drag_force, w);
        joint.gravity_power = lerp(ramp.root.gravity_power, ramp.tip.gravity_power, w);
        joint.hit_radius = lerp(ramp.root.hit_radius, ramp.tip.hit_radius, w);
        joint.gravity_dir = ramp.root.gravity_dir;
    }

    out
}

#[cfg(test)]
mod tests {
    use cgmath::Vector3;

    use super::*;

    fn make_chain(bone_ids: &[u32]) -> SpringChain {
        SpringChain {
            joints: bone_ids
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    stiffness: 1.0,
                    drag_force: 0.5,
                    gravity_power: 1.0,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.1,
                })
                .collect(),
            ..Default::default()
        }
    }

    fn ramp(root_stiffness: f32, tip_stiffness: f32, shape: RampShape) -> ChainRamp {
        ChainRamp {
            root: SpringJointParam {
                stiffness: root_stiffness,
                ..Default::default()
            },
            tip: SpringJointParam {
                stiffness: tip_stiffness,
                ..Default::default()
            },
            shape,
        }
    }

    #[test]
    fn single_joint_is_root_value() {
        let chain = make_chain(&[0]);
        let ramp = ramp(0.8, 0.2, RampShape::Linear);
        let result = apply_ramp(&chain, &ramp);

        assert_eq!(result.joints.len(), 1);
        assert!((result.joints[0].stiffness - 0.8).abs() < 1e-6);
    }

    #[test]
    fn four_joints_linear_interpolation() {
        let chain = make_chain(&[0, 1, 2, 3]);
        let ramp = ramp(0.8, 0.2, RampShape::Linear);
        let result = apply_ramp(&chain, &ramp);

        assert_eq!(result.joints.len(), 4);
        assert!((result.joints[0].stiffness - 0.8).abs() < 1e-6);
        assert!((result.joints[1].stiffness - 0.6).abs() < 1e-6);
        assert!((result.joints[2].stiffness - 0.4).abs() < 1e-6);
        assert!((result.joints[3].stiffness - 0.2).abs() < 1e-6);
    }

    #[test]
    fn four_joints_ease_in_second_joint_closer_to_root() {
        let chain = make_chain(&[0, 1, 2, 3]);
        let ramp_linear = ramp(0.8, 0.2, RampShape::Linear);
        let ramp_ease_in = ramp(0.8, 0.2, RampShape::EaseIn);

        let result_linear = apply_ramp(&chain, &ramp_linear);
        let result_ease_in = apply_ramp(&chain, &ramp_ease_in);
        assert!(result_ease_in.joints[1].stiffness > result_linear.joints[1].stiffness);
    }

    #[test]
    fn gravity_dir_is_root_for_all_joints() {
        let chain = make_chain(&[0, 1, 2]);
        let ramp = ChainRamp {
            root: SpringJointParam {
                gravity_dir: Vector3::new(1.0, 0.0, 0.0),
                ..Default::default()
            },
            tip: SpringJointParam {
                gravity_dir: Vector3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
            shape: RampShape::Linear,
        };
        let result = apply_ramp(&chain, &ramp);

        for joint in &result.joints {
            assert_eq!(joint.gravity_dir, Vector3::new(1.0, 0.0, 0.0));
        }
    }

    #[test]
    fn bone_ids_preserved_from_input_chain() {
        let chain = make_chain(&[5, 12, 7]);
        let ramp = ramp(0.8, 0.2, RampShape::Linear);
        let result = apply_ramp(&chain, &ramp);

        assert_eq!(result.joints[0].bone_id, 5);
        assert_eq!(result.joints[1].bone_id, 12);
        assert_eq!(result.joints[2].bone_id, 7);
    }
}
