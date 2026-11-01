use std::collections::BTreeMap;

use anyhow::Context;
use serde_json::{json, Value};

use super::meta::VrmMeta;
use crate::humanoid::components::role::HumanoidRole;
use crate::humanoid::components::vrm_version::VrmVersion;

pub const VRMC_VRM: &str = "VRMC_vrm";

/// The `VRMC_vrm` extension: meta and the humanoid bones, from the sidecar's Unity-named bone map.
pub fn build_vrmc_vrm(
    humanoid: &BTreeMap<String, String>,
    node_of_bone: &BTreeMap<String, u32>,
    meta: &VrmMeta,
) -> anyhow::Result<Value> {
    meta.validate()?;
    let human_bones = humanoid
        .iter()
        .map(|(unity_name, bone)| {
            let role = HumanoidRole::from_unity_name(unity_name)
                .with_context(|| format!("unknown humanoid role {unity_name:?}"))?;
            let node = node_of_bone
                .get(bone)
                .copied()
                .with_context(|| format!("humanoid bone {bone:?} has no glTF node"))?;
            Ok((role.vrm_name(VrmVersion::V1).to_string(), node))
        })
        .collect::<anyhow::Result<Vec<(String, u32)>>>()?;

    Ok(json!({
        "specVersion": "1.0",
        "meta": serde_json::to_value(meta)?,
        "humanoid": humanoid_value(&human_bones),
    }))
}

/// `VRMC_vrm` carrying the humanoid alone, for an animation GLB that has no avatar meta.
pub fn build_vrmc_vrm_humanoid(human_bones: &[(String, u32)]) -> Value {
    json!({
        "specVersion": "1.0",
        "humanoid": humanoid_value(human_bones),
    })
}

fn humanoid_value(human_bones: &[(String, u32)]) -> Value {
    let bones: serde_json::Map<String, Value> = human_bones
        .iter()
        .map(|(vrm_name, node)| (vrm_name.clone(), json!({ "node": node })))
        .collect();
    json!({ "humanBones": bones })
}
