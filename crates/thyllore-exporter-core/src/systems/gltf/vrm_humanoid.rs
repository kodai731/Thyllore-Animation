use std::collections::HashMap;

use gltf::json;
use thyllore_anim_core::BoneId;

const VRMC_VRM_EXTENSION: &str = "VRMC_vrm";

pub(crate) fn write_vrm_humanoid(
    root: &mut json::Root,
    humanoid_bones: &[(String, usize)],
    bone_to_node: &HashMap<BoneId, u32>,
) {
    if humanoid_bones.is_empty() {
        return;
    }

    let human_bones: serde_json::Map<String, serde_json::Value> = humanoid_bones
        .iter()
        .filter_map(|(vrm_name, bone_index)| {
            let node = bone_to_node.get(&(*bone_index as BoneId))?;
            Some((vrm_name.clone(), serde_json::json!({ "node": node })))
        })
        .collect();

    let vrmc_vrm = serde_json::json!({
        "specVersion": "1.0",
        "humanoid": { "humanBones": human_bones },
    });

    root.extensions
        .get_or_insert_with(Default::default)
        .others
        .insert(VRMC_VRM_EXTENSION.to_string(), vrmc_vrm);

    if !root
        .extensions_used
        .iter()
        .any(|name| name == VRMC_VRM_EXTENSION)
    {
        root.extensions_used.push(VRMC_VRM_EXTENSION.to_string());
    }
}
