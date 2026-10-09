use super::role::HumanoidRole;

#[derive(Clone, Debug)]
pub enum GeometryWarning {
    Asymmetric {
        left: HumanoidRole,
        right: HumanoidRole,
        distance: f32,
    },
    NotAscending {
        lower: HumanoidRole,
        upper: HumanoidRole,
    },
    LengthRatio {
        upper: HumanoidRole,
        lower: HumanoidRole,
        ratio: f32,
    },
}
