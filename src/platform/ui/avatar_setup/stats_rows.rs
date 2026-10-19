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

pub fn rows() -> Vec<UiRow<StatsView>> {
    vec![
        UiRow::Choice {
            label: "Platform",
            options: |_world, view| {
                let platforms = [Platform::Pc, Platform::Quest];
                platforms
                    .iter()
                    .map(|p| ChoiceOption {
                        label: match p {
                            Platform::Pc => "PC".to_string(),
                            Platform::Quest => "Quest".to_string(),
                        },
                        selected: *p == view.platform,
                    })
                    .collect()
            },
            pick: |view, index| {
                view.platform = match index {
                    0 => Platform::Pc,
                    1 => Platform::Quest,
                    _ => unreachable!(),
                };
            },
        },
        UiRow::Canvas(|ui, _world, view| {
            let Some(rank) = &view.rank else {
                ui.text("No rank available");
                return;
            };
            if let Some(_table) = ui.begin_table("##avatar_stats", 3) {
                ui.table_setup_column("Stat", imgui::TableColumnFlags::empty(), None);
                ui.table_setup_column("Value", imgui::TableColumnFlags::empty(), None);
                ui.table_setup_column("Rank", imgui::TableColumnFlags::empty(), None);
                ui.table_headers_row();
                for item in &rank.items {
                    ui.table_next_row();
                    ui.table_next_column();
                    ui.text(item.stat);
                    ui.table_next_column();
                    ui.text(item.value.to_string());
                    ui.table_next_column();
                    ui.text_colored(rank_color(item.rank), format!("{:?}", item.rank));
                }
            }

            ui.separator();
            ui.text("Overall:");
            ui.same_line();
            ui.text_colored(rank_color(rank.overall), format!("{:?}", rank.overall));
        }),
    ]
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
