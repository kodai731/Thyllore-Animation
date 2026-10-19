use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::{
    AppliedLightningPreset, LightningEffect, LightningPath, LightningTarget, LIGHTNING_DOMAIN,
};
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::systems::lightning::{resolve_selected_lightning, LightningUiCommand};
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::LIGHTNING_SPAWN_HOOK;
use crate::ecs::world::Entity;
use crate::ecs::World;

use super::effect_rows::effect_instances;
use super::form_window::{FormSection, RowsSection};
use super::lightning_rows::{lightning_head_rows, lightning_tail_rows, LightningSectionView};
use super::param_widgets::{rows_from_params, EditedScalars};
use super::scene_overlay::send_key_button;
use super::ui_rows::{FieldRows, UiRow};

fn lightning_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            LIGHTNING_DOMAIN
                .property_type_for_cli_name(name)
                .map(|property_type| (property_type, *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

fn read_lightning_section_view(
    ecs_world: &World,
    lightnings: &[Entity],
    selected_entity: Option<Entity>,
) -> LightningSectionView {
    let instance_index = selected_entity
        .and_then(|entity| lightnings.iter().position(|&e| e == entity))
        .unwrap_or(0);
    let (instances, _) = effect_instances(ecs_world, lightnings, instance_index, "Lightning");
    let preset = selected_entity.and_then(|entity| {
        ecs_world
            .get_component::<AppliedLightningPreset>(entity)
            .map(|preset| preset.name.clone())
    });
    let target_name = selected_entity.and_then(|entity| {
        ecs_world
            .get_component::<LightningTarget>(entity)
            .map(|target| target.entity_name.clone())
    });
    let waypoints = selected_entity
        .and_then(|entity| {
            ecs_world
                .get_component::<LightningPath>(entity)
                .map(|path| path.waypoints.clone())
        })
        .unwrap_or_default();
    let render = ecs_world
        .get_resource::<LightningRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default();
    let effect = selected_entity
        .and_then(|entity| ecs_world.get_component::<LightningEffect>(entity))
        .cloned()
        .unwrap_or_default();

    LightningSectionView {
        instances,
        instance_index,
        preset,
        target_name,
        waypoints,
        selected: selected_entity,
        effect,
        render,
    }
}

fn effect_rows(view: &LightningSectionView) -> UiRow<LightningSectionView> {
    let hidden: &[&str] = if view.target_name.is_some() {
        &["shape_end_offset"]
    } else {
        &[]
    };
    UiRow::Field(Box::new(FieldRows {
        field: |view: &mut LightningSectionView| &mut view.effect,
        rows: rows_from_params(
            &thyllore_effect_core::LIGHTNING_UI_PARAMS,
            &thyllore_effect_core::LIGHTNING_SCALAR_PARAMS,
            hidden,
        ),
    }))
}

fn lightning_section(world: &World) -> Option<Box<dyn FormSection>> {
    let lightnings = world.entities_with::<LightningEffect>();
    let selected_entity = resolve_selected_lightning(world);
    let view = read_lightning_section_view(world, &lightnings, selected_entity);

    let mut rows = lightning_head_rows(&view);
    if view.selected.is_some() {
        rows.push(effect_rows(&view));
        rows.extend(lightning_tail_rows());
    }

    let group = UiRow::Group {
        title: "Lightning".into(),
        initially_open: false,
        rows,
    };

    Some(Box::new(RowsSection {
        rows: vec![group],
        view,
        commit: commit_lightning_view,
        after_item: Some(lightning_key_button),
    }))
}

fn commit_lightning_view(world: &World, shown: &LightningSectionView, view: &LightningSectionView) {
    if view.instance_index != shown.instance_index {
        world.send_command(OverlayEvent::SelectEffectInstance {
            key: LIGHTNING_SPAWN_HOOK.key,
            index: view.instance_index,
        });
    }

    let mut effect_applied_this_frame = false;
    if let (Some(chosen), Some(_)) = (&view.preset, view.selected) {
        if view.preset != shown.preset {
            world.send_command(ScalarCurveEvent::ClearScalarKeys);
            world.send_command(LightningUiCommand::ApplyPreset(chosen.clone()));
            effect_applied_this_frame = true;
        }
    }

    let Some(entity) = view.selected else {
        return;
    };
    if !effect_applied_this_frame && view.effect != shown.effect {
        world.send_command(LightningUiCommand::UpdateEffect {
            entity,
            effect: Box::new(view.effect.clone()),
        });
    }

    if view.render != shown.render {
        world.send_command(LightningUiCommand::UpdateRenderSettings(view.render));
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "lightning",
    order: 3,
    section: lightning_section,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;

    fn selected_view() -> LightningSectionView {
        LightningSectionView {
            instances: vec!["Lightning 1".to_string()],
            instance_index: 0,
            preset: None,
            target_name: None,
            waypoints: Vec::new(),
            selected: Some(0),
            effect: LightningEffect::default(),
            render: LightningRenderSettings::default(),
        }
    }

    fn committed_command_names(
        shown: &LightningSectionView,
        view: &LightningSectionView,
    ) -> Vec<String> {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        commit_lightning_view(&world, shown, view);

        let names = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        names
    }

    #[test]
    fn commit_without_change_sends_nothing() {
        let shown = selected_view();
        assert!(committed_command_names(&shown, &shown.clone()).is_empty());
    }

    #[test]
    fn commit_preset_change_clears_keys_and_applies_preset_without_update() {
        let shown = selected_view();
        let view = LightningSectionView {
            preset: Some(thyllore_effect_core::LIGHTNING_PRESET_NAMES[0].to_string()),
            ..shown.clone()
        };

        let names = committed_command_names(&shown, &view);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names[0].starts_with("ClearScalarKeys"), "{names:?}");
        assert!(names[1].starts_with("ApplyPreset"), "{names:?}");
    }

    #[test]
    fn commit_effect_change_sends_update() {
        let shown = selected_view();
        let mut view = shown.clone();
        view.effect.time += 1.0;

        let names = committed_command_names(&shown, &view);
        assert!(
            names.iter().any(|name| name.starts_with("UpdateEffect")),
            "{names:?}"
        );
    }
}
