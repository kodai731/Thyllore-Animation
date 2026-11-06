use std::collections::BTreeMap;

use cgmath::Vector3;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::vrm::{t_pose_warnings, VrmMeta, T_POSE_TOLERANCE_DEG};
use thyllore_exporter_core::systems::gltf::write_vrm;

use crate::avatar_setup_bridge::sidecar_read;

/// The T-pose warnings of the rig the VRM would carry: the inferred humanoid mapping with the
/// user's role overrides (Unity role name to bone name) on top.
pub fn vrm_t_pose_warnings(
    bones: &[BoneInput],
    humanoid_overrides: &BTreeMap<String, String>,
) -> anyhow::Result<Vec<String>> {
    let (mut mapping, _) = infer_mapping(bones);
    for (role_name, bone_name) in humanoid_overrides {
        let role = HumanoidRole::from_unity_name(role_name)
            .ok_or_else(|| anyhow::anyhow!("unknown humanoid role {role_name:?}"))?;
        let bone = bones
            .iter()
            .position(|bone| bone.name == *bone_name)
            .ok_or_else(|| {
                anyhow::anyhow!("humanoid override {role_name:?} names unknown bone {bone_name:?}")
            })?;
        mapping.by_role.insert(role, bone);
    }
    let heads: Vec<Vector3<f32>> = bones
        .iter()
        .map(|bone| Vector3::from(bone.rest_position))
        .collect();
    Ok(t_pose_warnings(&mapping, &heads, T_POSE_TOLERANCE_DEG))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VrmWriteRequest {
    pub glb: serde_bytes::ByteBuf,
    pub sidecar_json: String,
    pub meta: VrmMeta,
    #[serde(default = "default_metres_per_gltf_unit")]
    pub metres_per_gltf_unit: f32,
}

fn default_metres_per_gltf_unit() -> f32 {
    1.0
}

/// The `.vrm` bytes for a GLB the standard glTF exporter wrote and the sidecar JSON of the same
/// armature; `metres_per_gltf_unit` is the scene's unit scale (the sidecar is in metres).
pub fn vrm_write(request: &VrmWriteRequest) -> anyhow::Result<Vec<u8>> {
    let sidecar = sidecar_read(&request.sidecar_json)?;
    write_vrm(
        &request.glb,
        &sidecar,
        &request.meta,
        request.metres_per_gltf_unit,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use thyllore_avatar_core::stats::components::stats::AvatarStats;
    use thyllore_avatar_core::vrchat::sidecar::{
        AvatarSidecar, SidecarBlink, SIDECAR_SCHEMA_VERSION,
    };
    use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring;

    use super::*;

    fn glb_with_json(json: &str) -> Vec<u8> {
        let mut json_bytes = json.as_bytes().to_vec();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let total = 12 + 8 + json_bytes.len();
        let mut glb = Vec::with_capacity(total);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb
    }

    fn two_bone_glb() -> Vec<u8> {
        glb_with_json(
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
"nodes":[{"name":"Hips","children":[1],"translation":[0,1,0]},{"name":"Head","translation":[0,0.5,0]}]}"#,
        )
    }

    fn sidecar_json() -> String {
        let sidecar = AvatarSidecar {
            schema: SIDECAR_SCHEMA_VERSION,
            humanoid: [("Hips", "Hips"), ("Head", "Head")]
                .into_iter()
                .map(|(role, bone)| (role.to_string(), bone.to_string()))
                .collect(),
            visemes: BTreeMap::new(),
            blink: SidecarBlink {
                both: None,
                left: None,
                right: None,
            },
            expression_mesh: None,
            expressions: vec![],
            spring: SidecarSpring::default(),
            materials: BTreeMap::new(),
            submesh_roles: vec![],
            hidden_triangles: vec![],
            stats: AvatarStats::default(),
        };
        serde_json::to_string(&sidecar).expect("sidecar json")
    }

    #[test]
    fn writes_a_vrm_with_the_three_extensions() {
        let meta = VrmMeta::new("avatar", "tester");
        let request = VrmWriteRequest {
            glb: two_bone_glb().into(),
            sidecar_json: sidecar_json(),
            meta,
            metres_per_gltf_unit: 1.0,
        };

        let vrm = vrm_write(&request).expect("vrm");

        let json_length = u32::from_le_bytes(vrm[12..16].try_into().unwrap()) as usize;
        let json: serde_json::Value = serde_json::from_slice(&vrm[20..20 + json_length]).unwrap();
        assert_eq!(
            json["extensionsUsed"],
            serde_json::json!(["VRMC_vrm", "VRMC_springBone", "THYLLORE_avatar"])
        );
        assert_eq!(
            json["extensions"]["VRMC_vrm"]["humanoid"]["humanBones"]["head"]["node"],
            1
        );
        assert_eq!(json["extensions"]["THYLLORE_avatar"]["schema"], 4);
    }

    #[test]
    fn invalid_sidecar_json_is_an_error() {
        let meta = VrmMeta::new("avatar", "tester");
        let request = VrmWriteRequest {
            glb: two_bone_glb().into(),
            sidecar_json: "{".to_string(),
            meta,
            metres_per_gltf_unit: 1.0,
        };
        assert!(vrm_write(&request).is_err());
    }

    #[test]
    fn vrm_bridge_vrm_write_request_rejects_unknown_field() {
        let meta_json = serde_json::to_value(VrmMeta::new("avatar", "tester")).unwrap();
        let mut obj: serde_json::Value =
            serde_json::from_str(r#"{"glb":"","sidecar_json":"{}","metres_per_gltf_unit":1.0}"#)
                .unwrap();
        obj["meta"] = meta_json;
        obj["unknown_key"] = serde_json::json!(1);
        let json = serde_json::to_string(&obj).unwrap();
        let result = serde_json::from_str::<VrmWriteRequest>(&json);
        let err = result.err().unwrap();
        assert!(
            err.to_string().contains("unknown field"),
            "expected unknown field error, got: {err}"
        );
    }
}
