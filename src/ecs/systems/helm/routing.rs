//! Routes a normalized utterance with the loaded runtime: pose table motions first, scored by
//! the raw encoder on the utterance without its slot words, then the trained router.

use crate::ecs::resource::HelmRuntime;
use crate::helm::components::route::HelmMode;
use crate::helm::systems::compose_decision::{decide_compose_motion, ComposeThresholds};
use crate::helm::systems::modifier::strip_slot_terms;
use crate::helm::systems::router::{
    rank_routes, route_utterance, RouterDecision, RouterThresholds, RoutingRequest,
};

pub struct RoutingThresholds {
    pub router: RouterThresholds,
    pub compose: ComposeThresholds,
}

pub struct RoutedUtterance {
    pub decision: RouterDecision,
    pub raw_top_score: Option<f32>,
}

/// What the motions are scored on: the utterance without its slot words, or the whole utterance
/// when nothing else is left.
pub fn motion_query(normalized: &str) -> String {
    let stripped = strip_slot_terms(normalized);
    if stripped.is_empty() {
        normalized.to_string()
    } else {
        stripped
    }
}

pub fn route_with_runtime(
    runtime: &mut HelmRuntime,
    normalized: &str,
    mode: HelmMode,
    thresholds: &RoutingThresholds,
) -> Result<RoutedUtterance, String> {
    let raw_vector = runtime
        .raw_encoder
        .encode(normalized)
        .map_err(|e| format!("raw encoding failed: {}", e))?;
    let raw_ranked = rank_routes(&runtime.raw_index, &raw_vector, mode);
    let raw_top_score = raw_ranked.first().map(|(_, score)| *score);

    let query = motion_query(normalized);
    let motion_ranked = if query == normalized {
        raw_ranked
    } else {
        let query_vector = runtime
            .raw_encoder
            .encode(&query)
            .map_err(|e| format!("raw encoding failed: {}", e))?;
        rank_routes(&runtime.raw_index, &query_vector, mode)
    };
    if let Some(decision) = decide_compose_motion(&motion_ranked, thresholds.compose) {
        return Ok(RoutedUtterance {
            decision,
            raw_top_score,
        });
    }

    let vector = runtime
        .encoder
        .encode(normalized)
        .map_err(|e| format!("encoding failed: {}", e))?;
    let decision = route_utterance(
        RoutingRequest {
            utterance: normalized,
            query_vector: &vector,
            mode,
            raw_top_score,
        },
        &runtime.index,
        thresholds.router,
    );
    Ok(RoutedUtterance {
        decision,
        raw_top_score,
    })
}
