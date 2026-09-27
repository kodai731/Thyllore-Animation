use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct HumanoidNamingRules {
    pub role: Vec<RoleEntry>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoleEntry {
    pub part: String,
    pub patterns: Vec<Vec<String>>,
}

impl Default for HumanoidNamingRules {
    fn default() -> Self {
        load_default()
    }
}

fn load_default() -> HumanoidNamingRules {
    let data = include_str!("../../../data/humanoid_naming.toml");
    toml::from_str(data).expect("humanoid_naming.toml is valid TOML")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default() {
        let rules = HumanoidNamingRules::default();
        assert!(!rules.role.is_empty());
        let hips = rules
            .role
            .iter()
            .find(|r| r.part == "Hips")
            .expect("Hips role");
        assert!(hips.patterns.contains(&vec!["hips".to_string()]));
    }
}
