//! Embeds the pose table's motion labels so each motion becomes a route of the exemplar index.

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
}
