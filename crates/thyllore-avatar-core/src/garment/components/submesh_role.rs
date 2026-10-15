use std::collections::BTreeMap;

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubmeshKey {
    pub mesh: String,
    pub material: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmeshRoleSource {
    Guessed,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmeshRoleEntry {
    pub role: SubmeshRole,
    pub source: SubmeshRoleSource,
}

const HAIR_KEYWORDS: &[&str] = &["hair", "bang", "ponytail", "twintail"];
const SKIN_KEYWORDS: &[&str] = &["body", "skin", "face", "head", "hand", "leg", "arm"];
const ACCESSORY_KEYWORDS: &[&str] = &[
    "accessory",
    "acc_",
    "ribbon",
    "glasses",
    "earring",
    "necklace",
    "hat",
];

pub fn guess_submesh_role(key: &SubmeshKey) -> SubmeshRole {
    let combined = format!("{}{}", key.mesh, key.material).to_lowercase();
    for kw in HAIR_KEYWORDS {
        if combined.contains(kw) {
            return SubmeshRole::Hair;
        }
    }
    for kw in SKIN_KEYWORDS {
        if combined.contains(kw) {
            return SubmeshRole::Skin;
        }
    }
    for kw in ACCESSORY_KEYWORDS {
        if combined.contains(kw) {
            return SubmeshRole::Accessory;
        }
    }
    SubmeshRole::Garment
}

pub fn merge_guessed_roles(
    current: &BTreeMap<SubmeshKey, SubmeshRoleEntry>,
    keys: &[SubmeshKey],
) -> BTreeMap<SubmeshKey, SubmeshRoleEntry> {
    let mut out = BTreeMap::new();
    for key in keys {
        if let Some(entry) = current.get(key) {
            if entry.source == SubmeshRoleSource::User {
                out.insert(key.clone(), *entry);
            } else {
                out.insert(
                    key.clone(),
                    SubmeshRoleEntry {
                        role: guess_submesh_role(key),
                        source: SubmeshRoleSource::Guessed,
                    },
                );
            }
        } else {
            out.insert(
                key.clone(),
                SubmeshRoleEntry {
                    role: guess_submesh_role(key),
                    source: SubmeshRoleSource::Guessed,
                },
            );
        }
    }
    out
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
    fn test_guess_garment_and_idempotent() {
        let k_coat = key("Coat", "Cloth");
        assert_eq!(guess_submesh_role(&k_coat), SubmeshRole::Garment);

        let mut current: BTreeMap<SubmeshKey, SubmeshRoleEntry> = BTreeMap::new();
        let keys = [k_coat.clone()];

        let first = merge_guessed_roles(&current, &keys);
        assert_eq!(first.len(), 1);
        assert_eq!(
            first.get(&k_coat).unwrap().source,
            SubmeshRoleSource::Guessed
        );

        let second = merge_guessed_roles(&first, &keys);
        assert_eq!(first, second);

        current.insert(
            k_coat.clone(),
            SubmeshRoleEntry {
                role: SubmeshRole::Garment,
                source: SubmeshRoleSource::User,
            },
        );
        let with_user = merge_guessed_roles(&current, &keys);
        assert_eq!(
            with_user.get(&k_coat).unwrap().source,
            SubmeshRoleSource::User
        );

        let empty_keys: [SubmeshKey; 0] = [];
        let dropped = merge_guessed_roles(&current, &empty_keys);
        assert!(dropped.is_empty());
    }
}
