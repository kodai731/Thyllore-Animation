#[derive(Clone, Debug)]
pub struct RankThresholds {
    pub pc: PlatformThresholds,
    pub quest: PlatformThresholds,
}

#[derive(Clone, Debug)]
pub struct PlatformThresholds {
    pub triangles: RankBounds,
    pub texture_memory_mb: RankBounds,
    pub skinned_meshes: RankBounds,
    pub meshes: RankBounds,
    pub material_slots: RankBounds,
    pub physbone_components: RankBounds,
    pub physbone_transforms: RankBounds,
    pub physbone_colliders: RankBounds,
    pub bones: RankBounds,
}

#[derive(Clone, Debug)]
pub struct RankBounds {
    pub excellent: u64,
    pub good: u64,
    pub medium: u64,
    pub poor: u64,
}

fn load() -> RankThresholds {
    let toml_str = include_str!("../../data/vrchat_rank_thresholds.toml");
    let table: toml::Table =
        toml::from_str(toml_str).expect("failed to parse rank thresholds TOML");

    let pc_table = table
        .get("pc")
        .and_then(|v| v.as_table())
        .expect("missing [pc] table");
    let quest_table = table
        .get("quest")
        .and_then(|v| v.as_table())
        .expect("missing [quest] table");

    RankThresholds {
        pc: parse_platform(pc_table),
        quest: parse_platform(quest_table),
    }
}

fn parse_platform(table: &toml::Table) -> PlatformThresholds {
    let triangles = parse_bounds(table.get("triangles").expect("missing triangles"));
    let texture_memory_mb = parse_bounds(
        table
            .get("texture_memory_mb")
            .expect("missing texture_memory_mb"),
    );
    let skinned_meshes = parse_bounds(table.get("skinned_meshes").expect("missing skinned_meshes"));
    let meshes = parse_bounds(table.get("meshes").expect("missing meshes"));
    let material_slots = parse_bounds(table.get("material_slots").expect("missing material_slots"));
    let physbone_components = parse_bounds(
        table
            .get("physbone_components")
            .expect("missing physbone_components"),
    );
    let physbone_transforms = parse_bounds(
        table
            .get("physbone_transforms")
            .expect("missing physbone_transforms"),
    );
    let physbone_colliders = parse_bounds(
        table
            .get("physbone_colliders")
            .expect("missing physbone_colliders"),
    );
    let bones = parse_bounds(table.get("bones").expect("missing bones"));

    PlatformThresholds {
        triangles,
        texture_memory_mb,
        skinned_meshes,
        meshes,
        material_slots,
        physbone_components,
        physbone_transforms,
        physbone_colliders,
        bones,
    }
}

fn parse_bounds(value: &toml::Value) -> RankBounds {
    let table = value.as_table().expect("rank bounds must be a table");
    RankBounds {
        excellent: table.get("excellent").and_then(|v| v.as_integer()).unwrap() as u64,
        good: table.get("good").and_then(|v| v.as_integer()).unwrap() as u64,
        medium: table.get("medium").and_then(|v| v.as_integer()).unwrap() as u64,
        poor: table.get("poor").and_then(|v| v.as_integer()).unwrap() as u64,
    }
}

pub fn default_thresholds() -> RankThresholds {
    load()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_smoke() {
        let thresholds = default_thresholds();
        assert_eq!(thresholds.pc.triangles.excellent, 32000);
        assert_eq!(thresholds.quest.triangles.excellent, 7500);
    }

    #[test]
    fn test_pc_values() {
        let thresholds = default_thresholds();
        assert_eq!(thresholds.pc.bones.excellent, 75);
        assert_eq!(thresholds.pc.bones.good, 150);
        assert_eq!(thresholds.pc.bones.medium, 256);
        assert_eq!(thresholds.pc.bones.poor, 400);
    }

    #[test]
    fn test_quest_values() {
        let thresholds = default_thresholds();
        assert_eq!(thresholds.quest.bones.excellent, 75);
        assert_eq!(thresholds.quest.bones.good, 90);
        assert_eq!(thresholds.quest.bones.medium, 150);
        assert_eq!(thresholds.quest.bones.poor, 150);
    }

    #[test]
    fn test_clone() {
        let thresholds = default_thresholds();
        let cloned = thresholds.clone();
        assert_eq!(
            cloned.pc.triangles.excellent,
            thresholds.pc.triangles.excellent
        );
        assert_eq!(cloned.quest.bones.poor, thresholds.quest.bones.poor);
    }
}
