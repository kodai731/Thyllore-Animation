#[derive(Clone, Debug)]
pub struct VrmHumanoid {
    pub is_v1: bool,
    pub bones: Vec<(String, u32)>,
}

pub fn parse_vrm_humanoid(
    vrmc_vrm: Option<&serde_json::Value>,
    vrm0: Option<&serde_json::Value>,
) -> Option<VrmHumanoid> {
    if let Some(v1) = vrmc_vrm {
        parse_v1(v1)
    } else if let Some(v0) = vrm0 {
        parse_v0(v0)
    } else {
        None
    }
}

fn parse_v1(vrmc_vrm: &serde_json::Value) -> Option<VrmHumanoid> {
    let humanoid = vrmc_vrm.get("humanoid")?;
    let human_bones = humanoid.get("humanBones")?;
    let human_bones = human_bones.as_object()?;

    let mut bones: Vec<(String, u32)> = Vec::new();
    for (bone_name, entry) in human_bones {
        let node = entry.get("node")?.as_u64()? as u32;
        bones.push((bone_name.clone(), node));
    }

    Some(VrmHumanoid { is_v1: true, bones })
}

fn parse_v0(vrm0: &serde_json::Value) -> Option<VrmHumanoid> {
    let humanoid = vrm0.get("humanoid")?;
    let human_bones = humanoid.get("humanBones")?;
    let human_bones = human_bones.as_array()?;

    let mut bones: Vec<(String, u32)> = Vec::new();
    for entry in human_bones {
        let bone_name = entry.get("bone")?.as_str()?.to_string();
        let node = entry.get("node")?.as_u64()? as u32;
        bones.push((bone_name, node));
    }

    Some(VrmHumanoid {
        is_v1: false,
        bones,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v1_json(bones: &[(&str, u32)]) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for (name, node) in bones {
            let mut entry = serde_json::Map::new();
            entry.insert("node".to_string(), serde_json::json!(node));
            obj.insert(name.to_string(), serde_json::Value::Object(entry));
        }
        let mut humanoid = serde_json::Map::new();
        humanoid.insert("humanBones".to_string(), serde_json::Value::Object(obj));
        let mut vrmc = serde_json::Map::new();
        vrmc.insert("humanoid".to_string(), serde_json::Value::Object(humanoid));
        serde_json::Value::Object(vrmc)
    }

    fn make_v0_json(bones: &[(&str, u32)]) -> serde_json::Value {
        let arr: Vec<serde_json::Value> = bones
            .iter()
            .map(|(name, node)| {
                let mut obj = serde_json::Map::new();
                obj.insert("bone".to_string(), serde_json::json!(name));
                obj.insert("node".to_string(), serde_json::json!(node));
                serde_json::Value::Object(obj)
            })
            .collect();
        let mut humanoid = serde_json::Map::new();
        humanoid.insert("humanBones".to_string(), serde_json::Value::Array(arr));
        let mut vrm0 = serde_json::Map::new();
        vrm0.insert("humanoid".to_string(), serde_json::Value::Object(humanoid));
        serde_json::Value::Object(vrm0)
    }

    #[test]
    fn test_parse_v1_object_form() {
        let v1 = make_v1_json(&[("hips", 3), ("spine", 4)]);
        let result = parse_vrm_humanoid(Some(&v1), None);
        assert!(result.is_some());
        let rh = result.unwrap();
        assert!(rh.is_v1);
        assert_eq!(rh.bones.len(), 2);
        assert_eq!(rh.bones[0], ("hips".to_string(), 3));
        assert_eq!(rh.bones[1], ("spine".to_string(), 4));
    }

    #[test]
    fn test_parse_v0_array_form() {
        let v0 = make_v0_json(&[("hips", 5), ("spine", 6)]);
        let result = parse_vrm_humanoid(None, Some(&v0));
        assert!(result.is_some());
        let rh = result.unwrap();
        assert!(!rh.is_v1);
        assert_eq!(rh.bones.len(), 2);
        assert_eq!(rh.bones[0], ("hips".to_string(), 5));
        assert_eq!(rh.bones[1], ("spine".to_string(), 6));
    }

    #[test]
    fn test_v1_priority_when_both_present() {
        let v1 = make_v1_json(&[("hips", 3), ("spine", 4)]);
        let v0 = make_v0_json(&[("hips", 99), ("spine", 98)]);
        let result = parse_vrm_humanoid(Some(&v1), Some(&v0));
        assert!(result.is_some());
        let rh = result.unwrap();
        assert!(rh.is_v1);
        assert_eq!(rh.bones.len(), 2);
        assert_eq!(rh.bones[0], ("hips".to_string(), 3));
        assert_eq!(rh.bones[1], ("spine".to_string(), 4));
    }
}
