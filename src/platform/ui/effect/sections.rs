use std::sync::OnceLock;

use thyllore_scene_core::{ScalarParam, UiParam};

use crate::ecs::component::ScalarChannelDomain;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::{resolve_selected_effect, EffectEdit, EffectPreset};
use crate::ecs::world::{Entity, World};
use crate::platform::ui::effect::rows::{effect_instances, instance_options, preset_options};
use crate::platform::ui::form_window::{FormSection, RowsSection};
use crate::platform::ui::param_widgets::rows_from_params;
use crate::platform::ui::ui_rows::{FieldRows, UiRow};

pub(crate) struct EffectSectionHook {
    pub key: &'static str,
    pub order: u32,
    pub section: fn(&World) -> Option<Box<dyn FormSection>>,
}

inventory::collect!(EffectSectionHook);

#[macro_export]
macro_rules! effect_section_hook {
    ($hook:expr) => {
        inventory::submit! { $hook }
    };
}

pub(crate) fn collect_effect_sections() -> &'static [&'static EffectSectionHook] {
    static SECTIONS: OnceLock<Vec<&'static EffectSectionHook>> = OnceLock::new();
    SECTIONS.get_or_init(|| {
        let mut sections: Vec<&'static EffectSectionHook> =
            inventory::iter::<EffectSectionHook>().collect();
        sections.sort_by_key(|hook| (hook.order, hook.key));
        for pair in sections.windows(2) {
            assert!(
                pair[0].key != pair[1].key,
                "effect section hook {} registered twice",
                pair[0].key
            );
        }
        sections
    })
}

pub(super) trait EffectPanel: 'static {
    type Component: EffectPreset + Clone + PartialEq + Default + std::fmt::Debug + 'static;
    type RenderSettings: Clone + PartialEq + Default + std::fmt::Debug + 'static;

    const NAME: &'static str;
    const ADD_LABEL: &'static str;
    const SPAWN_HOOK_KEY: &'static str;
    const PRESET_NAMES: &'static [&'static str];

    fn ui_params() -> &'static [UiParam];
    fn scalar_params() -> &'static [ScalarParam<Self::Component>];
    fn domain() -> &'static ScalarChannelDomain;

    fn applied_preset_name(applied: &<Self::Component as EffectPreset>::Applied) -> String;

    fn debug_rows() -> Vec<UiRow<PanelView<Self>>>;
}

#[derive(Clone)]
pub(super) struct EffectSectionView<E, R> {
    pub instances: Vec<String>,
    pub instance_index: usize,
    pub preset: Option<String>,
    pub selected: Option<Entity>,
    pub effect: E,
    pub render: R,
}

pub(super) type PanelView<P> =
    EffectSectionView<<P as EffectPanel>::Component, <P as EffectPanel>::RenderSettings>;

type PanelEdit<P> = EffectEdit<<P as EffectPanel>::Component, <P as EffectPanel>::RenderSettings>;

pub(super) fn build_effect_section<P: EffectPanel>(world: &World) -> Option<Box<dyn FormSection>> {
    let view = read_effect_section_view::<P>(world);

    let mut rows = effect_head_rows::<P>(&view);
    if view.selected.is_some() {
        rows.push(UiRow::Field(Box::new(FieldRows {
            scope: "effect",
            field: |view: &mut PanelView<P>| &mut view.effect,
            rows: rows_from_params(P::ui_params(), P::scalar_params(), &[]),
        })));
        rows.extend(effect_tail_rows::<P>());
    }

    let group = UiRow::Group {
        title: P::NAME.into(),
        initially_open: false,
        rows,
    };

    Some(Box::new(RowsSection {
        rows: vec![group],
        view,
        commit: commit_effect_view::<P>,
        keys: Some(P::domain()),
    }))
}

fn read_effect_section_view<P: EffectPanel>(world: &World) -> PanelView<P> {
    let entities = world.entities_with::<P::Component>();
    let selected = resolve_selected_effect::<P::Component>(world);

    let instance_index = selected
        .and_then(|entity| entities.iter().position(|&e| e == entity))
        .unwrap_or(0);
    let (instances, _) = effect_instances(world, &entities, instance_index, P::NAME);
    let preset = selected.and_then(|entity| {
        world
            .get_component::<<P::Component as EffectPreset>::Applied>(entity)
            .map(P::applied_preset_name)
    });
    let render = world
        .get_resource::<P::RenderSettings>()
        .map(|settings| (*settings).clone())
        .unwrap_or_default();
    let effect = selected
        .and_then(|entity| world.get_component::<P::Component>(entity))
        .cloned()
        .unwrap_or_default();

    EffectSectionView {
        instances,
        instance_index,
        preset,
        selected,
        effect,
        render,
    }
}

pub(super) fn effect_head_rows<P: EffectPanel>(view: &PanelView<P>) -> Vec<UiRow<PanelView<P>>> {
    let mut rows = vec![UiRow::Action {
        label: P::ADD_LABEL,
        enabled: |_, _| true,
        on_click: |_| Box::new(ScalarCurveEvent::AddEffect(P::SPAWN_HOOK_KEY)),
    }];

    if view.instances.len() > 1 {
        rows.push(UiRow::Choice {
            label: "Instance",
            options: |_, view: &PanelView<P>| {
                instance_options(&view.instances, view.instance_index)
            },
            pick: |view, index| view.instance_index = index,
        });
    }

    rows.push(UiRow::Choice {
        label: "Preset",
        options: |_, view: &PanelView<P>| preset_options(P::PRESET_NAMES, view.preset.as_deref()),
        pick: |view, index| view.preset = Some(P::PRESET_NAMES[index].to_string()),
    });

    rows
}

fn effect_tail_rows<P: EffectPanel>() -> Vec<UiRow<PanelView<P>>> {
    vec![
        UiRow::Action {
            label: "Curves",
            enabled: |_, _| true,
            on_click: |_| Box::new(ScalarCurveEvent::OpenScalarCurveEditor),
        },
        UiRow::Group {
            title: format!("{} Debug", P::NAME).into(),
            initially_open: false,
            rows: P::debug_rows(),
        },
    ]
}

pub(super) fn commit_effect_view<P: EffectPanel>(
    world: &World,
    shown: &PanelView<P>,
    view: &PanelView<P>,
) {
    if view.instance_index != shown.instance_index {
        world.send_command(OverlayEvent::SelectEffectInstance {
            key: P::SPAWN_HOOK_KEY,
            index: view.instance_index,
        });
    }

    let mut effect_applied_this_frame = false;
    if let (Some(chosen), Some(_)) = (&view.preset, view.selected) {
        if view.preset != shown.preset {
            world.send_command(ScalarCurveEvent::ClearScalarKeys);
            world.send_command(PanelEdit::<P>::ApplyPreset(chosen.clone()));
            effect_applied_this_frame = true;
        }
    }

    let Some(entity) = view.selected else {
        return;
    };
    if !effect_applied_this_frame && view.effect != shown.effect {
        world.send_command(PanelEdit::<P>::UpdateEffect {
            entity,
            effect: Box::new(view.effect.clone()),
        });
    }

    if view.render != shown.render {
        world.send_command(PanelEdit::<P>::UpdateRenderSettings(view.render.clone()));
    }
}
