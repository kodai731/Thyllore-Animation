//! Embeds the pose table's motion labels so each motion becomes a route of the exemplar index.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thyllore_avatar_core::motion::seed::components::pose_table::PoseTable;

use crate::helm::components::route::Route;
use crate::helm::systems::normalize::normalize_utterance;
use crate::helm::systems::router::ExemplarIndex;

/// Appends one `ComposeMotion` block per motion, each label normalized like an utterance and
/// encoded on its own by `encode`, the way the utterance it must match is encoded.
pub fn append_motion_label_routes(
    index: &mut ExemplarIndex,
    table: &'static PoseTable,
    mut encode: impl FnMut(&str) -> anyhow::Result<Vec<f32>>,
) -> Result<(), String> {
    for motion in table.motions() {
        if motion.labels.is_empty() {
            continue;
        }
        let vectors = motion
            .labels
            .iter()
            .map(|label| encode(&normalize_utterance(label)))
            .collect::<anyhow::Result<Vec<Vec<f32>>>>()
            .map_err(|e| format!("failed to encode labels of motion '{}': {e}", motion.name))?;
        index
            .append_route(Route::ComposeMotion(motion.name.as_str()), &vectors)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub const LABEL_CACHE_FILENAME: &str = "motion_label_vectors.json";

/// One encoder's label vectors, keyed by normalized label and valid while `fingerprint` (the
/// encoder's files) is unchanged.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LabelVectorCache {
    fingerprint: String,
    vectors: BTreeMap<String, Vec<f32>>,
}

impl LabelVectorCache {
    /// A missing, unreadable or stale file reads as an empty cache, so every label is encoded.
    pub fn read(path: &Path, fingerprint: &str) -> Self {
        let cached = std::fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str::<Self>(&json).ok());
        match cached {
            Some(cache) if cache.fingerprint == fingerprint => cache,
            _ => Self {
                fingerprint: fingerprint.to_string(),
                vectors: BTreeMap::new(),
            },
        }
    }

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, serde_json::to_string(self)?)
    }
}

/// `append_motion_label_routes` that encodes only the labels `cached` lacks; the returned cache
/// holds exactly the labels the table has now.
pub fn append_motion_label_routes_cached(
    index: &mut ExemplarIndex,
    table: &'static PoseTable,
    cached: &LabelVectorCache,
    mut encode: impl FnMut(&str) -> anyhow::Result<Vec<f32>>,
) -> Result<LabelVectorCache, String> {
    let mut refreshed = LabelVectorCache {
        fingerprint: cached.fingerprint.clone(),
        vectors: BTreeMap::new(),
    };
    append_motion_label_routes(index, table, |label| {
        let vector = match cached.vectors.get(label) {
            Some(vector) => vector.clone(),
            None => encode(label)?,
        };
        refreshed.vectors.insert(label.to_string(), vector.clone());
        Ok(vector)
    })?;
    Ok(refreshed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helm::components::route::HelmMode;
    use crate::helm::systems::router::rank_routes;

    const DIMENSIONS: usize = 4;

    fn unit(values: [f32; DIMENSIONS]) -> Vec<f32> {
        let length = values.iter().map(|v| v * v).sum::<f32>().sqrt();
        values.iter().map(|value| value / length).collect()
    }

    fn one_route_index() -> ExemplarIndex {
        let manifest = format!(
            r#"{{"dimensions":{DIMENSIONS},"routes":[{{"route":"play_animation","exemplars":1}}]}}"#
        );
        let bytes: Vec<u8> = unit([1.0, 0.0, 0.0, 0.0])
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        ExemplarIndex::from_export(&manifest, &bytes).unwrap()
    }

    #[test]
    fn every_motion_gets_a_block_with_one_row_per_label() {
        let mut index = one_route_index();
        let table = PoseTable::builtin();
        append_motion_label_routes(&mut index, table, |label| {
            Ok(unit([0.0, label.len() as f32, 1.0, 0.0]))
        })
        .unwrap();

        let label_count: usize = table.motions().iter().map(|m| m.labels.len()).sum();
        assert_eq!(index.exemplar_count(), 1 + label_count);
        let ranked = rank_routes(&index, &unit([0.0, 1.0, 0.0, 0.0]), HelmMode::AllowEdit);
        assert!(matches!(ranked[0].0, Route::ComposeMotion(_)));
        assert_eq!(
            ranked.len(),
            1 + table.motions().len(),
            "one entry per route, motions included"
        );
    }

    #[test]
    fn an_encoder_failure_names_the_motion() {
        let mut index = one_route_index();
        let error = append_motion_label_routes(&mut index, PoseTable::builtin(), |_| {
            anyhow::bail!("offline")
        })
        .unwrap_err();
        assert!(error.contains("offline"), "{error}");
        assert!(
            error.contains(&PoseTable::builtin().motions()[0].name),
            "{error}"
        );
    }

    fn fake_encode(label: &str) -> anyhow::Result<Vec<f32>> {
        Ok(unit([0.0, label.len() as f32, 1.0, 0.0]))
    }

    #[test]
    fn a_warm_cache_encodes_nothing_and_ranks_like_a_cold_one() {
        let table = PoseTable::builtin();
        let empty = LabelVectorCache::default();

        let mut cold_index = one_route_index();
        let mut encoded = 0;
        let warm = append_motion_label_routes_cached(&mut cold_index, table, &empty, |label| {
            encoded += 1;
            fake_encode(label)
        })
        .unwrap();
        assert!(encoded > 0);

        let mut warm_index = one_route_index();
        let rewarmed = append_motion_label_routes_cached(
            &mut warm_index,
            table,
            &warm,
            |_| -> anyhow::Result<Vec<f32>> { anyhow::bail!("must not encode") },
        )
        .unwrap();
        assert_eq!(rewarmed, warm);

        let query = unit([0.0, 1.0, 0.0, 0.0]);
        assert_eq!(
            rank_routes(&warm_index, &query, HelmMode::AllowEdit),
            rank_routes(&cold_index, &query, HelmMode::AllowEdit)
        );
    }

    #[test]
    fn a_cache_from_another_encoder_reads_as_empty() {
        let path = std::env::temp_dir().join("thyllore_test_label_vector_cache.json");
        let mut index = one_route_index();
        let cache = append_motion_label_routes_cached(
            &mut index,
            PoseTable::builtin(),
            &LabelVectorCache {
                fingerprint: "encoder-a".to_string(),
                vectors: BTreeMap::new(),
            },
            fake_encode,
        )
        .unwrap();
        cache.write(&path).unwrap();

        assert_eq!(LabelVectorCache::read(&path, "encoder-a"), cache);
        assert_eq!(
            LabelVectorCache::read(&path, "encoder-b"),
            LabelVectorCache {
                fingerprint: "encoder-b".to_string(),
                vectors: BTreeMap::new(),
            }
        );
        std::fs::remove_file(&path).unwrap();
    }
}
