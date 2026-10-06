use super::mapping::HumanoidMapping;

#[derive(Clone, Debug)]
pub enum AvatarRig {
    Generic,
    Inferred(HumanoidMapping),
    Humanoid(HumanoidMapping),
}

impl AvatarRig {
    pub fn mapping(&self) -> Option<&HumanoidMapping> {
        match self {
            AvatarRig::Generic => None,
            AvatarRig::Inferred(mapping) | AvatarRig::Humanoid(mapping) => Some(mapping),
        }
    }
}
