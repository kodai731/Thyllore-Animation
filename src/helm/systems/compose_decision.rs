//! Decides whether an utterance asks for a pose table motion, from the raw encoder's ranking
//! of the slot-stripped utterance over every route of the raw index (trained routes and the
//! motions' label blocks together), before the trained router sees the utterance.

use crate::helm::components::route::Route;
use crate::helm::systems::router::RouterDecision;

pub const DEFAULT_COMPOSE_TAU: f32 = 0.90;
pub const DEFAULT_COMPOSE_MARGIN: f32 = 0.02;

/// `tau` is the lowest raw similarity a motion is taken at, `margin` the lead over the runner-up
/// below which the user picks between the close candidates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComposeThresholds {
    pub tau: f32,
    pub margin: f32,
}

impl Default for ComposeThresholds {
    fn default() -> Self {
        Self {
            tau: DEFAULT_COMPOSE_TAU,
            margin: DEFAULT_COMPOSE_MARGIN,
        }
    }
}

/// `None` leaves the utterance to the trained router: no motion ranks first, or it ranks below `tau`.
pub fn decide_compose_motion(
    ranked: &[(Route, f32)],
    thresholds: ComposeThresholds,
) -> Option<RouterDecision> {
    let &(best, score) = ranked.first()?;
    if !matches!(best, Route::ComposeMotion(_)) || score < thresholds.tau {
        return None;
    }

    let runner_up = ranked.get(1).map_or(f32::NEG_INFINITY, |(_, s)| *s);
    let close_candidates: Vec<(Route, f32)> = ranked
        .iter()
        .take(3)
        .filter(|(_, s)| *s >= thresholds.tau && score - *s < thresholds.margin)
        .copied()
        .collect();
    if score - runner_up < thresholds.margin && close_candidates.len() >= 2 {
        return Some(RouterDecision::Clarify {
            candidates: close_candidates,
        });
    }

    Some(RouterDecision::Accept {
        route: best,
        score,
        needs_confirm: score - runner_up < thresholds.margin,
        raw_near_miss: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helm::components::tool_call::ShotPreset;

    const THRESHOLDS: ComposeThresholds = ComposeThresholds {
        tau: 0.9,
        margin: 0.05,
    };

    fn accepted(decision: Option<RouterDecision>) -> (Route, bool) {
        match decision {
            Some(RouterDecision::Accept {
                route,
                needs_confirm,
                ..
            }) => (route, needs_confirm),
            other => panic!("expected Accept, got {other:?}"),
        }
    }

    #[test]
    fn a_motion_ranked_first_with_a_clear_lead_is_accepted() {
        let ranked = [
            (Route::ComposeMotion("wave"), 0.97),
            (Route::ComposeMotion("bow"), 0.90),
            (Route::PlayAnimation, 0.88),
        ];
        assert_eq!(
            accepted(decide_compose_motion(&ranked, THRESHOLDS)),
            (Route::ComposeMotion("wave"), false)
        );
    }

    #[test]
    fn a_trained_route_ranked_first_goes_to_the_trained_router() {
        let ranked = [
            (Route::PlayAnimation, 0.99),
            (Route::ComposeMotion("wave"), 0.95),
        ];
        assert_eq!(decide_compose_motion(&ranked, THRESHOLDS), None);
    }

    #[test]
    fn a_motion_below_tau_goes_to_the_trained_router() {
        let ranked = [
            (Route::ComposeMotion("wave"), 0.85),
            (Route::PlayAnimation, 0.70),
        ];
        assert_eq!(decide_compose_motion(&ranked, THRESHOLDS), None);
    }

    #[test]
    fn close_candidates_above_tau_are_offered_to_the_user() {
        let ranked = [
            (Route::ComposeMotion("arm_raise"), 0.95),
            (Route::ComposeMotion("knee_raise"), 0.94),
            (Route::CameraShot(ShotPreset::CraneUp), 0.93),
            (Route::ComposeMotion("bow"), 0.80),
        ];
        assert_eq!(
            decide_compose_motion(&ranked, THRESHOLDS),
            Some(RouterDecision::Clarify {
                candidates: ranked[..3].to_vec()
            })
        );
    }

    #[test]
    fn a_narrow_lead_over_a_runner_up_below_tau_is_accepted_with_confirmation() {
        let ranked = [
            (Route::ComposeMotion("wave"), 0.92),
            (Route::ComposeMotion("bow"), 0.89),
        ];
        assert_eq!(
            accepted(decide_compose_motion(&ranked, THRESHOLDS)),
            (Route::ComposeMotion("wave"), true)
        );
    }
}
