use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::humanoid::components::avatar_rig::AvatarRig;
use crate::humanoid::components::mapping::{HumanoidMapping, StoredHumanoidMapping, StoredRig};
use crate::humanoid::components::skeleton_input::BoneInput;

pub fn humanoid_mapping_path(model_path: &Path) -> PathBuf {
    let stem = model_path
        .file_stem()
        .expect("model path has a file stem")
        .to_string_lossy();
    let mut mapping_path = model_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    mapping_path.push(format!("{}.humanoid.ron", stem));
    mapping_path
}

pub fn save_mapping(
    path: &Path,
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
) -> anyhow::Result<()> {
    let mut roles = BTreeMap::new();
    for (role, index) in &mapping.by_role {
        let bone_name = &bones[*index].name;
        roles.insert(*role, bone_name.clone());
    }
    let stored = StoredHumanoidMapping {
        roles,
        rig: StoredRig::Confirmed,
    };
    let content = ron::ser::to_string_pretty(&stored, ron::ser::PrettyConfig::new())?;
    fs::write(path, content)?;
    Ok(())
}

pub fn save_not_humanoid(path: &Path) -> anyhow::Result<()> {
    let stored = StoredHumanoidMapping {
        roles: BTreeMap::new(),
        rig: StoredRig::NotHumanoid,
    };
    let content = ron::ser::to_string_pretty(&stored, ron::ser::PrettyConfig::new())?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_or_infer_rig(
    model_path: &Path,
    bones: &[BoneInput],
    imported: Option<&HumanoidMapping>,
) -> anyhow::Result<(AvatarRig, Vec<String>)> {
    let mapping_path = humanoid_mapping_path(model_path);
    if mapping_path.exists() {
        let data = fs::read_to_string(&mapping_path)?;
        let stored: StoredHumanoidMapping = ron::from_str(&data)?;
        match stored.rig {
            StoredRig::NotHumanoid => Ok((AvatarRig::NotHumanoid, Vec::new())),
            StoredRig::Confirmed => {
                let (mapping, missing) = resolve_stored(&stored, bones);
                Ok((AvatarRig::Confirmed(mapping), missing))
            }
        }
    } else if let Some(imported) = imported {
        Ok((AvatarRig::Inferred(imported.clone()), Vec::new()))
    } else {
        let (mapping, _) = crate::humanoid::systems::name_match::infer_mapping(bones);
        Ok((AvatarRig::Inferred(mapping), Vec::new()))
    }
}

fn resolve_stored(
    stored: &StoredHumanoidMapping,
    bones: &[BoneInput],
) -> (HumanoidMapping, Vec<String>) {
    let mut by_role = BTreeMap::new();
    let mut missing = Vec::new();

    for (role, bone_name) in &stored.roles {
        match bones.iter().position(|b| b.name == *bone_name) {
            Some(index) => {
                by_role.insert(*role, index);
            }
            None => {
                missing.push(bone_name.clone());
            }
        }
    }

    let mapping = HumanoidMapping { by_role };
    (mapping, missing)
}

pub fn load_mapping(
    path: &Path,
    bones: &[BoneInput],
) -> anyhow::Result<(HumanoidMapping, Vec<String>)> {
    let data = fs::read_to_string(path)?;
    let stored: StoredHumanoidMapping = ron::from_str(&data)?;
    let (mapping, missing) = resolve_stored(&stored, bones);
    Ok((mapping, missing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humanoid::components::role::HumanoidRole;
    use tempfile::tempdir;

    fn make_bones(names: &[&str]) -> Vec<BoneInput> {
        names
            .iter()
            .enumerate()
            .map(|(i, name)| BoneInput {
                name: name.to_string(),
                parent: if i == 0 { None } else { Some(i - 1) },
                rest_position: [0.0, 0.0, 0.0],
            })
            .collect()
    }

    #[test]
    fn test_save_load_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.humanoid.ron");

        let bones = make_bones(&["Hips", "Spine", "LeftUpperArm", "RightUpperArm"]);

        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);
        mapping.by_role.insert(HumanoidRole::Spine, 1);
        mapping.by_role.insert(HumanoidRole::LeftUpperArm, 2);
        mapping.by_role.insert(HumanoidRole::RightUpperArm, 3);

        save_mapping(&path, &mapping, &bones).unwrap();

        let (loaded, missing) = load_mapping(&path, &bones).unwrap();

        assert!(missing.is_empty());
        assert_eq!(loaded.by_role.get(&HumanoidRole::Hips), Some(&0));
        assert_eq!(loaded.by_role.get(&HumanoidRole::Spine), Some(&1));
        assert_eq!(loaded.by_role.get(&HumanoidRole::LeftUpperArm), Some(&2));
        assert_eq!(loaded.by_role.get(&HumanoidRole::RightUpperArm), Some(&3));
    }

    #[test]
    fn test_renamed_bone_reported_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.humanoid.ron");

        let original_bones = make_bones(&["Hips", "Spine", "LeftUpperArm"]);

        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);
        mapping.by_role.insert(HumanoidRole::Spine, 1);
        mapping.by_role.insert(HumanoidRole::LeftUpperArm, 2);

        save_mapping(&path, &mapping, &original_bones).unwrap();

        let renamed_bones = make_bones(&["Hips", "Spine", "LeftArm"]);

        let (loaded, missing) = load_mapping(&path, &renamed_bones).unwrap();

        assert_eq!(missing, vec!["LeftUpperArm".to_string()]);
        assert_eq!(loaded.by_role.get(&HumanoidRole::Hips), Some(&0));
        assert_eq!(loaded.by_role.get(&HumanoidRole::Spine), Some(&1));
        assert_eq!(loaded.by_role.get(&HumanoidRole::LeftUpperArm), None);
    }

    #[test]
    fn test_humanoid_mapping_path() {
        let model_path = Path::new("/assets/models/character.fbx");
        let mapping_path = humanoid_mapping_path(model_path);
        assert_eq!(
            mapping_path,
            PathBuf::from("/assets/models/character.humanoid.ron")
        );
    }

    #[test]
    fn test_humanoid_mapping_path_no_extension() {
        let model_path = Path::new("/assets/models/character");
        let mapping_path = humanoid_mapping_path(model_path);
        assert_eq!(
            mapping_path,
            PathBuf::from("/assets/models/character.humanoid.ron")
        );
    }

    #[test]
    fn test_old_ron_without_rig_field_reads_as_confirmed() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("model.fbx");
        let sidecar_path = humanoid_mapping_path(&model_path);

        let old_content = "(
            roles: {
                Hips: \"Hips\",
                Spine: \"Spine\",
            },
        )";
        fs::write(&sidecar_path, old_content).unwrap();

        let bones = make_bones(&["Hips", "Spine"]);
        let (rig, missing) = load_or_infer_rig(&model_path, &bones, None).unwrap();
        assert!(missing.is_empty());
        match rig {
            AvatarRig::Confirmed(mapping) => {
                assert_eq!(mapping.by_role.get(&HumanoidRole::Hips), Some(&0));
                assert_eq!(mapping.by_role.get(&HumanoidRole::Spine), Some(&1));
            }
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn test_save_not_humanoid_and_load() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("model.fbx");
        let sidecar_path = humanoid_mapping_path(&model_path);

        save_not_humanoid(&sidecar_path).unwrap();

        let bones = make_bones(&["Hips", "Spine"]);
        let (rig, missing) = load_or_infer_rig(&model_path, &bones, None).unwrap();
        assert!(missing.is_empty());
        assert!(matches!(rig, AvatarRig::NotHumanoid));
    }

    #[test]
    fn test_no_sidecar_returns_inferred() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("model.fbx");

        let bones = make_bones(&["Hips", "Spine"]);
        let (rig, missing) = load_or_infer_rig(&model_path, &bones, None).unwrap();
        assert!(missing.is_empty());
        assert!(matches!(rig, AvatarRig::Inferred(_)), "expected Inferred");
    }

    #[test]
    fn test_no_sidecar_with_imported_returns_imported_as_inferred() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("model.fbx");

        let bones = make_bones(&["Hips", "Spine"]);
        let mut imported = HumanoidMapping::default();
        imported.by_role.insert(HumanoidRole::Hips, 0);
        imported.by_role.insert(HumanoidRole::Spine, 1);

        let (rig, missing) = load_or_infer_rig(&model_path, &bones, Some(&imported)).unwrap();
        assert!(missing.is_empty());
        match rig {
            AvatarRig::Inferred(mapping) => {
                assert_eq!(mapping.by_role.get(&HumanoidRole::Hips), Some(&0));
                assert_eq!(mapping.by_role.get(&HumanoidRole::Spine), Some(&1));
            }
            other => panic!("expected Inferred, got {:?}", other),
        }
    }

    #[test]
    fn test_sidecar_takes_priority_over_imported() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("model.fbx");
        let sidecar_path = humanoid_mapping_path(&model_path);

        let bones = make_bones(&["Hips", "Spine"]);

        let mut sidecar_mapping = HumanoidMapping::default();
        sidecar_mapping.by_role.insert(HumanoidRole::Hips, 0);
        save_mapping(&sidecar_path, &sidecar_mapping, &bones).unwrap();

        let mut imported = HumanoidMapping::default();
        imported.by_role.insert(HumanoidRole::Spine, 1);

        let (rig, missing) = load_or_infer_rig(&model_path, &bones, Some(&imported)).unwrap();
        assert!(missing.is_empty());
        match rig {
            AvatarRig::Confirmed(mapping) => {
                assert_eq!(mapping.by_role.get(&HumanoidRole::Hips), Some(&0));
                assert_eq!(mapping.by_role.get(&HumanoidRole::Spine), None);
            }
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }
}
