use crate::ecs::events::UiCommand;
use crate::ecs::resource::{BlendShapeInspectorState, ExpressionLibraryState};
use crate::ecs::world::World;
use crate::platform::ui::form_window::FormSection;
use thyllore_avatar_core::expression::components::grouping::ChannelGroup;

use super::expression_presets::*;
use super::sections::*;

use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

fn make_source(names: &[&str], deforming_channels: Vec<usize>) -> ChannelSource {
    ChannelSource {
        names: names.iter().map(|name| name.to_string()).collect(),
        weights: (0..names.len()).map(|index| index as f32 * 0.25).collect(),
        deforming_channels,
    }
}

fn make_group(channel_indices: Vec<usize>) -> ChannelGroup {
    ChannelGroup {
        name: "Face".to_string(),
        channel_indices,
        collapsed: false,
        exclude_from_reset: false,
    }
}

fn filtered_labels(group: &ChannelGroup, source: &ChannelSource) -> Vec<String> {
    filter_channels(group, source, "")
        .into_iter()
        .map(|channel| channel.label)
        .collect()
}

#[test]
fn blend_shape_rows_follow_declaration_order() {
    assert_eq!(
        visible_labels(&blend_shape_rows()),
        [
            "Search",
            "Mirror L/R",
            "Reset Weights",
            "Key Weights",
            "Channels"
        ]
    );
}

#[test]
fn blend_shape_rows_have_no_duplicate_labels() {
    assert!(find_duplicate_labels(&blend_shape_rows()).is_empty());
}

#[test]
fn filter_channels_matches_search_case_insensitively() {
    let source = make_source(&["mouth_open", "Eye_Blink", "smile"], vec![0, 1, 2]);
    let filtered = filter_channels(&make_group(vec![0, 1, 2]), &source, "eye");

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].label, "Eye_Blink");
    assert_eq!(filtered[0].key, 1);
    assert_eq!(filtered[0].value, 0.25);
}

#[test]
fn filter_channels_skips_non_deforming_channels() {
    let source = make_source(&["mouth_open", "eye_blink"], vec![1]);
    assert_eq!(
        filtered_labels(&make_group(vec![0, 1]), &source),
        ["eye_blink"]
    );
}

#[test]
fn filter_channels_keeps_group_order() {
    let source = make_source(&["a", "b", "c"], vec![0, 1, 2]);
    assert_eq!(
        filtered_labels(&make_group(vec![2, 0]), &source),
        ["c", "a"]
    );
}

#[test]
fn expression_presets_rows_follow_declaration_order() {
    use crate::platform::ui::ui_rows::row_labels;
    let rows = expression_presets_rows();
    let labels: Vec<Option<&str>> = row_labels(&rows);
    assert_eq!(
        labels,
        [
            Some("Presets"),
            Some("Preset Name"),
            Some("Capture Preset"),
            Some("Save Presets")
        ]
    );
}

#[test]
fn expression_presets_rows_have_no_duplicate_labels() {
    use crate::platform::ui::ui_rows::find_duplicate_labels;
    let rows = expression_presets_rows();
    assert!(find_duplicate_labels(&rows).is_empty());
}

#[test]
fn preset_header_has_weight_count() {
    let presets: Vec<(String, usize)> = vec![("My Preset".to_string(), 3)];
    let view = ExpressionPresetsView {
        entity: 0,
        presets,
        capture_name: String::new(),
    };
    let items = preset_items(&World::new(), &view);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].0, "My Preset (3 weights)");
}

#[test]
fn preset_header_empty_when_no_weights() {
    let presets: Vec<(String, usize)> = vec![("Empty Preset".to_string(), 0)];
    let view = ExpressionPresetsView {
        entity: 0,
        presets,
        capture_name: String::new(),
    };
    let items = preset_items(&World::new(), &view);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].0, "Empty Preset (empty)");
}

#[test]
fn commit_blend_shape_view_no_change_when_shown_equals_view() {
    let mut world = World::new();
    let initial_search = "test".to_string();
    let state = BlendShapeInspectorState {
        search: initial_search.clone(),
        mirror_edit: true,
    };
    world.insert_resource(state);

    let view = BlendShapeView {
        entity: 0,
        search: "test".to_string(),
        mirror_edit: true,
    };

    commit_blend_shape_view(&world, &view, &view);

    let stored = world.get_resource::<BlendShapeInspectorState>().unwrap();
    assert_eq!(stored.search, initial_search);
    assert_eq!(stored.mirror_edit, true);
}

#[test]
fn commit_blend_shape_view_reflects_search_change() {
    let mut world = World::new();
    let state = BlendShapeInspectorState {
        search: "old".to_string(),
        mirror_edit: false,
    };
    world.insert_resource(state);

    let shown = BlendShapeView {
        entity: 0,
        search: "old".to_string(),
        mirror_edit: false,
    };

    let mut view = shown.clone();
    view.search = "new".to_string();

    commit_blend_shape_view(&world, &shown, &view);

    let stored = world.get_resource::<BlendShapeInspectorState>().unwrap();
    assert_eq!(stored.search, "new");
}

#[test]
fn commit_blend_shape_view_reflects_mirror_edit_change() {
    let mut world = World::new();
    let state = BlendShapeInspectorState {
        search: "test".to_string(),
        mirror_edit: false,
    };
    world.insert_resource(state);

    let shown = BlendShapeView {
        entity: 0,
        search: "test".to_string(),
        mirror_edit: false,
    };

    let mut view = shown.clone();
    view.mirror_edit = true;

    commit_blend_shape_view(&world, &shown, &view);

    let stored = world.get_resource::<BlendShapeInspectorState>().unwrap();
    assert_eq!(stored.mirror_edit, true);
}

#[test]
fn commit_expression_presets_view_no_change_when_shown_equals_view() {
    let mut world = World::new();
    let initial_capture_name = String::new();
    let state = ExpressionLibraryState::default();
    world.insert_resource(state);

    let view = ExpressionPresetsView {
        entity: 0,
        presets: vec![],
        capture_name: "test".to_string(),
    };

    commit_expression_presets_view(&world, &view, &view);

    let stored = world.get_resource::<ExpressionLibraryState>().unwrap();
    assert_eq!(stored.capture_name, initial_capture_name);
}

#[test]
fn commit_expression_presets_view_reflects_capture_name_change() {
    let mut world = World::new();
    let state = ExpressionLibraryState::default();
    world.insert_resource(state);

    let shown = ExpressionPresetsView {
        entity: 0,
        presets: vec![],
        capture_name: "old".to_string(),
    };

    let view = ExpressionPresetsView {
        entity: 0,
        presets: vec![],
        capture_name: "new".to_string(),
    };

    commit_expression_presets_view(&world, &shown, &view);

    let stored = world.get_resource::<ExpressionLibraryState>().unwrap();
    assert_eq!(stored.capture_name, "new");
}
