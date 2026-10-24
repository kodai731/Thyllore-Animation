use thyllore_avatar_core::expression::components::grouping::{ChannelGroup, ExpressionGrouping};
use thyllore_avatar_core::expression::systems::grouping::group_channels;

use crate::asset::AssetStorage;
use crate::ecs::component::{MorphChannelCatalog, MorphWeights};
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{BlendShapeInspectorState, ExpressionLibraryState};
use crate::ecs::systems::phases::event_dispatch::morph::MorphEvent;
use crate::ecs::systems::{find_expression_morph_entity, find_mesh_morph};
use crate::ecs::world::{Entity, World};
use crate::platform::ui::form_window::{rows_section, FormSection};
use crate::platform::ui::ui_rows::{ListRows, NamedValue, UiRow};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone)]
struct BlendShapeView {
    entity: Entity,
    search: String,
    mirror_edit: bool,
}

struct ChannelSource {
    names: Vec<String>,
    weights: Vec<f32>,
    deforming_channels: Vec<usize>,
}

#[derive(Clone)]
struct ChannelGroupView {
    entity: Entity,
    channels: Vec<NamedValue>,
    collapsed: bool,
}

#[derive(Clone)]
struct ExpressionPresetsView {
    entity: Entity,
    presets: Vec<(String, usize)>,
    capture_name: String,
}

#[derive(Clone)]
struct PresetItem {
    index: usize,
}

fn preset_items(_world: &World, view: &ExpressionPresetsView) -> Vec<(String, PresetItem)> {
    view.presets
        .iter()
        .enumerate()
        .map(|(i, (name, weight_count))| {
            let label = if *weight_count == 0 {
                format!("{} (empty)", name)
            } else {
                format!("{} ({} weights)", name, weight_count)
            };
            (label, PresetItem { index: i })
        })
        .collect::<Vec<_>>()
}

fn preset_rows(_item: &PresetItem) -> Vec<UiRow<PresetItem>> {
    vec![
        UiRow::Action {
            label: "Apply Preset",
            enabled: preset_always_enabled,
            on_click: preset_apply_command,
        },
        UiRow::Action {
            label: "Remove Preset",
            enabled: preset_always_enabled,
            on_click: preset_remove_command,
        },
    ]
}

fn preset_always_enabled(_world: &World, _item: &PresetItem) -> bool {
    true
}

fn preset_apply_command(item: &PresetItem) -> Box<dyn UiCommand> {
    Box::new(ApplyExpressionPresetCommand {
        preset_index: item.index,
    })
}

fn preset_remove_command(item: &PresetItem) -> Box<dyn UiCommand> {
    Box::new(RemoveExpressionPresetCommand {
        preset_index: item.index,
    })
}

fn preset_on_change(
    _view: &ExpressionPresetsView,
    _index: usize,
    _item: PresetItem,
) -> Box<dyn UiCommand> {
    Box::new(NoOpCommand)
}

fn preset_initially_open(_item: &PresetItem) -> bool {
    false
}

fn expression_presets_rows() -> Vec<UiRow<ExpressionPresetsView>> {
    vec![
        UiRow::List(Box::new(ListRows::<ExpressionPresetsView, PresetItem> {
            title: "Presets",
            items: preset_items,
            rows: preset_rows,
            on_change: preset_on_change,
            initially_open: preset_initially_open,
        })),
        UiRow::Text {
            label: "Preset Name",
            get: expression_capture_name_get,
            set: expression_capture_name_set,
        },
        UiRow::Action {
            label: "Capture Preset",
            enabled: expression_capture_enabled,
            on_click: expression_capture_command,
        },
        UiRow::Action {
            label: "Save Presets",
            enabled: expression_save_enabled,
            on_click: expression_save_command,
        },
    ]
}

fn expression_capture_name_get(view: &ExpressionPresetsView) -> &str {
    &view.capture_name
}

fn expression_capture_name_set(view: &mut ExpressionPresetsView, value: String) {
    view.capture_name = value;
}

fn expression_capture_enabled(_world: &World, view: &ExpressionPresetsView) -> bool {
    !view.capture_name.is_empty()
}

fn expression_capture_command(view: &ExpressionPresetsView) -> Box<dyn UiCommand> {
    Box::new(CaptureExpressionPresetCommand {
        entity: view.entity,
        name: view.capture_name.clone(),
    })
}

fn expression_save_enabled(_world: &World, _view: &ExpressionPresetsView) -> bool {
    true
}

fn expression_save_command(_view: &ExpressionPresetsView) -> Box<dyn UiCommand> {
    Box::new(SaveExpressionLibraryCommand)
}

pub fn blend_shape_sections(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Vec<Box<dyn FormSection>> {
    let mut sections: Vec<Box<dyn FormSection>> = Vec::new();

    if world.has_component::<MorphChannelCatalog>(entity) {
        let (search, mirror_edit) = world
            .get_resource::<BlendShapeInspectorState>()
            .map(|state| (state.search.clone(), state.mirror_edit))
            .unwrap_or_default();
        let view = BlendShapeView {
            entity,
            search,
            mirror_edit,
        };
        sections.push(rows_section(
            vec![UiRow::Group {
                title: format!(
                    "Blend Shapes ({})###blend_shapes_{}",
                    format_section_title(world, entity, assets, graphics),
                    entity
                )
                .into(),
                initially_open: true,
                rows: blend_shape_rows(),
            }],
            view,
            commit_blend_shape_view,
        ));
    }

    if let Some(expression_entity) = find_expression_morph_entity(world, assets, graphics) {
        let (capture_name, presets): (String, Vec<(String, usize)>) = world
            .get_resource::<ExpressionLibraryState>()
            .map(|state| {
                let capture_name = state.capture_name.clone();
                let presets: Vec<(String, usize)> = state
                    .library
                    .presets
                    .iter()
                    .map(|preset| (preset.name.clone(), preset.weights.len()))
                    .collect();
                (capture_name, presets)
            })
            .unwrap_or((String::new(), Vec::new()));
        let view = ExpressionPresetsView {
            entity: expression_entity,
            presets,
            capture_name,
        };
        sections.push(rows_section(
            vec![UiRow::Group {
                title: "Expression Presets".into(),
                initially_open: false,
                rows: expression_presets_rows(),
            }],
            view,
            commit_expression_presets_view,
        ));
    }

    sections
}

fn commit_blend_shape_view(world: &World, shown: &BlendShapeView, view: &BlendShapeView) {
    if view.search != shown.search || view.mirror_edit != shown.mirror_edit {
        if let Some(mut state) = world.get_resource_mut::<BlendShapeInspectorState>() {
            state.search = view.search.clone();
            state.mirror_edit = view.mirror_edit;
        }
    }
}

fn commit_expression_presets_view(
    world: &World,
    shown: &ExpressionPresetsView,
    view: &ExpressionPresetsView,
) {
    if view.capture_name != shown.capture_name {
        if let Some(mut state) = world.get_resource_mut::<ExpressionLibraryState>() {
            state.capture_name = view.capture_name.clone();
        }
    }
}

fn format_section_title(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> String {
    find_mesh_morph(world, entity, assets, graphics)
        .map(|morph| morph.source_mesh.clone())
        .filter(|source_mesh| !source_mesh.is_empty())
        .unwrap_or_else(|| format!("entity {}", entity))
}

fn channel_source(world: &World, entity: Entity) -> Option<ChannelSource> {
    let catalog = world.get_component::<MorphChannelCatalog>(entity)?;
    let weights = world.get_component::<MorphWeights>(entity)?;
    Some(ChannelSource {
        names: catalog.names.clone(),
        weights: weights.weights.clone(),
        deforming_channels: catalog.deforming_channels.clone(),
    })
}

fn blend_shape_rows() -> Vec<UiRow<BlendShapeView>> {
    vec![
        UiRow::Text {
            label: "Search",
            get: |view: &BlendShapeView| &view.search,
            set: |view: &mut BlendShapeView, value: String| {
                view.search = value;
            },
        },
        UiRow::Toggle {
            label: "Mirror L/R",
            get: |view: &BlendShapeView| view.mirror_edit,
            set: |view: &mut BlendShapeView, value: bool| {
                view.mirror_edit = value;
            },
        },
        UiRow::Action {
            label: "Reset Weights",
            enabled: |_, _: &BlendShapeView| true,
            on_click: |view: &BlendShapeView| {
                Box::new(MorphEvent::ResetMorphWeights {
                    entity: view.entity,
                }) as _
            },
        },
        UiRow::Action {
            label: "Key Weights",
            enabled: |_, _: &BlendShapeView| true,
            on_click: |view: &BlendShapeView| {
                Box::new(MorphEvent::KeyMorphWeights {
                    entity: view.entity,
                }) as _
            },
        },
        UiRow::List(Box::new(ListRows::<BlendShapeView, ChannelGroupView> {
            title: "Channels",
            items: collect_channel_groups,
            rows: |_| vec![channel_sliders_row()],
            on_change: |_, _, _| unreachable!("channel sliders send their own commands"),
            initially_open: |group| !group.collapsed,
        })),
    ]
}

fn channel_sliders_row() -> UiRow<ChannelGroupView> {
    UiRow::Sliders {
        items: |_, group: &ChannelGroupView| group.channels.clone(),
        min: 0.0,
        max: 1.0,
        on_change: |group: &ChannelGroupView, channel: &NamedValue, weight: f32| {
            Box::new(MorphEvent::SetMorphWeight {
                entity: group.entity,
                channel: channel.label.clone(),
                weight,
            }) as _
        },
    }
}

fn collect_channel_groups(world: &World, view: &BlendShapeView) -> Vec<(String, ChannelGroupView)> {
    let Some(source) = channel_source(world, view.entity) else {
        return Vec::new();
    };
    group_channels(&source.names, ExpressionGrouping::shared())
        .into_iter()
        .filter_map(|group| {
            let channels = filter_channels(&group, &source, &view.search);
            if channels.is_empty() {
                return None;
            }
            let group_view = ChannelGroupView {
                entity: view.entity,
                channels,
                collapsed: group.collapsed,
            };
            Some((group.name, group_view))
        })
        .collect()
}

fn filter_channels(group: &ChannelGroup, source: &ChannelSource, search: &str) -> Vec<NamedValue> {
    let search = search.to_lowercase();
    group
        .channel_indices
        .iter()
        .copied()
        .filter(|channel| source.deforming_channels.contains(channel))
        .filter(|&channel| source.names[channel].to_lowercase().contains(&search))
        .filter_map(|channel| {
            Some(NamedValue {
                label: source.names[channel].clone(),
                key: channel,
                value: *source.weights.get(channel)?,
            })
        })
        .collect()
}

#[derive(Clone, Debug)]
struct ApplyExpressionPresetCommand {
    preset_index: usize,
}
impl UiCommand for ApplyExpressionPresetCommand {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        _assets: &mut AssetStorage,
        _graphics: &GraphicsResources,
    ) {
        let entity = find_expression_morph_entity(
            world,
            &AssetStorage::default(),
            &GraphicsResources::default(),
        )
        .unwrap_or_else(|| 0);
        world.send_command(MorphEvent::ApplyExpressionPreset {
            entity,
            preset_index: self.preset_index,
        });
    }
}

#[derive(Clone, Debug)]
struct RemoveExpressionPresetCommand {
    preset_index: usize,
}
impl UiCommand for RemoveExpressionPresetCommand {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        _assets: &mut AssetStorage,
        _graphics: &GraphicsResources,
    ) {
        world.send_command(MorphEvent::RemoveExpressionPreset {
            preset_index: self.preset_index,
        });
    }
}

#[derive(Clone, Debug)]
struct CaptureExpressionPresetCommand {
    entity: Entity,
    name: String,
}
impl UiCommand for CaptureExpressionPresetCommand {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        _assets: &mut AssetStorage,
        _graphics: &GraphicsResources,
    ) {
        world.send_command(MorphEvent::CaptureExpressionPreset {
            entity: self.entity,
            name: self.name,
        });
    }
}

#[derive(Clone, Debug)]
struct SaveExpressionLibraryCommand;
impl UiCommand for SaveExpressionLibraryCommand {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        _assets: &mut AssetStorage,
        _graphics: &GraphicsResources,
    ) {
        world.send_command(MorphEvent::SaveExpressionLibrary);
    }
}

#[derive(Clone, Debug)]
struct NoOpCommand;
impl UiCommand for NoOpCommand {
    fn apply(
        self: Box<Self>,
        _world: &mut World,
        _assets: &mut AssetStorage,
        _graphics: &GraphicsResources,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
