#![cfg(test)]

//! E2E integration test: text → ONNX encoder → router index → dispatch → command.
//!
//! Runs the real ONNX encoder against the real exemplar index with real thresholds,
//! exercising the full path from an utterance string to a dispatched UI command.
//!
//! Requires `THYLLORE_ROUTER_MODEL_DIR` pointing at a model directory holding
//! `tokenizer.json`, `onnx/model.onnx` and the three files that export writes, plus
//! `ORT_DYLIB_PATH` resolving to the vendored runtime.
//!
//! A developer-machine test, not a CI one: it needs both the root crate (which CI
//! cannot compile) and a trained encoder that is not in the repository.

use std::path::PathBuf;

use thyllore_animation::ecs::events::UiCommandQueue;
use thyllore_animation::ecs::resource::{load_runtime, HelmRuntime, HierarchyState, TimelineState};
use thyllore_animation::ecs::systems::helm::command::HelmCommand;
use thyllore_animation::ecs::systems::helm::dispatcher::dispatch_tool_call;
use thyllore_animation::ecs::systems::helm::name_resolver::list_entity_names;
use thyllore_animation::ecs::world::{Entity, Name, World};
use thyllore_animation::helm::components::route::{HelmMode, Route, RouteKind};
use thyllore_animation::helm::systems::normalize::normalize_utterance;
use thyllore_animation::helm::systems::resolution::{
    resolve_decision, HelmFeedback, ResolvedAction,
};
use thyllore_animation::helm::systems::router::{
    rank_routes, route_utterance, RouterDecision, RouterThresholds, RoutingRequest,
    DEFAULT_REJECTION_THRESHOLD,
};

const MODEL_DIR_ENV_VAR: &str = "THYLLORE_ROUTER_MODEL_DIR";

fn resolve_model_dir() -> Option<PathBuf> {
    let model_dir = PathBuf::from(std::env::var(MODEL_DIR_ENV_VAR).ok()?);
    model_dir.exists().then_some(model_dir)
}

fn build_world() -> (World, Entity) {
    let mut world = World::new();
    // Spawn a hero entity and insert Name component
    let hero = world.spawn();
    world.insert_component(hero, Name("Hero".to_string()));
    // Insert HierarchyState with the hero selected
    let mut hierarchy = HierarchyState::default();
    hierarchy.selected_entity = Some(hero);
    world.insert_resource(hierarchy);
    // Insert TimelineState
    world.insert_resource(TimelineState::default());
    // Insert ClipLibrary
    world.insert_resource(thyllore_animation::ecs::resource::ClipLibrary::default());
    // Insert command queue (starts empty)
    world.insert_resource(UiCommandQueue::default());
    (world, hero)
}

fn thresholds() -> RouterThresholds {
    RouterThresholds {
        tau_reject: DEFAULT_REJECTION_THRESHOLD,
        delta: 0.0,
        tau_confirm: 0.0,
        tau_raw: 0.90,
        tau_raw_nearmiss: 0.0,
    }
}

#[test]
fn e2e_text_to_ui_event_full_path() {
    let Some(model_dir) = resolve_model_dir() else {
        eprintln!(
            "Skipping: set {MODEL_DIR_ENV_VAR} to a model directory prepared by \
            scripts/helm_eval/export_router_index.py"
        );
        return;
    };

    // Load runtime (encoder + index + hash consistency checks)
    let mut runtime: HelmRuntime = load_runtime(&model_dir).expect("load_runtime must succeed");

    // Spawn a named entity "Hero" in the world
    let mut world = World::new();
    let hero_entity = world.spawn();
    world.insert_component(hero_entity, Name("Hero".to_string()));

    let thresholds = thresholds();

    // Case (1): AllowEdit + confirm_all=false + "play the animation"
    // -> Accept { route: PlayAnimation } -> AwaitConfirm { call: PlayAnimation, reason: Mutating }
    // -> dispatch_tool_call (user confirmation) returns Command(Play) -> queue len increases
    {
        let (world, hero_entity) = build_world();
        let utterance = "play the animation";
        let normalized = normalize_utterance(utterance);
        let query = runtime
            .encoder
            .encode(utterance)
            .expect("encoder must produce a vector");

        let decision = route_utterance(
            RoutingRequest {
                utterance: &normalized,
                query_vector: &query,
                mode: HelmMode::AllowEdit,
                raw_top_score: None,
            },
            &runtime.index,
            thresholds,
        );

        match &decision {
            RouterDecision::Accept { route, score, .. } => {
                assert!(
                    matches!(route, Route::PlayAnimation),
                    "expected PlayAnimation route, got {:?} (score={:.4})",
                    route,
                    score
                );
            }
            other => panic!("expected Accept decision, got {:?}", other),
        }

        let scene_names = list_entity_names(&world);
        let action = resolve_decision(decision, &normalized, &scene_names, false);

        let tool_call = match action {
            ResolvedAction::AwaitConfirm { call, reason } => {
                assert!(
                    matches!(
                        call,
                        thyllore_animation::helm::components::tool_call::ToolCall::PlayAnimation
                    ),
                    "expected PlayAnimation call in AwaitConfirm, got {:?}",
                    call
                );
                assert_eq!(
                    reason,
                    thyllore_animation::helm::systems::resolution::ConfirmReason::Mutating,
                    "expected Mutating reason"
                );
                call
            }
            other => panic!("expected AwaitConfirm action, got {:?}", other),
        };

        let queue_len_before = world.get_resource::<UiCommandQueue>().unwrap().len();
        let outcome = dispatch_tool_call(
            &world,
            &thyllore_animation::helm::systems::seek::TimelineContext::default(),
            &tool_call,
        );

        match outcome {
            thyllore_animation::ecs::systems::helm::dispatcher::DispatchOutcome::Command(event) => {
                assert!(
                    matches!(event, HelmCommand::Play),
                    "expected Play command, got {:?}",
                    event
                );
                world.send_command(event);
            }
            other => panic!("expected Command outcome, got {:?}", other),
        }

        let queue_len_after = world.get_resource::<UiCommandQueue>().unwrap().len();
        assert!(
            queue_len_after > queue_len_before,
            "UiCommandQueue length should increase after dispatch"
        );
    }

    // Case (2): Same conditions + Japanese "アニメーションを再生して"
    // -> Accept { route: PlayAnimation }
    {
        let utterance = "アニメーションを再生して";
        let normalized = normalize_utterance(utterance);
        let query = runtime
            .encoder
            .encode(utterance)
            .expect("encoder must produce a vector");

        let decision = route_utterance(
            RoutingRequest {
                utterance: &normalized,
                query_vector: &query,
                mode: HelmMode::AllowEdit,
                raw_top_score: None,
            },
            &runtime.index,
            thresholds,
        );

        match &decision {
            RouterDecision::Accept { route, score, .. } => {
                assert!(
                    matches!(route, Route::PlayAnimation),
                    "expected PlayAnimation route for Japanese utterance, got {:?} (score={:.4})",
                    route,
                    score
                );
            }
            other => panic!(
                "expected Accept decision for Japanese utterance, got {:?}",
                other
            ),
        }
    }

    // Case (3): AllowEdit + confirm_all=false + scene contains "Hero" + "select the hero"
    // -> AwaitConfirm { call: SelectObject(ObjectName("Hero")), reason: Mutating }
    // -> dispatch_tool_call (user confirmation) returns Command(SelectEntity(hero_entity))
    {
        let (world, hero_entity) = build_world();
        let utterance = "select the hero";
        let normalized = normalize_utterance(utterance);
        let query = runtime
            .encoder
            .encode(utterance)
            .expect("encoder must produce a vector");

        let decision = route_utterance(
            RoutingRequest {
                utterance: &normalized,
                query_vector: &query,
                mode: HelmMode::AllowEdit,
                raw_top_score: None,
            },
            &runtime.index,
            thresholds,
        );

        match &decision {
            RouterDecision::Accept { route, score, .. } => {
                assert!(
                    matches!(route, Route::SelectObject),
                    "expected SelectObject route, got {:?} (score={:.4})",
                    route,
                    score
                );
            }
            other => panic!("expected Accept decision, got {:?}", other),
        }

        let scene_names = list_entity_names(&world);
        let action = resolve_decision(decision, &normalized, &scene_names, false);

        let tool_call = match action {
            ResolvedAction::AwaitConfirm { call, reason } => {
                assert!(
                    matches!(
                        call,
                        thyllore_animation::helm::components::tool_call::ToolCall::SelectObject(
                            thyllore_animation::helm::components::tool_call::ObjectName(ref name)
                        ) if name == "Hero"
                    ),
                    "expected SelectObject(ObjectName(\"Hero\")) in AwaitConfirm, got {:?}",
                    call
                );
                assert_eq!(
                    reason,
                    thyllore_animation::helm::systems::resolution::ConfirmReason::Mutating,
                    "expected Mutating reason"
                );
                call
            }
            other => panic!("expected AwaitConfirm action, got {:?}", other),
        };

        let queue_len_before = world.get_resource::<UiCommandQueue>().unwrap().len();
        let outcome = dispatch_tool_call(
            &world,
            &thyllore_animation::helm::systems::seek::TimelineContext::default(),
            &tool_call,
        );

        match outcome {
            thyllore_animation::ecs::systems::helm::dispatcher::DispatchOutcome::Command(event) => {
                assert!(
                    matches!(event, HelmCommand::SelectEntity(e) if e == hero_entity),
                    "expected SelectEntity(hero_entity) command, got {:?}",
                    event
                );
                world.send_command(event);
            }
            other => panic!("expected Command outcome, got {:?}", other),
        }

        let queue_len_after = world.get_resource::<UiCommandQueue>().unwrap().len();
        assert!(
            queue_len_after > queue_len_before,
            "UiCommandQueue length should increase after dispatch"
        );
    }

    // Case (4): AllowEdit + confirm_all=true + "play the animation"
    // -> ResolvedAction::AwaitConfirm { .. } (safety gate)
    {
        let utterance = "play the animation";
        let normalized = normalize_utterance(utterance);
        let query = runtime
            .encoder
            .encode(utterance)
            .expect("encoder must produce a vector");

        let decision = route_utterance(
            RoutingRequest {
                utterance: &normalized,
                query_vector: &query,
                mode: HelmMode::AllowEdit,
                raw_top_score: None,
            },
            &runtime.index,
            thresholds,
        );

        let scene_names = list_entity_names(&world);
        let action = resolve_decision(decision, &normalized, &scene_names, true);

        match action {
            ResolvedAction::AwaitConfirm { call, reason } => {
                assert!(
                    matches!(
                        call,
                        thyllore_animation::helm::components::tool_call::ToolCall::PlayAnimation
                    ),
                    "expected PlayAnimation call in AwaitConfirm, got {:?}",
                    call
                );
                assert_eq!(
                    reason,
                    thyllore_animation::helm::systems::resolution::ConfirmReason::ConfirmAll,
                    "expected ConfirmAll reason"
                );
            }
            other => panic!(
                "expected AwaitConfirm action with confirm_all=true, got {:?}",
                other
            ),
        }
    }

    // Case (5): AllowEdit + confirm_all=false + "list the objects in the scene"
    // -> Accept { route: ListObjects } -> Dispatch(ToolCall::ListObjects)
    // -> dispatch_tool_call returns Report(_) (read_only, no confirmation needed)
    {
        let utterance = "list the objects in the scene";
        let normalized = normalize_utterance(utterance);
        let query = runtime
            .encoder
            .encode(utterance)
            .expect("encoder must produce a vector");

        let decision = route_utterance(
            RoutingRequest {
                utterance: &normalized,
                query_vector: &query,
                mode: HelmMode::AllowEdit,
                raw_top_score: None,
            },
            &runtime.index,
            thresholds,
        );

        match &decision {
            RouterDecision::Accept { route, score, .. } => {
                assert!(
                    matches!(route, Route::ListObjects),
                    "expected ListObjects route, got {:?} (score={:.4})",
                    route,
                    score
                );
            }
            other => panic!("expected Accept decision, got {:?}", other),
        }

        let scene_names = list_entity_names(&world);
        let action = resolve_decision(decision, &normalized, &scene_names, false);

        let tool_call = match action {
            ResolvedAction::Dispatch(call) => call,
            other => panic!("expected Dispatch action, got {:?}", other),
        };

        assert!(
            matches!(
                tool_call,
                thyllore_animation::helm::components::tool_call::ToolCall::ListObjects
            ),
            "expected ListObjects tool call"
        );

        let outcome = dispatch_tool_call(
            &world,
            &thyllore_animation::helm::systems::seek::TimelineContext::default(),
            &tool_call,
        );

        match outcome {
            thyllore_animation::ecs::systems::helm::dispatcher::DispatchOutcome::Report(_) => {}
            other => panic!(
                "expected Report outcome for read-only ListObjects, got {:?}",
                other
            ),
        }
    }

    // Case (6): Irrelevant utterance "what is the weather today"
    // -> RouterDecision::Reject { .. } via raw gate with production thresholds
    {
        let utterance = "what is the weather today";
        let normalized = normalize_utterance(utterance);
        let query = runtime
            .encoder
            .encode(&normalized)
            .expect("encoder must produce a vector");
        let raw_query = runtime
            .raw_encoder
            .encode(&normalized)
            .expect("raw encoder must produce a vector");
        let raw_ranked = rank_routes(&runtime.raw_index, &raw_query, HelmMode::AllowEdit);
        let raw_top = raw_ranked
            .first()
            .map(|(_, s)| *s)
            .unwrap_or(f32::NEG_INFINITY);

        let decision = route_utterance(
            RoutingRequest {
                utterance: &normalized,
                query_vector: &query,
                mode: HelmMode::AllowEdit,
                raw_top_score: Some(raw_top),
            },
            &runtime.index,
            RouterThresholds {
                tau_reject: 0.93,
                delta: 0.005,
                tau_confirm: 0.95,
                tau_raw: 0.90,
                tau_raw_nearmiss: 0.90,
            },
        );

        assert!(
            matches!(decision, RouterDecision::Reject { .. }),
            "production thresholds must reject out-of-domain utterance via raw gate, got {:?}",
            decision
        );
    }

    // Case (7): HelmMode::ReadOnly
    // -> rank_routes results all have route.kind() == RouteKind::ReadOnly
    {
        let utterance = "play the animation";
        let query = runtime
            .encoder
            .encode(utterance)
            .expect("encoder must produce a vector");

        let ranked = rank_routes(&runtime.index, &query, HelmMode::ReadOnly);

        assert!(
            !ranked.is_empty(),
            "rank_routes should return at least one route in ReadOnly mode"
        );

        for (route, _) in &ranked {
            assert_eq!(
                route.kind(),
                RouteKind::ReadOnly,
                "ReadOnly mode should only return ReadOnly routes, got {:?} which is {:?}",
                route,
                route.kind()
            );
        }
    }
}

/// Sentences that match a pose-table label verbatim are accepted (or clarified against a
/// near-tie) through the label routes the runtime embeds at load, and their modifiers bind.
/// Paraphrases with modifiers only have to keep their motion in the top three: acceptance
/// under the bundle's thresholds is reported, not asserted, because those thresholds were tuned
/// on the exported exemplars and the labels are a far smaller sample.
#[test]
fn e2e_gesture_sentences_reach_their_motion() {
    use thyllore_animation::ecs::resource::{apply_encoder_thresholds, HelmState};
    use thyllore_animation::helm::components::tool_call::ToolCall;
    use thyllore_animation::helm::systems::binder::{bind_route, BindOutcome};
    use thyllore_avatar_core::motion::seed::components::motion_spec::MotionSide;

    let Some(model_dir) = resolve_model_dir() else {
        eprintln!("Skipping: set {MODEL_DIR_ENV_VAR} to a router model directory");
        return;
    };
    let load_started = std::time::Instant::now();
    let mut runtime = load_runtime(&model_dir).expect("load_runtime must succeed");
    eprintln!(
        "runtime loaded in {:.1}s with {} exemplars",
        load_started.elapsed().as_secs_f32(),
        runtime.index.exemplar_count()
    );
    let encode_started = std::time::Instant::now();
    for _ in 0..10 {
        runtime.encoder.encode("右手を振って").unwrap();
    }
    eprintln!(
        "one single-label encode: {:.1}ms",
        encode_started.elapsed().as_secs_f32() * 100.0
    );
    let mut thresholds = HelmState::default().thresholds;
    apply_encoder_thresholds(&mut thresholds, &runtime.thresholds);

    #[derive(Clone, Copy)]
    enum Expect {
        Accepted,
        InTopThree,
    }
    let cases: [(&str, &str, MotionSide, u32, Expect); 8] = [
        (
            "右手を振って",
            "wave",
            MotionSide::Right,
            1,
            Expect::Accepted,
        ),
        ("お辞儀して", "bow", MotionSide::Right, 1, Expect::Accepted),
        ("頷いて", "nod", MotionSide::Right, 1, Expect::Accepted),
        (
            "しゃがんで",
            "crouch",
            MotionSide::Right,
            1,
            Expect::Accepted,
        ),
        (
            "腕を上げて",
            "arm_raise",
            MotionSide::Right,
            1,
            Expect::Accepted,
        ),
        (
            "左手を2回振って",
            "wave",
            MotionSide::Left,
            2,
            Expect::InTopThree,
        ),
        (
            "左で正拳突き",
            "punch",
            MotionSide::Left,
            1,
            Expect::InTopThree,
        ),
        (
            "throw a straight punch",
            "punch",
            MotionSide::Right,
            1,
            Expect::InTopThree,
        ),
    ];

    let mut failures = Vec::new();
    let mut accepted = 0;
    for (utterance, motion, side, count, expect) in cases {
        let normalized = normalize_utterance(utterance);
        let query = runtime.encoder.encode(&normalized).unwrap();
        let raw_query = runtime.raw_encoder.encode(&normalized).unwrap();
        let raw_top_score = rank_routes(&runtime.raw_index, &raw_query, HelmMode::AllowEdit)
            .first()
            .map(|(_, score)| *score);
        let ranked = rank_routes(&runtime.index, &query, HelmMode::AllowEdit);
        let decision = route_utterance(
            RoutingRequest {
                utterance: &normalized,
                query_vector: &query,
                mode: HelmMode::AllowEdit,
                raw_top_score,
            },
            &runtime.index,
            thresholds,
        );
        eprintln!(
            "{utterance:>24} -> {decision:?} | raw_top={:.3} | top3={:?}",
            raw_top_score.unwrap_or(0.0),
            ranked
                .iter()
                .take(3)
                .map(|(r, s)| format!("{}={s:.3}", r.id()))
                .collect::<Vec<_>>()
        );

        let expected = Route::ComposeMotion(motion);
        let reached = match (&decision, expect) {
            (RouterDecision::Accept { route, .. }, Expect::Accepted) => *route == expected,
            (RouterDecision::Clarify { candidates }, Expect::Accepted) => {
                candidates.iter().any(|(r, _)| *r == expected)
            }
            (_, Expect::InTopThree) => ranked.iter().take(3).any(|(r, _)| *r == expected),
            (RouterDecision::Reject { .. } | RouterDecision::NoCandidate, Expect::Accepted) => {
                false
            }
        };
        if matches!(decision, RouterDecision::Accept { .. }) {
            accepted += 1;
        }
        if !reached {
            failures.push(format!("{utterance}: {decision:?}"));
            continue;
        }
        match bind_route(expected, &normalized, &[]) {
            BindOutcome::Call(ToolCall::ComposeMotion(request)) => {
                if request.side != side || request.count != count {
                    failures.push(format!("{utterance}: bound {request:?}"));
                }
            }
            other => failures.push(format!("{utterance}: bound {other:?}")),
        }
    }
    eprintln!(
        "{accepted}/{} gesture sentences accepted at tau_reject={}",
        cases.len(),
        thresholds.tau_reject
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Padded batch encoding of the setfit export drifts from single encoding (cosine 0.96 on a
/// short label), so the runtime encodes labels one at a time. Ignored until the export is fixed.
#[test]
#[ignore = "setfit ONNX export is not padding invariant; labels are encoded singly instead"]
fn e2e_batch_encoding_matches_single_encoding() {
    let Some(model_dir) = resolve_model_dir() else {
        eprintln!("Skipping: set {MODEL_DIR_ENV_VAR} to a router model directory");
        return;
    };
    let mut runtime = load_runtime(&model_dir).expect("load_runtime must succeed");
    let labels = ["右手を振って", "wave at the camera", "バイバイ"];

    let batch = runtime.encoder.encode_batch(&labels).unwrap();
    for (label, batched) in labels.iter().zip(&batch) {
        let single = runtime.encoder.encode(label).unwrap();
        let cosine: f32 = single.iter().zip(batched).map(|(a, b)| a * b).sum();
        eprintln!("{label:>20}: batch vs single cosine = {cosine:.6}");
        assert!(
            cosine > 0.999,
            "{label}: batch and single encodings differ ({cosine})"
        );
    }
}
