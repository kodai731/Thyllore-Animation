use crate::ecs::resource::AvatarSetupState;
use crate::platform::ui::ui_rows::{ChoiceOption, UiRow};
use thyllore_avatar_core::vrchat::rank::{PerformanceRank, Platform, RankReport};

#[derive(Clone)]
pub struct StatsView {
    pub platform: Platform,
    pub rank: Option<RankReport>,
}

impl StatsView {
    pub fn from_state(state: &AvatarSetupState) -> Self {
        Self {
            platform: state.platform,
            rank: state.rank.clone(),
        }
    }
}

const PLATFORMS: [Platform; 2] = [Platform::Pc, Platform::Quest];

fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Pc => "PC",
        Platform::Quest => "Quest",
    }
}

fn platform_choice_row() -> UiRow<StatsView> {
    UiRow::Choice {
        label: "Platform",
        options: |_world, view| {
            PLATFORMS
                .iter()
                .map(|&platform| ChoiceOption {
                    label: platform_label(platform).to_string(),
                    selected: platform == view.platform,
                })
                .collect()
        },
        pick: |view, index| {
            if let Some(&platform) = PLATFORMS.get(index) {
                view.platform = platform;
            }
        },
    }
}

pub fn rows() -> Vec<UiRow<StatsView>> {
    vec![
        platform_choice_row(),
        UiRow::Canvas(|ui, _world, view| match &view.rank {
            Some(rank) => draw_rank_report(ui, rank),
            None => ui.text("No rank available"),
        }),
    ]
}

fn draw_rank_report(ui: &imgui::Ui, rank: &RankReport) {
    if let Some(_table) = ui.begin_table("##avatar_stats", 3) {
        ui.table_setup_column("Stat", imgui::TableColumnFlags::empty(), None);
        ui.table_setup_column("Value", imgui::TableColumnFlags::empty(), None);
        ui.table_setup_column("Rank", imgui::TableColumnFlags::empty(), None);
        ui.table_headers_row();
        for item in &rank.items {
            draw_rank_row(ui, item.stat, &item.value.to_string(), item.rank);
        }
    }

    ui.separator();
    ui.text("Overall:");
    ui.same_line();
    ui.text_colored(rank_color(rank.overall), format!("{:?}", rank.overall));
}

fn draw_rank_row(ui: &imgui::Ui, stat: &str, value: &str, rank: PerformanceRank) {
    ui.table_next_row();
    ui.table_next_column();
    ui.text(stat);
    ui.table_next_column();
    ui.text(value);
    ui.table_next_column();
    ui.text_colored(rank_color(rank), format!("{:?}", rank));
}

fn rank_color(rank: PerformanceRank) -> [f32; 4] {
    match rank {
        PerformanceRank::Excellent => [0.3, 0.9, 1.0, 1.0],
        PerformanceRank::Good => [0.3, 1.0, 0.3, 1.0],
        PerformanceRank::Medium => [1.0, 1.0, 0.3, 1.0],
        PerformanceRank::Poor => [1.0, 0.6, 0.2, 1.0],
        PerformanceRank::VeryPoor => [1.0, 0.3, 0.3, 1.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;

    #[test]
    fn test_row_labels() {
        let rows = rows();
        let labels = visible_labels(&rows);
        assert_eq!(labels, ["Platform"]);
    }

    #[test]
    fn test_no_duplicate_labels() {
        let rows = rows();
        let mut seen = std::collections::HashSet::new();
        for label in visible_labels(&rows) {
            assert!(seen.insert(label), "duplicate label: {}", label);
        }
    }
}
