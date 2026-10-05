#![cfg(test)]
#[derive(Clone, Copy, Debug)]
pub enum BoneAxisRule {
    Along([f64; 3]),
    WorldAligned,
    Random { seed: u64 },
}

#[derive(Clone, Copy, Debug)]
pub enum ContainerNode {
    None,
    Armature { rotation_x_degrees: f64 },
    BipedRoot { rotation_up_degrees: f64 },
    RootBone,
}

#[derive(Clone, Copy, Debug)]
pub struct RigConvention {
    pub id: &'static str,
    pub character_to_file: &'static [([f64; 3], f64)],
    pub up_axis: i32,
    pub unit_scale_factor: f64,
    pub bone_axis: BoneAxisRule,
    pub roll_degrees: f64,
    pub uses_pre_rotation: bool,
    pub body_scale: f64,
    pub arm_drop_degrees: f64,
    pub container: ContainerNode,
    pub leaf_end_bones: bool,
}

pub fn rig_convention(id: &str) -> RigConvention {
    match id {
        "blender" => RigConvention {
            id: "blender",
            character_to_file: &[],
            up_axis: 1,
            unit_scale_factor: 100.0,
            bone_axis: BoneAxisRule::Along([0.0, 1.0, 0.0]),
            roll_degrees: 0.0,
            uses_pre_rotation: false,
            body_scale: 1.0,
            arm_drop_degrees: 0.0,
            container: ContainerNode::Armature {
                rotation_x_degrees: -90.0,
            },
            leaf_end_bones: true,
        },
        "maya" => RigConvention {
            id: "maya",
            character_to_file: &[],
            up_axis: 1,
            unit_scale_factor: 1.0,
            bone_axis: BoneAxisRule::Along([1.0, 0.0, 0.0]),
            roll_degrees: 0.0,
            uses_pre_rotation: true,
            body_scale: 1.0,
            arm_drop_degrees: 0.0,
            container: ContainerNode::None,
            leaf_end_bones: false,
        },
        "max_biped" => RigConvention {
            id: "max_biped",
            character_to_file: &[([1.0, 0.0, 0.0], 90.0)],
            up_axis: 2,
            unit_scale_factor: 2.54,
            bone_axis: BoneAxisRule::Along([1.0, 0.0, 0.0]),
            roll_degrees: 90.0,
            uses_pre_rotation: false,
            body_scale: 1.0,
            arm_drop_degrees: 0.0,
            container: ContainerNode::BipedRoot {
                rotation_up_degrees: 90.0,
            },
            leaf_end_bones: false,
        },
        "mixamo" => RigConvention {
            id: "mixamo",
            character_to_file: &[],
            up_axis: 1,
            unit_scale_factor: 1.0,
            bone_axis: BoneAxisRule::Along([0.0, 1.0, 0.0]),
            roll_degrees: 0.0,
            uses_pre_rotation: false,
            body_scale: 1.0,
            arm_drop_degrees: 0.0,
            container: ContainerNode::None,
            leaf_end_bones: false,
        },
        "unreal" => RigConvention {
            id: "unreal",
            character_to_file: &[([1.0, 0.0, 0.0], 90.0), ([0.0, 0.0, 1.0], 180.0)],
            up_axis: 2,
            unit_scale_factor: 1.0,
            bone_axis: BoneAxisRule::Along([1.0, 0.0, 0.0]),
            roll_degrees: 0.0,
            uses_pre_rotation: false,
            body_scale: 1.0,
            arm_drop_degrees: 0.0,
            container: ContainerNode::RootBone,
            leaf_end_bones: false,
        },
        "vrm_normalized" => RigConvention {
            id: "vrm_normalized",
            character_to_file: &[],
            up_axis: 1,
            unit_scale_factor: 100.0,
            bone_axis: BoneAxisRule::WorldAligned,
            roll_degrees: 0.0,
            uses_pre_rotation: false,
            body_scale: 1.0,
            arm_drop_degrees: 0.0,
            container: ContainerNode::None,
            leaf_end_bones: false,
        },
        "blender_apose" => RigConvention {
            id: "blender_apose",
            arm_drop_degrees: 45.0,
            ..rig_convention("blender")
        },
        "adversarial" => RigConvention {
            id: "adversarial",
            character_to_file: &[([0.0, 0.0, 1.0], 90.0), ([-1.0, 0.0, 0.0], 137.0)],
            up_axis: 1,
            unit_scale_factor: 1.0,
            bone_axis: BoneAxisRule::Random { seed: 7 },
            roll_degrees: 0.0,
            uses_pre_rotation: false,
            body_scale: 0.5,
            arm_drop_degrees: 0.0,
            container: ContainerNode::None,
            leaf_end_bones: false,
        },
        unknown => panic!("unknown rig convention id: {:?}", unknown),
    }
}
