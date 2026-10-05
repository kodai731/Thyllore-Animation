use crate::expression::components::side::Side;
use crate::humanoid::components::role::HumanoidRole;

#[derive(Clone, Copy, Debug)]
pub enum ExtraParent {
    Role(HumanoidRole),
    Row(usize),
}

#[derive(Clone, Debug)]
pub struct ExtraBone {
    pub name: String,
    pub parent: ExtraParent,
    pub position: [f64; 3],
    pub tip: [f64; 3],
    pub rest_euler_degrees: [f64; 3],
}

#[rustfmt::skip]
const EXTRA_BONE_TABLE: &[(&str, Option<Side>, ExtraParent, [f64; 3], [f64; 3], [f64; 3])] = &[
    ("Neck_Scarf",   None,       ExtraParent::Role(HumanoidRole::Neck),         [0.0, 1.43, 0.0],  [0.0, -0.03, 0.07], [0.0, 0.0, 0.0]),
    ("Scarf_Front_1",None,       ExtraParent::Row(0),                            [0.0, 1.40, 0.07], [0.0, -0.10, 0.01], [10.0, 0.0, 0.0]),
    ("Scarf_Front_2",None,       ExtraParent::Row(1),                            [0.0, 1.30, 0.08], [0.0, -0.10, 0.0],  [10.0, 0.0, 0.0]),
    ("Scarf_Front_3",None,       ExtraParent::Row(2),                            [0.0, 1.20, 0.08], [0.0, -0.10, 0.0],  [10.0, 0.0, 0.0]),
    ("Scarf_Back_1", None,       ExtraParent::Row(0),                            [0.0, 1.40, -0.07],[0.0, -0.10, -0.02],[-10.0, 0.0, 0.0]),
    ("Scarf_Back_2", None,       ExtraParent::Row(4),                            [0.0, 1.30, -0.09],[0.0, -0.10, -0.02],[-10.0, 0.0, 0.0]),
    ("Scarf_Back_3", None,       ExtraParent::Row(5),                            [0.0, 1.20, -0.11],[0.0, -0.10, 0.0],  [-10.0, 0.0, 0.0]),
    ("Skirt_Front_1",None,       ExtraParent::Role(HumanoidRole::Hips),         [0.0, 0.90, 0.10], [0.0, -0.15, 0.04], [-15.0, 0.0, 0.0]),
    ("Skirt_Front_2",None,       ExtraParent::Row(7),                            [0.0, 0.75, 0.14], [0.0, -0.15, 0.04], [-15.0, 0.0, 0.0]),
    ("Skirt_Back_1", None,       ExtraParent::Role(HumanoidRole::Hips),         [0.0, 0.90, -0.10],[0.0, -0.15, -0.04],[15.0, 0.0, 0.0]),
    ("Skirt_Back_2", None,       ExtraParent::Row(9),                            [0.0, 0.75, -0.14],[0.0, -0.15, -0.04],[15.0, 0.0, 0.0]),
    ("Hair_Back_1",  None,       ExtraParent::Role(HumanoidRole::Head),         [0.0, 1.68, -0.08],[0.0, -0.10, -0.04],[20.0, 0.0, 0.0]),
    ("Hair_Back_2",  None,       ExtraParent::Row(11),                           [0.0, 1.58, -0.12],[0.0, -0.10, -0.01],[20.0, 0.0, 0.0]),
    ("Hair_Back_3",  None,       ExtraParent::Row(12),                           [0.0, 1.48, -0.13],[0.0, -0.10, 0.0],  [20.0, 0.0, 0.0]),
    ("Head_Hat",     None,       ExtraParent::Role(HumanoidRole::Head),         [0.0, 1.76, 0.0],  [0.0, 0.08, 0.0],   [0.0, 0.0, 0.0]),
    ("Tail_1",       None,       ExtraParent::Role(HumanoidRole::Hips),         [0.0, 0.92, -0.10],[0.0, -0.07, -0.12],[30.0, 0.0, 0.0]),
    ("Tail_2",       None,       ExtraParent::Row(15),                           [0.0, 0.85, -0.22],[0.0, -0.07, -0.10],[30.0, 0.0, 0.0]),
    ("Tail_3",       None,       ExtraParent::Row(16),                           [0.0, 0.78, -0.32],[0.0, -0.05, -0.10],[30.0, 0.0, 0.0]),
    ("Skirt_FrontSide_1", Some(Side::Left), ExtraParent::Role(HumanoidRole::Hips), [0.08, 0.90, 0.08], [0.03, -0.15, 0.03], [-10.0, 0.0, 10.0]),
    ("Skirt_FrontSide_2", Some(Side::Left), ExtraParent::Row(18),                  [0.11, 0.75, 0.11], [0.03, -0.15, 0.03], [-10.0, 0.0, 10.0]),
    ("Skirt_Side_1", Some(Side::Left), ExtraParent::Role(HumanoidRole::Hips), [0.12, 0.90, 0.0], [0.04, -0.15, 0.0], [0.0, 0.0, 15.0]),
    ("Skirt_Side_2", Some(Side::Left), ExtraParent::Row(20),                    [0.16, 0.75, 0.0], [0.04, -0.15, 0.0], [0.0, 0.0, 15.0]),
    ("Skirt_BackSide_1", Some(Side::Left), ExtraParent::Role(HumanoidRole::Hips), [0.08, 0.90, -0.08], [0.03, -0.15, -0.03], [10.0, 0.0, 10.0]),
    ("Skirt_BackSide_2", Some(Side::Left), ExtraParent::Row(22),                 [0.11, 0.75, -0.11], [0.03, -0.15, -0.03], [10.0, 0.0, 10.0]),
    ("UpperArm_Sleeve", Some(Side::Left), ExtraParent::Role(HumanoidRole::LeftUpperArm), [0.25, 1.40, 0.0], [0.08, 0.0, 0.0], [0.0, 0.0, 5.0]),
    ("LowerArm_Cuff",   Some(Side::Left), ExtraParent::Role(HumanoidRole::LeftLowerArm), [0.62, 1.40, 0.0], [0.05, 0.0, 0.0], [0.0, 0.0, 5.0]),
    ("Shoe",     Some(Side::Left), ExtraParent::Role(HumanoidRole::LeftFoot), [0.09, 0.04, 0.04], [0.0, -0.02, -0.08], [0.0, 0.0, 0.0]),
    ("Shoe_Heel",Some(Side::Left), ExtraParent::Row(26),                       [0.09, 0.02, -0.04],[0.0, 0.0, -0.03],   [0.0, 0.0, 0.0]),
    ("Hand_Prop_R", Some(Side::Right), ExtraParent::Role(HumanoidRole::RightHand), [-0.80, 1.40, 0.05], [0.0, -0.05, 0.15], [0.0, 0.0, 0.0]),
];

fn mirror_parent(parent: &ExtraParent) -> ExtraParent {
    match parent {
        ExtraParent::Role(role) => ExtraParent::Role(role.mirrored()),
        ExtraParent::Row(i) => {
            let left_position = EXTRA_BONE_TABLE[..=*i]
                .iter()
                .filter(|entry| entry.1 == Some(Side::Left))
                .count()
                - 1;
            ExtraParent::Row(EXTRA_BONE_TABLE.len() + left_position)
        }
    }
}

pub fn extra_bones() -> Vec<ExtraBone> {
    let mut output: Vec<ExtraBone> = Vec::new();

    for &(base, side, parent, position, tip, euler) in EXTRA_BONE_TABLE.iter() {
        let name = if side == Some(Side::Left) {
            format!("{}_L", base)
        } else {
            base.to_string()
        };
        output.push(ExtraBone {
            name,
            parent,
            position,
            tip,
            rest_euler_degrees: euler,
        });
    }

    for (row_idx, &(base, side, parent, position, tip, euler)) in
        EXTRA_BONE_TABLE.iter().enumerate()
    {
        if side != Some(Side::Left) {
            continue;
        }
        let mirrored_parent = mirror_parent(&parent);
        output.push(ExtraBone {
            name: format!("{}_R", base),
            parent: mirrored_parent,
            position: [-position[0], position[1], position[2]],
            tip: [-tip[0], tip[1], tip[2]],
            rest_euler_degrees: [euler[0], -euler[1], -euler[2]],
        });
    }

    output
}
