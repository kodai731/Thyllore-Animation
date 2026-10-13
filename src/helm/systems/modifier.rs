//! Fills the speed, side and count slots from words in the utterance, and strips those
//! words so the rest of the utterance can be compared with the pose table's labels.
//!
//! This is slot extraction, not routing, which is why it outlived the keyword rule
//! table it used to share a file with. A modifier can only set `speed` on a route
//! the embedding router already chose, so a wrong match degrades one argument of an
//! otherwise correct call. The rules it sat beside chose the route itself and could
//! turn a save into a screenshot.
//!
//! Terms are matched against `normalize::normalize_utterance` output, so they must
//! be lowercase. Longer terms come first within a preset because the first hit wins
//! and `半分の速さ` must not be reached through `速`.

use thyllore_avatar_core::motion::seed::components::motion_spec::MotionSide;

use crate::helm::components::tool_call::SpeedPreset;

const SLOW_TERMS: [&str; 8] = [
    "slowly",
    "slower",
    "slow",
    "遅く",
    "ゆっくり",
    "スロー",
    "低速",
    "半分の速さ",
];

const FAST_TERMS: [&str; 7] = [
    "faster",
    "fast",
    "quickly",
    "速く",
    "高速",
    "倍速",
    "早送り",
];

const NORMAL_TERMS: [&str; 7] = [
    "normal speed",
    "standard",
    "default speed",
    "標準",
    "等速",
    "通常速度",
    "普通",
];

const SPEED_MODIFIERS: [(SpeedPreset, &[&str]); 3] = [
    (SpeedPreset::Slow, &SLOW_TERMS),
    (SpeedPreset::Fast, &FAST_TERMS),
    (SpeedPreset::Normal, &NORMAL_TERMS),
];

pub fn extract_speed_modifier(normalized: &str) -> Option<SpeedPreset> {
    SPEED_MODIFIERS
        .iter()
        .find(|(_, terms)| terms.iter().any(|term| normalized.contains(term)))
        .map(|(preset, _)| *preset)
}

const LEFT_WORD: &str = "left";
const RIGHT_WORD: &str = "right";
const LEFT_MARK: &str = "左";
const RIGHT_MARK: &str = "右";

const LEFT_TERMS: [&str; 3] = [LEFT_WORD, "左手", LEFT_MARK];
const RIGHT_TERMS: [&str; 3] = [RIGHT_WORD, "右手", RIGHT_MARK];

const SIDE_PARTICLES: [&str; 4] = ["の", "で", "を", "に"];

const SIDE_MODIFIERS: [(MotionSide, &[&str]); 2] = [
    (MotionSide::Left, &LEFT_TERMS),
    (MotionSide::Right, &RIGHT_TERMS),
];

pub fn extract_side_modifier(normalized: &str) -> Option<MotionSide> {
    SIDE_MODIFIERS
        .iter()
        .find(|(_, terms)| terms.iter().any(|term| normalized.contains(term)))
        .map(|(side, _)| *side)
}

const COUNT_WORDS: [(&str, u32); 10] = [
    ("twice", 2),
    ("two times", 2),
    ("three times", 3),
    ("四回", 4),
    ("三回", 3),
    ("二回", 2),
    ("2度", 2),
    ("3度", 3),
    ("二度", 2),
    ("三度", 3),
];

const COUNT_UNITS: [&str; 2] = ["回", " times"];

const DIGIT_COUNT_UNITS: [&str; 4] = ["回", "度", " times", "times"];

const DEGREE_TERMS: [&str; 11] = [
    "してください",
    "ください",
    "もう一度",
    "大きく",
    "深く",
    "高く",
    "低く",
    "deeply",
    "high",
    "low",
    "please",
];

/// `3回` / `3 times` style counts first, then the spelled-out words; a count of 0 is no count.
pub fn extract_count_modifier(normalized: &str) -> Option<u32> {
    if let Some(count) = COUNT_UNITS
        .iter()
        .filter_map(|unit| digits_before(normalized, unit))
        .find(|count| *count >= 1)
    {
        return Some(count);
    }
    COUNT_WORDS
        .iter()
        .find(|(word, _)| normalized.contains(word))
        .map(|(_, count)| *count)
}

fn digits_before(normalized: &str, unit: &str) -> Option<u32> {
    let unit_start = normalized.find(unit)?;
    let digits: String = normalized[..unit_start]
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse().ok()
}

/// The utterance without its side, count, speed and degree words; body part nouns stay
/// (`左手を振って` → `手を振って`), and the slots themselves are read by the extractors above.
pub fn strip_slot_terms(normalized: &str) -> String {
    let mut text = strip_side_terms(normalized);
    text = strip_digit_counts(&text);

    let count_words = COUNT_WORDS.iter().map(|(word, _)| *word);
    let speed_terms = SPEED_MODIFIERS
        .iter()
        .flat_map(|(_, terms)| terms.iter().copied());
    for term in count_words.chain(speed_terms).chain(DEGREE_TERMS) {
        text = strip_term(&text, term);
    }
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn strip_side_terms(normalized: &str) -> String {
    let mut text = strip_term(&strip_term(normalized, LEFT_WORD), RIGHT_WORD);
    for mark in [LEFT_MARK, RIGHT_MARK] {
        for particle in SIDE_PARTICLES {
            text = text.replace(&format!("{mark}{particle}"), "");
        }
        text = text.replace(mark, "");
    }
    text
}

fn strip_digit_counts(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(|c: char| c.is_ascii_digit()) {
        stripped.push_str(&rest[..start]);
        let after_digits = rest[start..].trim_start_matches(|c: char| c.is_ascii_digit());
        rest = DIGIT_COUNT_UNITS
            .iter()
            .find_map(|unit| after_digits.strip_prefix(unit))
            .unwrap_or(after_digits);
    }
    stripped.push_str(rest);
    stripped
}

/// ASCII terms are removed as whole words (`low` must not cut `slowly`), others as substrings.
fn strip_term(text: &str, term: &str) -> String {
    if !term.is_ascii() {
        return text.replace(term, "");
    }
    let padded = format!(" {text} ");
    padded.replace(&format!(" {term} "), "  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helm::systems::normalize::normalize_utterance;

    fn extract(utterance: &str) -> Option<SpeedPreset> {
        extract_speed_modifier(&normalize_utterance(utterance))
    }

    #[test]
    fn every_term_is_lowercase_so_it_matches_normalized_text() {
        for (_, terms) in SPEED_MODIFIERS {
            for term in terms {
                assert_eq!(*term, term.to_lowercase());
            }
        }
    }

    #[test]
    fn english_modifiers_map_to_their_preset() {
        assert_eq!(extract("generate a slow walk"), Some(SpeedPreset::Slow));
        assert_eq!(extract("make him run fast"), Some(SpeedPreset::Fast));
        assert_eq!(extract("use the normal speed"), Some(SpeedPreset::Normal));
    }

    #[test]
    fn japanese_modifiers_map_to_their_preset() {
        assert_eq!(
            extract("ゆっくり歩くモーションを作って"),
            Some(SpeedPreset::Slow)
        );
        assert_eq!(extract("倍速で再生して"), Some(SpeedPreset::Fast));
        assert_eq!(extract("標準の速度に戻して"), Some(SpeedPreset::Normal));
    }

    #[test]
    fn an_utterance_without_a_modifier_yields_nothing() {
        assert_eq!(extract("play the animation"), None);
        assert_eq!(extract("シーンを保存して"), None);
    }

    #[test]
    fn full_width_input_still_matches() {
        assert_eq!(extract("ＦＡＳＴ で再生"), Some(SpeedPreset::Fast));
    }

    fn side(utterance: &str) -> Option<MotionSide> {
        extract_side_modifier(&normalize_utterance(utterance))
    }

    fn count(utterance: &str) -> Option<u32> {
        extract_count_modifier(&normalize_utterance(utterance))
    }

    #[test]
    fn side_words_in_both_languages_pick_the_side() {
        assert_eq!(side("左手を振って"), Some(MotionSide::Left));
        assert_eq!(side("wave the right hand"), Some(MotionSide::Right));
        assert_eq!(side("手を振って"), None);
    }

    #[test]
    fn counts_come_from_digits_or_words() {
        assert_eq!(count("手を3回振って"), Some(3));
        assert_eq!(count("３回頷いて"), Some(3));
        assert_eq!(count("wave 2 times"), Some(2));
        assert_eq!(count("wave twice"), Some(2));
        assert_eq!(count("二回お辞儀して"), Some(2));
        assert_eq!(count("0回振る"), None);
        assert_eq!(count("手を振って"), None);
    }

    fn strip(utterance: &str) -> String {
        strip_slot_terms(&normalize_utterance(utterance))
    }

    #[test]
    fn side_count_and_degree_words_leave_the_motion() {
        assert_eq!(strip("左手を2回振って"), "手を振って");
        assert_eq!(strip("右で2回パンチして"), "パンチして");
        assert_eq!(strip("左を向いて"), "向いて");
        assert_eq!(strip("深くお辞儀してください"), "お辞儀");
        assert_eq!(strip("wave the left hand twice"), "wave the hand");
        assert_eq!(strip("wave your right hand 3 times"), "wave your hand");
        assert_eq!(strip("nod your head two times slowly"), "nod your head");
        assert_eq!(strip("squat down low"), "squat down");
    }

    #[test]
    fn utterances_without_slot_words_are_unchanged() {
        assert_eq!(strip("手を振って"), "手を振って");
        assert_eq!(strip("follow the camera"), "follow the camera");
    }
}
