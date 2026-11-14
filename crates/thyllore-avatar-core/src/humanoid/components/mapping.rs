use std::collections::BTreeMap;

use super::role::HumanoidRole;

#[derive(Clone, Debug, Default)]
pub struct HumanoidMapping {
    pub by_role: BTreeMap<HumanoidRole, usize>,
}

impl HumanoidMapping {
    pub fn bone_of(&self, role: HumanoidRole) -> Option<usize> {
        self.by_role.get(&role).copied()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredHumanoidMapping {
    pub roles: BTreeMap<HumanoidRole, String>,
    #[serde(default)]
    pub rig: StoredRig,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StoredRig {
    #[default]
    #[serde(alias = "confirmed")]
    Humanoid,
    #[serde(alias = "nothumanoid")]
    Generic,
}

#[derive(Clone, Debug)]
pub struct UnresolvedRole {
    pub role: HumanoidRole,
}
