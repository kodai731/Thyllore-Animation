use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::ExpressionLibraryState;
use crate::ecs::systems::find_expression_morph_entity;
use crate::ecs::systems::phases::event_dispatch::morph::MorphEvent;
use crate::ecs::world::{Entity, World};
use crate::platform::ui::ui_rows::{ListRows, UiRow};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone)]
pub(super) struct ExpressionPresetsView {
    pub(super) entity: Entity,
    pub(super) presets: Vec<(String, usize)>,
    pub(super) capture_name: String,
}

#[derive(Clone)]
pub(super) struct PresetItem {
    index: usize,
}

pub(super) fn preset_items(
    _world: &World,
    view: &ExpressionPresetsView,
) -> Vec<(String, PresetItem)> {
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

pub(super) fn preset_rows(_item: &PresetItem) -> Vec<UiRow<PresetItem>> {
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

pub(super) fn preset_always_enabled(_world: &World, _item: &PresetItem) -> bool {
    true
}

pub(super) fn preset_apply_command(item: &PresetItem) -> Box<dyn UiCommand> {
    Box::new(ApplyExpressionPresetCommand {
        preset_index: item.index,
    })
}

pub(super) fn preset_remove_command(item: &PresetItem) -> Box<dyn UiCommand> {
    Box::new(RemoveExpressionPresetCommand {
        preset_index: item.index,
    })
}

pub(super) fn preset_on_change(
    _view: &ExpressionPresetsView,
    _index: usize,
    _item: PresetItem,
) -> Box<dyn UiCommand> {
    Box::new(NoOpCommand)
}

pub(super) fn preset_initially_open(_item: &PresetItem) -> bool {
    false
}

pub(super) fn expression_presets_rows() -> Vec<UiRow<ExpressionPresetsView>> {
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

pub(super) fn expression_capture_name_get(view: &ExpressionPresetsView) -> &str {
    &view.capture_name
}

pub(super) fn expression_capture_name_set(view: &mut ExpressionPresetsView, value: String) {
    view.capture_name = value;
}

pub(super) fn expression_capture_enabled(_world: &World, view: &ExpressionPresetsView) -> bool {
    !view.capture_name.is_empty()
}

pub(super) fn expression_capture_command(view: &ExpressionPresetsView) -> Box<dyn UiCommand> {
    Box::new(CaptureExpressionPresetCommand {
        entity: view.entity,
        name: view.capture_name.clone(),
    })
}

pub(super) fn expression_save_enabled(_world: &World, _view: &ExpressionPresetsView) -> bool {
    true
}

pub(super) fn expression_save_command(_view: &ExpressionPresetsView) -> Box<dyn UiCommand> {
    Box::new(SaveExpressionLibraryCommand)
}

pub(super) fn commit_expression_presets_view(
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

#[derive(Clone, Debug)]
pub(super) struct ApplyExpressionPresetCommand {
    preset_index: usize,
}

impl UiCommand for ApplyExpressionPresetCommand {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        let Some(entity) = find_expression_morph_entity(world, assets, graphics) else {
            msg_warn!("Apply expression preset: no expression morph entity found");
            return;
        };
        world.send_command(MorphEvent::ApplyExpressionPreset {
            entity,
            preset_index: self.preset_index,
        });
    }
}

#[derive(Clone, Debug)]
pub(super) struct RemoveExpressionPresetCommand {
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
pub(super) struct CaptureExpressionPresetCommand {
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
pub(super) struct SaveExpressionLibraryCommand;

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
pub(super) struct NoOpCommand;

impl UiCommand for NoOpCommand {
    fn apply(
        self: Box<Self>,
        _world: &mut World,
        _assets: &mut AssetStorage,
        _graphics: &GraphicsResources,
    ) {
    }
}
