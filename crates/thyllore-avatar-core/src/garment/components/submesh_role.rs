use std::collections::BTreeMap;
use std::sync::LazyLock;

pub use thyllore_anim_core::spring_bone::{SubmeshKey, SubmeshSelector};

/// What the avatar setup does with a submesh: skin is hidden under garments and fit by colliders,
/// garment and hair grow spring chains, an accessory takes part in neither.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SubmeshRole {
    Skin,
    Garment,
    Hair,
    Accessory,
}

#[derive(serde::Deserialize)]
struct RoleKeywords {
    hair: Vec<String>,
    skin: Vec<String>,
    accessory: Vec<String>,
}

static ROLE_KEYWORDS: LazyLock<RoleKeywords> = LazyLock::new(|| {
    toml::from_str(include_str!("../../../data/submesh_role_keywords.toml"))
        .expect("submesh_role_keywords.toml is valid TOML")
});

/// The role the mesh and material names suggest, from `data/submesh_role_keywords.toml`.
pub fn guess_submesh_role(key: &SubmeshKey) -> SubmeshRole {
    let combined = format!("{}{}", key.mesh, key.material).to_lowercase();
    let matches_any =
        |keywords: &[String]| keywords.iter().any(|kw| combined.contains(kw.as_str()));

    if matches_any(&ROLE_KEYWORDS.hair) {
        SubmeshRole::Hair
    } else if matches_any(&ROLE_KEYWORDS.skin) {
        SubmeshRole::Skin
    } else if matches_any(&ROLE_KEYWORDS.accessory) {
        SubmeshRole::Accessory
    } else {
        SubmeshRole::Garment
    }
}

/// The roles of `keys`: an already assigned role is kept, a new submesh gets the guessed role.
pub fn refresh_submesh_roles(
    current: &BTreeMap<SubmeshKey, SubmeshRole>,
    keys: &[SubmeshKey],
) -> BTreeMap<SubmeshKey, SubmeshRole> {
    keys.iter()
        .map(|key| {
            let role = current
                .get(key)
                .copied()
                .unwrap_or_else(|| guess_submesh_role(key));
            (key.clone(), role)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(mesh: &str, material: &str) -> SubmeshKey {
        SubmeshKey {
            mesh: mesh.to_string(),
            material: material.to_string(),
        }
    }

    #[test]
    fn test_guess_hair() {
        assert_eq!(
            guess_submesh_role(&key("Hair_Front", "Hair")),
            SubmeshRole::Hair
        );
    }

    #[test]
    fn test_guess_skin() {
        assert_eq!(guess_submesh_role(&key("Body", "Skin")), SubmeshRole::Skin);
    }

    #[test]
    fn test_guess_accessory() {
        assert_eq!(
            guess_submesh_role(&key("Ribbon", "Cloth")),
            SubmeshRole::Accessory
        );
    }

    #[test]
    fn test_refresh_keeps_assigned_roles_and_is_idempotent() {
        let k_coat = key("Coat", "Cloth");
        assert_eq!(guess_submesh_role(&k_coat), SubmeshRole::Garment);
        let keys = [k_coat.clone()];

        let first = refresh_submesh_roles(&BTreeMap::new(), &keys);
        assert_eq!(first[&k_coat], SubmeshRole::Garment);
        assert_eq!(refresh_submesh_roles(&first, &keys), first);

        let mut assigned = BTreeMap::new();
        assigned.insert(k_coat.clone(), SubmeshRole::Accessory);
        let kept = refresh_submesh_roles(&assigned, &keys);
        assert_eq!(kept[&k_coat], SubmeshRole::Accessory);

        let empty_keys: [SubmeshKey; 0] = [];
        assert!(refresh_submesh_roles(&assigned, &empty_keys).is_empty());
    }
}
