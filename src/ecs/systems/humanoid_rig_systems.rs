use std::collections::HashMap;
use std::path::Path;

use thyllore_anim_core::BoneId;
use thyllore_avatar_core::humanoid::components::avatar_rig::AvatarRig;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::systems::mapping_io::load_or_infer_rig;

use crate::animation::Skeleton;
use crate::ecs::resource::HumanoidRig;
use crate::ecs::systems::avatar_setup_systems::skeleton_to_bone_inputs;
use crate::ecs::systems::role_clip_systems::build_role_retarget_context;

pub fn build_humanoid_rig(model_path: &Path, skeleton: &Skeleton) -> Option<HumanoidRig> {
    let bones = skeleton_to_bone_inputs(skeleton);
    let (avatar_rig, _) = match load_or_infer_rig(model_path, &bones) {
        Ok(result) => result,
        Err(error) => {
            log_warn!(
                "Failed to load or infer humanoid rig {}: {}",
                model_path.display(),
                error
            );
            return None;
        }
    };

    let confirmed = matches!(avatar_rig, AvatarRig::Confirmed(_));

    let mapping = match avatar_rig {
        AvatarRig::NotHumanoid => return None,
        AvatarRig::Inferred(mapping) => mapping,
        AvatarRig::Confirmed(mapping) => mapping,
    };

    let context = match build_role_retarget_context(skeleton, &mapping) {
        Ok(ctx) => ctx,
        Err(error) => {
            log_warn!(
                "Failed to build retarget context for {}: {}",
                model_path.display(),
                error
            );
            return None;
        }
    };

    let mut track_bones: HashMap<String, BoneId> = HashMap::new();
    let mut track_names: HashMap<BoneId, String> = HashMap::new();

    for (i, bone) in skeleton.bones.iter().enumerate() {
        let bone_id = i as BoneId;
        let name = if let Some((role, _)) = mapping.by_role.iter().find(|(_, &b)| b == i) {
            role.unity_name().to_string()
        } else {
            let name = &bone.name;
            if HumanoidRole::from_unity_name(name.as_str()).is_some() {
                log_warn!(
                    "Bone '{}' is not in mapping but matches role name, renamed to '{}#bone'",
                    name,
                    name
                );
                format!("{}#bone", name)
            } else {
                name.clone()
            }
        };
        track_bones.insert(name.clone(), bone_id);
        track_names.insert(bone_id, name);
    }

    Some(HumanoidRig {
        confirmed,
        mapping,
        context,
        track_bones,
        track_names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_avatar_core::humanoid::systems::mapping_io::save_not_humanoid;

    fn copy_test_fixture(temp_dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/test_humanoid");
        let fbx_source = source_dir.join("test_humanoid.fbx");
        let sidecar_source = source_dir.join("test_humanoid.humanoid.ron");

        if !fbx_source.exists() || !sidecar_source.exists() {
            panic!(
                "test fixture not found at {:?} (fbx or sidecar missing)",
                source_dir
            );
        }

        let fbx_dest = temp_dir.join("test_humanoid.fbx");
        let sidecar_dest = temp_dir.join("test_humanoid.humanoid.ron");

        std::fs::copy(&fbx_source, &fbx_dest).unwrap();
        std::fs::copy(&sidecar_source, &sidecar_dest).unwrap();

        (fbx_dest, sidecar_dest)
    }

    fn load_skeleton(fbx_path: &Path) -> Skeleton {
        let load_result = thyllore_importer_core::fbx::loader::load_fbx_to_graphics_resources(
            fbx_path.to_str().unwrap(),
        );
        let (fbx_result, _) = load_result.expect("Failed to load FBX");
        fbx_result
            .animation_system
            .skeletons
            .first()
            .expect("No skeleton in loaded model")
            .clone()
    }

    #[test]
    fn test_humanoid_builds_a_confirmed_rig_with_standard_names() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, _) = copy_test_fixture(temp_dir.path());
        let skeleton = load_skeleton(&fbx_path);

        let rig =
            build_humanoid_rig(&fbx_path, &skeleton).expect("build_humanoid_rig returned None");

        assert!(rig.confirmed, "expected confirmed rig");
        assert_eq!(
            rig.track_bones.len(),
            skeleton.bones.len(),
            "expected track_bones to cover all {} bones",
            skeleton.bones.len()
        );

        let head_bone_id = *rig
            .track_bones
            .get("Head")
            .expect("missing Head in track_bones");
        let head_name = &skeleton.bones[head_bone_id as usize].name;
        assert_eq!(head_name, "Head", "Head bone name mismatch");

        assert!(
            rig.track_bones.contains_key("Skirt_Front_1"),
            "missing Skirt_Front_1 in track_bones"
        );

        for name in rig.track_bones.keys() {
            assert!(
                !name.contains("#bone"),
                "found #bone suffix in track name: {}",
                name
            );
        }
    }

    #[test]
    fn not_humanoid_sidecar_builds_no_rig() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, sidecar_path) = copy_test_fixture(temp_dir.path());
        let skeleton = load_skeleton(&fbx_path);

        save_not_humanoid(&sidecar_path).expect("failed to save not-humanoid sidecar");

        let rig = build_humanoid_rig(&fbx_path, &skeleton);
        assert!(rig.is_none(), "expected None for not-humanoid sidecar");
    }
}
