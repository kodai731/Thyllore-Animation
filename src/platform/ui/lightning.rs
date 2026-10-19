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
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::SectionDefault;

use super::effect_rows::effect_instances;
use super::lightning_rows::{lightning_head_rows, lightning_tail_rows, LightningSectionView};
use super::param_widgets::{rows_from_params, EditedScalars};
use super::scene_overlay::send_key_button;
use super::ui_rows::draw_rows;

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

    LightningSectionView {
        instances,
        instance_index,
        preset,
        target_name,
        waypoints,
        render,
    }
}

pub(super) fn build_lightning_section(ui: &imgui::Ui, ecs_world: &World) {
    if !section_header(ui, ecs_world, "Lightning", SectionDefault::Closed) {
        return;
    }
    let _section_id = ui.push_id("lightning");

    let lightnings = ecs_world.entities_with::<LightningEffect>();
    let selected_entity = resolve_selected_lightning(ecs_world);
    let shown = read_lightning_section_view(ecs_world, &lightnings, selected_entity);
    let mut view = shown.clone();

    draw_rows(
        ui,
        ecs_world,
        &lightning_head_rows(&view),
        &mut view,
        |_, _| {},
    );

    if view.instance_index != shown.instance_index {
        ecs_world.send_command(OverlayEvent::SelectEffectInstance {
            key: LIGHTNING_SPAWN_HOOK.key,
            index: view.instance_index,
        });
    }

    let mut effect_applied_this_frame = false;
    if let (Some(chosen), Some(_)) = (&view.preset, selected_entity) {
        if view.preset != shown.preset {
            ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
            ecs_world.send_command(LightningUiCommand::ApplyPreset(chosen.clone()));
            effect_applied_this_frame = true;
        }
    }

    let Some(selected) = selected_entity else {
        return;
    };
    let Some(effect) = ecs_world.get_component::<LightningEffect>(selected) else {
        return;
    };
    let mut effect_copy = effect.clone();
    let hidden: &[&str] = if view.target_name.is_some() {
        &["shape_end_offset"]
    } else {
        &[]
    };
    let rows = rows_from_params(
        &thyllore_effect_core::LIGHTNING_UI_PARAMS,
        &thyllore_effect_core::LIGHTNING_SCALAR_PARAMS,
        hidden,
    );
    draw_rows(ui, ecs_world, &rows, &mut effect_copy, |ui, edited| {
        lightning_key_button(ui, ecs_world, edited);
    });
    if !effect_applied_this_frame {
        ecs_world.send_command(LightningUiCommand::UpdateEffect {
            entity: selected,
            effect: Box::new(effect_copy),
        });
    }

    draw_rows(ui, ecs_world, &lightning_tail_rows(), &mut view, |_, _| {});
    if view.render != shown.render {
        ecs_world.send_command(LightningUiCommand::UpdateRenderSettings(view.render));
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "lightning",
    order: 3,
    section: |_| Some(Box::new(crate::platform::ui::form_window::CanvasSection {
        draw: build_lightning_section
    })),
});
