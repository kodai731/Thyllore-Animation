use crate::stats::components::stats::AvatarStats;
use crate::vrchat::rank_thresholds::{PlatformThresholds, RankBounds, RankThresholds};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PerformanceRank {
    Excellent,
    Good,
    Medium,
    Poor,
    VeryPoor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Platform {
    #[default]
    Pc,
    Quest,
}

#[derive(Clone, Debug)]
pub struct RankItem {
    pub stat: &'static str,
    pub value: u64,
    pub rank: PerformanceRank,
}

#[derive(Clone, Debug)]
pub struct RankReport {
    pub items: Vec<RankItem>,
    pub overall: PerformanceRank,
}

fn rank_value(value: u64, bounds: &RankBounds) -> PerformanceRank {
    if value <= bounds.excellent {
        PerformanceRank::Excellent
    } else if value <= bounds.good {
        PerformanceRank::Good
    } else if value <= bounds.medium {
        PerformanceRank::Medium
    } else if value <= bounds.poor {
        PerformanceRank::Poor
    } else {
        PerformanceRank::VeryPoor
    }
}

fn platform_thresholds(thresholds: &RankThresholds, platform: Platform) -> &PlatformThresholds {
    match platform {
        Platform::Pc => &thresholds.pc,
        Platform::Quest => &thresholds.quest,
    }
}

pub fn rank_stats(
    stats: &AvatarStats,
    thresholds: &RankThresholds,
    platform: Platform,
) -> RankReport {
    let pt = platform_thresholds(thresholds, platform);

    let items: Vec<RankItem> = [
        ("triangles", stats.triangles as u64, &pt.triangles),
        ("bones", stats.bones as u64, &pt.bones),
        ("material_slots", stats.materials as u64, &pt.material_slots),
        (
            "skinned_meshes",
            stats.skinned_meshes as u64,
            &pt.skinned_meshes,
        ),
        ("meshes", stats.meshes as u64, &pt.meshes),
        (
            "physbone_components",
            stats.spring_chains as u64,
            &pt.physbone_components,
        ),
        (
            "physbone_transforms",
            stats.spring_transforms as u64,
            &pt.physbone_transforms,
        ),
        (
            "physbone_colliders",
            stats.spring_colliders as u64,
            &pt.physbone_colliders,
        ),
    ]
    .into_iter()
    .map(|(stat, value, bounds)| RankItem {
        stat,
        value,
        rank: rank_value(value, bounds),
    })
    .collect();

    let texture_mb = stats.texture_bytes / (1024 * 1024);
    let texture_rank = rank_value(texture_mb, &pt.texture_memory_mb);

    let overall = items
        .iter()
        .map(|item| item.rank)
        .max()
        .unwrap_or(PerformanceRank::Excellent);
    let overall = overall.max(texture_rank);

    let mut items_with_texture = items;
    items_with_texture.push(RankItem {
        stat: "texture_memory_mb",
        value: texture_mb,
        rank: texture_rank,
    });

    RankReport {
        items: items_with_texture,
        overall,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_thresholds() -> RankThresholds {
        crate::vrchat::rank_thresholds::default_thresholds()
    }

    #[test]
    fn test_rank_value_excellent() {
        let bounds = RankBounds {
            excellent: 10,
            good: 20,
            medium: 30,
            poor: 40,
        };
        assert_eq!(rank_value(0, &bounds), PerformanceRank::Excellent);
        assert_eq!(rank_value(10, &bounds), PerformanceRank::Excellent);
    }

    #[test]
    fn test_rank_value_good() {
        let bounds = RankBounds {
            excellent: 10,
            good: 20,
            medium: 30,
            poor: 40,
        };
        assert_eq!(rank_value(11, &bounds), PerformanceRank::Good);
        assert_eq!(rank_value(20, &bounds), PerformanceRank::Good);
    }

    #[test]
    fn test_rank_value_medium() {
        let bounds = RankBounds {
            excellent: 10,
            good: 20,
            medium: 30,
            poor: 40,
        };
        assert_eq!(rank_value(21, &bounds), PerformanceRank::Medium);
        assert_eq!(rank_value(30, &bounds), PerformanceRank::Medium);
    }

    #[test]
    fn test_rank_value_poor() {
        let bounds = RankBounds {
            excellent: 10,
            good: 20,
            medium: 30,
            poor: 40,
        };
        assert_eq!(rank_value(31, &bounds), PerformanceRank::Poor);
        assert_eq!(rank_value(40, &bounds), PerformanceRank::Poor);
    }

    #[test]
    fn test_rank_value_very_poor() {
        let bounds = RankBounds {
            excellent: 10,
            good: 20,
            medium: 30,
            poor: 40,
        };
        assert_eq!(rank_value(41, &bounds), PerformanceRank::VeryPoor);
        assert_eq!(rank_value(100, &bounds), PerformanceRank::VeryPoor);
    }

    #[test]
    fn test_rank_stats_zeroed_is_excellent() {
        let stats = AvatarStats::default();
        let thresholds = make_thresholds();
        let report = rank_stats(&stats, &thresholds, Platform::Pc);
        assert_eq!(report.overall, PerformanceRank::Excellent);
    }

    #[test]
    fn test_rank_report_contains_texture() {
        let stats = AvatarStats::default();
        let thresholds = make_thresholds();
        let report = rank_stats(&stats, &thresholds, Platform::Pc);
        assert!(report
            .items
            .iter()
            .any(|item| item.stat == "texture_memory_mb"));
    }

    #[test]
    fn test_rank_report_overall_is_worst() {
        let stats = AvatarStats {
            triangles: 100_000,
            ..Default::default()
        };
        let thresholds = make_thresholds();
        let report = rank_stats(&stats, &thresholds, Platform::Pc);
        assert_eq!(report.overall, PerformanceRank::VeryPoor);
    }

    #[test]
    fn test_rank_report_pc_vs_quest() {
        let stats = AvatarStats {
            triangles: 10_000,
            ..Default::default()
        };
        let thresholds = make_thresholds();

        let pc_report = rank_stats(&stats, &thresholds, Platform::Pc);
        let quest_report = rank_stats(&stats, &thresholds, Platform::Quest);

        let pc_triangles = pc_report
            .items
            .iter()
            .find(|i| i.stat == "triangles")
            .unwrap();
        let quest_triangles = quest_report
            .items
            .iter()
            .find(|i| i.stat == "triangles")
            .unwrap();

        assert_eq!(pc_triangles.rank, PerformanceRank::Excellent);
        assert_eq!(quest_triangles.rank, PerformanceRank::Good);
    }

    #[test]
    fn test_performance_rank_ord() {
        let ranks: Vec<PerformanceRank> = vec![
            PerformanceRank::Poor,
            PerformanceRank::Good,
            PerformanceRank::VeryPoor,
            PerformanceRank::Excellent,
            PerformanceRank::Medium,
        ];
        let mut sorted = ranks.clone();
        sorted.sort();
        assert_eq!(sorted[0], PerformanceRank::Excellent);
        assert_eq!(sorted[1], PerformanceRank::Good);
        assert_eq!(sorted[2], PerformanceRank::Medium);
        assert_eq!(sorted[3], PerformanceRank::Poor);
        assert_eq!(sorted[4], PerformanceRank::VeryPoor);
    }

    #[test]
    fn test_texture_bytes_to_mb() {
        let stats = AvatarStats {
            texture_bytes: 50 * 1024 * 1024,
            ..Default::default()
        };
        let thresholds = make_thresholds();
        let report = rank_stats(&stats, &thresholds, Platform::Pc);
        let texture_item = report
            .items
            .iter()
            .find(|i| i.stat == "texture_memory_mb")
            .unwrap();
        assert_eq!(texture_item.value, 50);
        assert_eq!(texture_item.rank, PerformanceRank::Good);
    }

    #[test]
    fn test_spring_maps_to_physbone() {
        let stats = AvatarStats {
            spring_chains: 10,
            spring_transforms: 100,
            spring_colliders: 20,
            ..Default::default()
        };
        let thresholds = make_thresholds();
        let report = rank_stats(&stats, &thresholds, Platform::Pc);

        assert!(report.items.iter().any(|i| i.stat == "physbone_components"));
        assert!(report.items.iter().any(|i| i.stat == "physbone_transforms"));
        assert!(report.items.iter().any(|i| i.stat == "physbone_colliders"));

        let pc = report
            .items
            .iter()
            .find(|i| i.stat == "physbone_components")
            .unwrap();
        assert_eq!(pc.value, 10);

        let pt = report
            .items
            .iter()
            .find(|i| i.stat == "physbone_transforms")
            .unwrap();
        assert_eq!(pt.value, 100);

        let pcol = report
            .items
            .iter()
            .find(|i| i.stat == "physbone_colliders")
            .unwrap();
        assert_eq!(pcol.value, 20);
    }

    #[test]
    fn test_materials_maps_to_material_slots() {
        let stats = AvatarStats {
            materials: 10,
            ..Default::default()
        };
        let thresholds = make_thresholds();
        let report = rank_stats(&stats, &thresholds, Platform::Pc);

        let ms = report
            .items
            .iter()
            .find(|i| i.stat == "material_slots")
            .unwrap();
        assert_eq!(ms.value, 10);
    }
}
