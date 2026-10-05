use std::collections::BTreeMap;

use super::role::HumanoidRole;

#[derive(Clone, Debug, Default)]
pub struct HumanoidMapping {
    pub by_role: BTreeMap<HumanoidRole, usize>,
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
    Confirmed,
    NotHumanoid,
}

#[derive(Clone, Debug)]
pub struct UnresolvedRole {
    pub role: HumanoidRole,
}
