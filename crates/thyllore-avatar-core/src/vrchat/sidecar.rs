use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::expression::components::preset::ExpressionLibrary;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::skeleton_input::BoneInput;
use crate::stats::components::stats::AvatarStats;
use crate::vrchat::blink::find_blink_candidates;
use crate::vrchat::viseme::VISEME_CHANNELS;

pub const SIDECAR_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AvatarSidecar {
    pub schema: u32,
    pub humanoid: BTreeMap<String, String>,
    pub visemes: BTreeMap<String, String>,
    pub blink: SidecarBlink,
    pub expression_mesh: Option<String>,
    pub expressions: Vec<SidecarExpression>,
    pub spring_chains: Vec<SidecarSpringChain>,
    pub stats: AvatarStats,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SidecarBlink {
    pub both: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SidecarExpression {
    pub name: String,
    pub weights: BTreeMap<String, f32>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SidecarSpringChain {
    pub root: String,
    pub stiffness: f32,
    pub gravity: f32,
    pub drag: f32,
    pub colliders: Vec<String>,
}

pub struct SidecarInput<'a> {
    pub mapping: &'a HumanoidMapping,
    pub bones: &'a [BoneInput],
    pub channel_names: &'a [String],
    pub expression_mesh_name: Option<&'a str>,
    pub library: &'a ExpressionLibrary,
    pub spring_chains: &'a [SidecarSpringChain],
    pub stats: &'a AvatarStats,
}

pub fn build_sidecar(input: SidecarInput<'_>) -> AvatarSidecar {
    let humanoid = build_hybrid_bone_map(input.mapping, input.bones);
    let visemes = build_viseme_map(input.channel_names);
    let blink = build_blink(input.channel_names);
    let expression_mesh = input.expression_mesh_name.map(|s| s.to_string());
    let expressions = build_expressions(input.library);

    AvatarSidecar {
        schema: SIDECAR_SCHEMA_VERSION,
        humanoid,
        visemes,
        blink,
        expression_mesh,
        expressions,
        spring_chains: input.spring_chains.to_vec(),
        stats: input.stats.clone(),
    }
}

fn build_hybrid_bone_map(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (role, index) in &mapping.by_role {
        let unity_name = role.unity_name();
        let bone_name = &bones[*index].name;
        map.insert(unity_name.to_string(), bone_name.clone());
    }
    map
}

fn build_viseme_map(channel_names: &[String]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (key, channel) in &VISEME_CHANNELS {
        if channel_names.contains(&channel.to_string()) {
            map.insert(key.to_string(), channel.to_string());
        }
    }
    map
}

fn build_blink(channel_names: &[String]) -> SidecarBlink {
    let candidates = find_blink_candidates(channel_names);
    SidecarBlink {
        both: candidates.both,
        left: candidates.left,
        right: candidates.right,
    }
}

fn build_expressions(library: &ExpressionLibrary) -> Vec<SidecarExpression> {
    library
        .presets
        .iter()
        .filter(|preset| !preset.weights.is_empty())
        .map(|preset| SidecarExpression {
            name: preset.name.clone(),
            weights: preset.weights.clone(),
        })
        .collect()
}

pub fn write_sidecar_json(path: &Path, sidecar: &AvatarSidecar) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(sidecar)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn sidecar_path(model_path: &Path) -> PathBuf {
    let stem = model_path
        .file_stem()
        .expect("model path has a file stem")
        .to_string_lossy();
    let mut sidecar_path = model_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    sidecar_path.push(format!("{}.avatar.json", stem));
    sidecar_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humanoid::components::role::HumanoidRole;

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
    fn test_build_and_serialize_sidecar() {
        let bones = make_bones(&["Hips", "Spine", "LeftUpperArm"]);

        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);
        mapping.by_role.insert(HumanoidRole::Spine, 1);
        mapping.by_role.insert(HumanoidRole::LeftUpperArm, 2);

        let channel_names: Vec<String> = vec!["vrc.v_aa".to_string(), "eye_blink".to_string()];

        let mut weights = BTreeMap::new();
        weights.insert("mouth_smile".to_string(), 1.0);
        let preset_with_weight = crate::expression::components::preset::ExpressionPreset {
            name: "happy".to_string(),
            weights,
        };

        let empty_preset = crate::expression::components::preset::ExpressionPreset {
            name: "empty".to_string(),
            weights: BTreeMap::new(),
        };

        let library = ExpressionLibrary {
            presets: vec![preset_with_weight, empty_preset],
        };

        let stats = AvatarStats {
            triangles: 1000,
            bones: 5,
            materials: 2,
            skinned_meshes: 1,
            meshes: 3,
            morph_meshes: 1,
            spring_chains: 0,
            spring_transforms: 0,
            spring_colliders: 0,
            texture_bytes: 0,
        };

        let input = SidecarInput {
            mapping: &mapping,
            bones: &bones,
            channel_names: &channel_names,
            expression_mesh_name: Some("face_expression"),
            library: &library,
            spring_chains: &[],
            stats: &stats,
        };

        let sidecar = build_sidecar(input);

        let value: serde_json::Value = serde_json::to_value(&sidecar).unwrap();

        assert_eq!(value["schema"], 1);

        assert_eq!(value["humanoid"]["Hips"], "Hips");

        assert_eq!(value["visemes"]["aa"], "vrc.v_aa");

        let expressions: &serde_json::Value = &value["expressions"];
        let expressions_array = expressions.as_array().unwrap();
        assert_eq!(expressions_array.len(), 1);
        assert_eq!(expressions_array[0]["name"], "happy");

        assert_eq!(value["expression_mesh"], "face_expression");

        assert_eq!(value["blink"]["both"], "eye_blink");

        assert_eq!(value["stats"]["triangles"], 1000);
    }

    #[test]
    fn test_sidecar_path() {
        let model_path = Path::new("/assets/models/character.fbx");
        let path = sidecar_path(model_path);
        assert_eq!(path, PathBuf::from("/assets/models/character.avatar.json"));
    }

    #[test]
    fn test_write_and_parse_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.avatar.json");

        let sidecar = AvatarSidecar {
            schema: SIDECAR_SCHEMA_VERSION,
            humanoid: BTreeMap::new(),
            visemes: BTreeMap::new(),
            blink: SidecarBlink {
                both: None,
                left: None,
                right: None,
            },
            expression_mesh: None,
            expressions: vec![],
            spring_chains: vec![],
            stats: AvatarStats::default(),
        };

        write_sidecar_json(&path, &sidecar).unwrap();

        let data = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&data).unwrap();

        assert_eq!(parsed["schema"], 1);
    }
}
