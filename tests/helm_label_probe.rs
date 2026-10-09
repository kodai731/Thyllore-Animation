#![cfg(test)]

//! Measurement probe, not a regression test: how paraphrased gesture sentences with side /
//! count words score against the pose table's labels under different query and label
//! treatments. Run with `--ignored --nocapture` and the router model directory set.

use std::collections::{BTreeMap, HashSet};

use thyllore_animation::ecs::resource::{apply_encoder_thresholds, load_runtime, HelmState};
use thyllore_animation::helm::components::route::{HelmMode, Route};
use thyllore_animation::helm::systems::normalize::normalize_utterance;
use thyllore_animation::helm::systems::router::{
    rank_routes, route_utterance, ExemplarIndex, RouterDecision, RouterThresholds, RoutingRequest,
};
use thyllore_avatar_core::motion::seed::components::pose_table::PoseTable;
use thyllore_ml_core::sentence_encoder::SentenceEncoder;

const MODEL_DIR_ENV_VAR: &str = "THYLLORE_ROUTER_MODEL_DIR";
const ACCEPT_TAU: f32 = 0.98;

const CASES: &[(&str, &str)] = &[
    ("左手を2回振って", "wave"),
    ("手を三回振る", "wave"),
    ("右手で大きく手を振ってください", "wave"),
    ("バイバイして", "wave"),
    ("wave the left hand twice", "wave"),
    ("wave your right hand three times", "wave"),
    ("say goodbye with your hand", "wave"),
    ("左で正拳突き", "punch"),
    ("右で2回パンチして", "punch"),
    ("拳を前に突き出して", "punch"),
    ("throw a straight punch", "punch"),
    ("punch twice with the left hand", "punch"),
    ("深くお辞儀して", "bow"),
    ("二回礼をして", "bow"),
    ("頭を下げてお辞儀", "bow"),
    ("bow twice", "bow"),
    ("bow deeply", "bow"),
    ("2回頷いて", "nod"),
    ("首を縦に振って", "nod"),
    ("nod twice", "nod"),
    ("nod your head three times", "nod"),
    ("左腕を上げて", "arm_raise"),
    ("右手を高く上げる", "arm_raise"),
    ("raise your left arm", "arm_raise"),
    ("put your right hand up", "arm_raise"),
    ("左を向いて", "head_turn"),
    ("右に振り向く", "head_turn"),
    ("turn your head to the left", "head_turn"),
    ("look to the right", "head_turn"),
    ("左膝を上げて", "knee_raise"),
    ("右の膝を3回上げる", "knee_raise"),
    ("lift your left knee twice", "knee_raise"),
    ("左足で前蹴り", "front_kick"),
    ("右で2回蹴って", "front_kick"),
    ("kick forward with the left leg", "front_kick"),
    ("do two front kicks", "front_kick"),
    ("左手で拳を握って", "grip"),
    ("右手をグーにする", "grip"),
    ("clench your left fist", "grip"),
    ("左手を開いて", "open_hand"),
    ("右手をパーにして", "open_hand"),
    ("open your left hand", "open_hand"),
    ("深くしゃがんで", "crouch"),
    ("2回しゃがむ", "crouch"),
    ("crouch down twice", "crouch"),
    ("squat down low", "crouch"),
];

const SIDE_REPLACEMENTS: &[(&str, &str)] = &[
    ("左手", "手"),
    ("右手", "手"),
    ("左足", "足"),
    ("右足", "足"),
    ("左腕", "腕"),
    ("右腕", "腕"),
    ("左膝", "膝"),
    ("右膝", "膝"),
    ("左の", ""),
    ("右の", ""),
    ("左で", ""),
    ("右で", ""),
    ("左を", ""),
    ("右を", ""),
    ("左に", ""),
    ("右に", ""),
    ("左", ""),
    ("右", ""),
    ("left ", ""),
    ("right ", ""),
    (" left", ""),
    (" right", ""),
];

const COUNT_TERMS: &[&str] = &[
    "three times",
    "two times",
    "twice",
    "二回",
    "三回",
    "四回",
    "二度",
    "三度",
    "もう一度",
];

const DEGREE_TERMS: &[&str] = &[
    "大きく",
    "深く",
    "高く",
    "低く",
    "ゆっくり",
    "速く",
    "してください",
    "ください",
    "deeply",
    "slowly",
    "quickly",
    "high",
    "low",
    "please",
];

const SIDE_TEMPLATES_JA: &[&str] = &["左手で{}", "右手で{}", "左で{}", "右で{}"];
const COUNT_TEMPLATES_JA: &[&str] = &["{}を2回", "2回{}", "3回{}", "もう一度{}"];
const SIDE_TEMPLATES_EN: &[&str] = &[
    "{} with the left hand",
    "{} with the right hand",
    "left {}",
    "right {}",
];
const COUNT_TEMPLATES_EN: &[&str] = &["{} twice", "{} three times", "{} 2 times"];

#[derive(Clone, Copy)]
enum Strip {
    None,
    Side,
    SideCount,
    SideCountDegree,
}

fn strip_digit_counts(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            while chars.peek().is_some_and(|n| n.is_ascii_digit()) {
                chars.next();
            }
            if matches!(chars.peek(), Some('回') | Some('度')) {
                chars.next();
            }
            out.push(' ');
            continue;
        }
        out.push(c);
    }
    out
}

fn strip_modifiers(normalized: &str, strip: Strip) -> String {
    let mut text = normalized.to_string();
    if matches!(
        strip,
        Strip::Side | Strip::SideCount | Strip::SideCountDegree
    ) {
        for (from, to) in SIDE_REPLACEMENTS {
            text = text.replace(from, to);
        }
    }
    if matches!(strip, Strip::SideCount | Strip::SideCountDegree) {
        for term in COUNT_TERMS {
            text = text.replace(term, " ");
        }
        text = strip_digit_counts(&text);
    }
    if matches!(strip, Strip::SideCountDegree) {
        for term in DEGREE_TERMS {
            text = text.replace(term, " ");
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_ascii_text(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii())
}

fn char_bigrams(s: &str) -> HashSet<String> {
    let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    chars.windows(2).map(|w| w.iter().collect()).collect()
}

fn dice(a: &str, b: &str) -> f32 {
    let (ga, gb) = (char_bigrams(a), char_bigrams(b));
    if ga.is_empty() || gb.is_empty() {
        return 0.0;
    }
    2.0 * ga.intersection(&gb).count() as f32 / (ga.len() + gb.len()) as f32
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

struct LabelSet {
    rows: Vec<(String, String)>,
}

impl LabelSet {
    fn base(table: &PoseTable) -> Self {
        let rows = table
            .motions()
            .iter()
            .flat_map(|m| {
                m.labels
                    .iter()
                    .map(|l| (m.name.clone(), normalize_utterance(l)))
            })
            .collect();
        Self { rows }
    }

    fn templated(table: &PoseTable) -> Self {
        let mut rows = Self::base(table).rows;
        for motion in table.motions() {
            for label in &motion.labels {
                let normalized = normalize_utterance(label);
                let (sides, counts) = if is_ascii_text(&normalized) {
                    (SIDE_TEMPLATES_EN, COUNT_TEMPLATES_EN)
                } else {
                    (SIDE_TEMPLATES_JA, COUNT_TEMPLATES_JA)
                };
                for template in sides.iter().chain(counts) {
                    rows.push((motion.name.clone(), template.replace("{}", &normalized)));
                }
            }
        }
        Self { rows }
    }
}

struct Embedded {
    rows: Vec<(String, String, Vec<f32>)>,
}

fn embed(set: &LabelSet, encoder: &mut SentenceEncoder) -> Embedded {
    let rows = set
        .rows
        .iter()
        .map(|(motion, text)| (motion.clone(), text.clone(), encoder.encode(text).unwrap()))
        .collect();
    Embedded { rows }
}

#[derive(Clone, Copy)]
enum Pool {
    Max,
    MeanTop3,
}

fn score_motions(
    embedded: &Embedded,
    query: &[f32],
    query_text: &str,
    pool: Pool,
    lexical_weight: f32,
) -> Vec<(String, f32)> {
    let mut per_motion: BTreeMap<&str, Vec<f32>> = BTreeMap::new();
    for (motion, text, vector) in &embedded.rows {
        let cosine = dot(vector, query);
        let score = (1.0 - lexical_weight) * cosine + lexical_weight * dice(text, query_text);
        per_motion.entry(motion).or_default().push(score);
    }
    let mut ranked: Vec<(String, f32)> = per_motion
        .into_iter()
        .map(|(motion, mut scores)| {
            scores.sort_by(|a, b| b.total_cmp(a));
            let value = match pool {
                Pool::Max => scores[0],
                Pool::MeanTop3 => {
                    let n = scores.len().min(3);
                    scores[..n].iter().sum::<f32>() / n as f32
                }
            };
            (motion.to_string(), value)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked
}

struct Summary {
    top1: usize,
    accepted: usize,
    mean_correct: f32,
    mean_margin: f32,
    misses: Vec<String>,
}

fn evaluate(
    embedded: &Embedded,
    encoder: &mut SentenceEncoder,
    strip: Strip,
    pool: Pool,
    lexical_weight: f32,
) -> Summary {
    let mut summary = Summary {
        top1: 0,
        accepted: 0,
        mean_correct: 0.0,
        mean_margin: 0.0,
        misses: Vec::new(),
    };
    for (utterance, expected) in CASES {
        let query_text = strip_modifiers(&normalize_utterance(utterance), strip);
        let query = encoder.encode(&query_text).unwrap();
        let ranked = score_motions(embedded, &query, &query_text, pool, lexical_weight);
        let correct = ranked
            .iter()
            .find(|(m, _)| m == expected)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        let best_other = ranked
            .iter()
            .find(|(m, _)| m != expected)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        summary.mean_correct += correct;
        summary.mean_margin += correct - best_other;
        if ranked[0].0 == *expected {
            summary.top1 += 1;
            if correct >= ACCEPT_TAU {
                summary.accepted += 1;
            }
        } else {
            summary.misses.push(format!(
                "{utterance} -> {}={:.3} (want {expected}={correct:.3})",
                ranked[0].0, ranked[0].1
            ));
        }
    }
    summary.mean_correct /= CASES.len() as f32;
    summary.mean_margin /= CASES.len() as f32;
    summary
}

fn report(name: &str, summary: &Summary) {
    eprintln!(
        "{name:<48} top1 {:>2}/{} | >= {ACCEPT_TAU}: {:>2} | mean correct {:.3} | mean margin {:+.3}",
        summary.top1,
        CASES.len(),
        summary.accepted,
        summary.mean_correct,
        summary.mean_margin
    );
    for miss in &summary.misses {
        eprintln!("    miss: {miss}");
    }
}

#[test]
#[ignore = "measurement probe; needs the router model directory"]
fn probe_paraphrase_robustness() {
    let Ok(model_dir) = std::env::var(MODEL_DIR_ENV_VAR) else {
        eprintln!("Skipping: set {MODEL_DIR_ENV_VAR}");
        return;
    };
    let mut runtime = load_runtime(std::path::Path::new(&model_dir)).unwrap();
    let table = PoseTable::builtin();
    let base = LabelSet::base(table);
    let templated = LabelSet::templated(table);
    eprintln!(
        "labels: base {} rows, templated {} rows",
        base.rows.len(),
        templated.rows.len()
    );

    for (encoder_name, encoder) in [
        ("setfit", &mut runtime.encoder),
        ("raw(ruri-v3-310m)", &mut runtime.raw_encoder),
    ] {
        let started = std::time::Instant::now();
        let base_embedded = embed(&base, encoder);
        let templated_embedded = embed(&templated, encoder);
        eprintln!(
            "\n== {encoder_name} (labels embedded in {:.1}s)",
            started.elapsed().as_secs_f32()
        );

        let configs: [(&str, &Embedded, Strip, Pool, f32); 9] = [
            (
                "A  base labels, raw query, max",
                &base_embedded,
                Strip::None,
                Pool::Max,
                0.0,
            ),
            (
                "B1 base labels, strip side, max",
                &base_embedded,
                Strip::Side,
                Pool::Max,
                0.0,
            ),
            (
                "B2 base labels, strip side+count, max",
                &base_embedded,
                Strip::SideCount,
                Pool::Max,
                0.0,
            ),
            (
                "B3 base labels, strip side+count+degree, max",
                &base_embedded,
                Strip::SideCountDegree,
                Pool::Max,
                0.0,
            ),
            (
                "C  templated labels, raw query, max",
                &templated_embedded,
                Strip::None,
                Pool::Max,
                0.0,
            ),
            (
                "D  templated labels, strip all, max",
                &templated_embedded,
                Strip::SideCountDegree,
                Pool::Max,
                0.0,
            ),
            (
                "E  base labels, strip all, mean top3",
                &base_embedded,
                Strip::SideCountDegree,
                Pool::MeanTop3,
                0.0,
            ),
            (
                "F  base, strip all, max + 0.3 char-bigram dice",
                &base_embedded,
                Strip::SideCountDegree,
                Pool::Max,
                0.3,
            ),
            (
                "G  templated, strip all, max + 0.3 dice",
                &templated_embedded,
                Strip::SideCountDegree,
                Pool::Max,
                0.3,
            ),
        ];
        for (name, embedded, strip, pool, lexical) in configs {
            report(name, &evaluate(embedded, encoder, strip, pool, lexical));
        }
    }
}

struct FullRun {
    top1: usize,
    accepted: usize,
    margin: f32,
    misses: Vec<String>,
}

fn run_full_router(
    encoder: &mut SentenceEncoder,
    index: &ExemplarIndex,
    strip: Strip,
    mut accept: Option<(&mut SentenceEncoder, &ExemplarIndex, RouterThresholds)>,
) -> FullRun {
    let mut run = FullRun {
        top1: 0,
        accepted: 0,
        margin: 0.0,
        misses: Vec::new(),
    };
    for (utterance, expected) in CASES {
        let query_text = strip_modifiers(&normalize_utterance(utterance), strip);
        let query = encoder.encode(&query_text).unwrap();
        let ranked = rank_routes(index, &query, HelmMode::AllowEdit);
        let want = Route::ComposeMotion(expected);
        let correct = ranked
            .iter()
            .find(|(r, _)| *r == want)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        let other = ranked
            .iter()
            .find(|(r, _)| *r != want)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        run.margin += correct - other;
        if ranked[0].0 == want {
            run.top1 += 1;
        } else {
            run.misses.push(format!(
                "{utterance} -> {}={:.3} (want {expected}={correct:.3})",
                ranked[0].0.id(),
                ranked[0].1
            ));
        }
        if let Some((raw_encoder, raw_index, thresholds)) = accept.as_mut() {
            let raw_query = raw_encoder.encode(&query_text).unwrap();
            let raw_top = rank_routes(raw_index, &raw_query, HelmMode::AllowEdit)
                .first()
                .map(|(_, s)| *s);
            let decision = route_utterance(
                RoutingRequest {
                    utterance: &query_text,
                    query_vector: &query,
                    mode: HelmMode::AllowEdit,
                    raw_top_score: raw_top,
                },
                index,
                *thresholds,
            );
            if matches!(decision, RouterDecision::Accept { route, .. } if route == want) {
                run.accepted += 1;
            }
        }
    }
    run.margin /= CASES.len() as f32;
    run
}

fn report_full(name: &str, run: &FullRun) {
    eprintln!(
        "{name:<40} top1 {:>2}/{} | accepted {:>2} | mean margin {:+.3}",
        run.top1,
        CASES.len(),
        run.accepted,
        run.margin
    );
    for miss in &run.misses {
        eprintln!("    miss: {miss}");
    }
}

/// The production ranking: label blocks compete with the 36 trained routes inside each index.
#[test]
#[ignore = "measurement probe; needs the router model directory"]
fn probe_full_router_ranking() {
    let Ok(model_dir) = std::env::var(MODEL_DIR_ENV_VAR) else {
        eprintln!("Skipping: set {MODEL_DIR_ENV_VAR}");
        return;
    };
    let mut runtime = load_runtime(std::path::Path::new(&model_dir)).unwrap();
    let mut thresholds = HelmState::default().thresholds;
    apply_encoder_thresholds(&mut thresholds, &runtime.thresholds);

    eprintln!("== setfit index (36 trained routes + label blocks), production thresholds");
    for (name, strip) in [
        ("raw query", Strip::None),
        ("strip side+count+degree", Strip::SideCountDegree),
    ] {
        let run = run_full_router(
            &mut runtime.encoder,
            &runtime.index,
            strip,
            Some((&mut runtime.raw_encoder, &runtime.raw_index, thresholds)),
        );
        report_full(name, &run);
    }

    eprintln!("== raw index (36 trained routes + label blocks), ranking only");
    for (name, strip) in [
        ("raw query", Strip::None),
        ("strip side+count+degree", Strip::SideCountDegree),
    ] {
        let run = run_full_router(&mut runtime.raw_encoder, &runtime.raw_index, strip, None);
        report_full(name, &run);
    }
}

/// Negative side: the smoke utterances of the trained routes must keep their route when the
/// label blocks are present and when the query is stripped.
#[test]
#[ignore = "measurement probe; needs the router model directory"]
fn probe_trained_routes_keep_winning() {
    let Ok(model_dir) = std::env::var(MODEL_DIR_ENV_VAR) else {
        eprintln!("Skipping: set {MODEL_DIR_ENV_VAR}");
        return;
    };
    let mut runtime = load_runtime(std::path::Path::new(&model_dir)).unwrap();
    let smoke = std::fs::read_to_string("tests/data/helm_batch_smoke.jsonl").unwrap();
    let rows: Vec<(String, String)> = smoke
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            (
                v["utterance"].as_str().unwrap().to_string(),
                v["expected_tool"].as_str().unwrap().to_string(),
            )
        })
        .collect();

    for (name, encoder, index) in [
        ("setfit index", &mut runtime.encoder, &runtime.index),
        ("raw index", &mut runtime.raw_encoder, &runtime.raw_index),
    ] {
        for (strip_name, strip) in [
            ("raw query", Strip::None),
            ("strip all", Strip::SideCountDegree),
        ] {
            let mut top1 = 0;
            let mut stolen = Vec::new();
            let mut min_gap: Option<f32> = None;
            for (utterance, expected_tool) in &rows {
                let query_text = strip_modifiers(&normalize_utterance(utterance), strip);
                let query = encoder.encode(&query_text).unwrap();
                let ranked = rank_routes(index, &query, HelmMode::AllowEdit);
                let best_compose = ranked
                    .iter()
                    .find(|(r, _)| matches!(r, Route::ComposeMotion(_)))
                    .map(|(_, s)| *s)
                    .unwrap_or(0.0);
                let gap = ranked[0].1 - best_compose;
                min_gap = Some(min_gap.map_or(gap, |g: f32| g.min(gap)));
                if ranked[0].0.tool_name() == expected_tool {
                    top1 += 1;
                } else {
                    stolen.push(format!(
                        "{utterance} -> {}={:.3} (want {expected_tool})",
                        ranked[0].0.id(),
                        ranked[0].1
                    ));
                }
            }
            eprintln!(
                "{name:<14} {strip_name:<10} top1 {top1:>2}/{} | min gap trained-vs-best-compose {:+.3}",
                rows.len(),
                min_gap.unwrap_or(0.0)
            );
            for s in stolen {
                eprintln!("    miss: {s}");
            }
        }
    }
}
