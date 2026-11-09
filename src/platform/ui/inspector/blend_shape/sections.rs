use super::expression_presets::{
    commit_expression_presets_view, expression_presets_rows, ExpressionPresetsView,
};
use crate::asset::AssetStorage;
use crate::ecs::component::{MorphChannelCatalog, MorphWeights};
use crate::ecs::resource::{BlendShapeInspectorState, ExpressionLibraryState};
use crate::ecs::systems::phases::event_dispatch::morph::MorphEvent;
use crate::ecs::systems::{find_expression_morph_entity, find_mesh_morph};
use crate::ecs::world::{Entity, World};
use crate::platform::ui::form_window::{rows_section, FormSection};
use crate::platform::ui::ui_rows::{ListRows, NamedValue, UiRow};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_avatar_core::expression::components::grouping::{ChannelGroup, ExpressionGrouping};
use thyllore_avatar_core::expression::systems::grouping::group_channels;

#[derive(Clone)]
pub(super) struct BlendShapeView {
    pub(super) entity: Entity,
    pub(super) search: String,
    pub(super) mirror_edit: bool,
}

pub(super) struct ChannelSource {
    pub(super) names: Vec<String>,
    pub(super) weights: Vec<f32>,
    pub(super) deforming_channels: Vec<usize>,
}

#[derive(Clone)]
pub(super) struct ChannelGroupView {
    pub(super) entity: Entity,
    pub(super) channels: Vec<NamedValue>,
    pub(super) collapsed: bool,
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

pub(super) fn commit_blend_shape_view(
    world: &World,
    shown: &BlendShapeView,
    view: &BlendShapeView,
) {
    if view.search != shown.search || view.mirror_edit != shown.mirror_edit {
        if let Some(mut state) = world.get_resource_mut::<BlendShapeInspectorState>() {
            state.search = view.search.clone();
            state.mirror_edit = view.mirror_edit;
        }
    }
}

pub(super) fn format_section_title(
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

pub(super) fn channel_source(world: &World, entity: Entity) -> Option<ChannelSource> {
    let catalog = world.get_component::<MorphChannelCatalog>(entity)?;
    let weights = world.get_component::<MorphWeights>(entity)?;
    Some(ChannelSource {
        names: catalog.names.clone(),
        weights: weights.weights.clone(),
        deforming_channels: catalog.deforming_channels.clone(),
    })
}

pub(super) fn blend_shape_rows() -> Vec<UiRow<BlendShapeView>> {
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

pub(super) fn channel_sliders_row() -> UiRow<ChannelGroupView> {
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

pub(super) fn collect_channel_groups(
    world: &World,
    view: &BlendShapeView,
) -> Vec<(String, ChannelGroupView)> {
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

pub(super) fn filter_channels(
    group: &ChannelGroup,
    source: &ChannelSource,
    search: &str,
) -> Vec<NamedValue> {
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
