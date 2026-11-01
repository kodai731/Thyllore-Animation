use thyllore_avatar_core::vrm::VrmMeta;
use thyllore_exporter_core::systems::gltf::write_vrm;

use crate::avatar_setup_bridge::sidecar_read;

/// The `.vrm` bytes for a GLB the standard glTF exporter wrote and the sidecar JSON of the same armature.
pub fn vrm_write(glb_bytes: &[u8], sidecar_json: &str, meta: &VrmMeta) -> anyhow::Result<Vec<u8>> {
    let sidecar = sidecar_read(sidecar_json)?;
    write_vrm(glb_bytes, &sidecar, meta)
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
        while json_bytes.len() % 4 != 0 {
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

        let vrm = vrm_write(&two_bone_glb(), &sidecar_json(), &meta).expect("vrm");

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
        assert!(vrm_write(&two_bone_glb(), "{", &meta).is_err());
    }
}
