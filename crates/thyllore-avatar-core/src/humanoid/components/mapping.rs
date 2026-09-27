use std::collections::BTreeMap;

use super::role::HumanoidRole;

#[derive(Clone, Debug, Default)]
pub struct HumanoidMapping {
    pub by_role: BTreeMap<HumanoidRole, usize>,
}

#[derive(Clone, Debug)]
pub struct UnresolvedRole {
    pub role: HumanoidRole,
}
