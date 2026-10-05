use super::mapping::HumanoidMapping;

#[derive(Clone, Debug)]
pub enum AvatarRig {
    NotHumanoid,
    Inferred(HumanoidMapping),
    Confirmed(HumanoidMapping),
}

impl AvatarRig {
    pub fn mapping(&self) -> Option<&HumanoidMapping> {
        match self {
            AvatarRig::NotHumanoid => None,
            AvatarRig::Inferred(mapping) | AvatarRig::Confirmed(mapping) => Some(mapping),
        }
    }
}
