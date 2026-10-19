use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::FLAME_DOMAIN;
use crate::ecs::World;
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::SectionDefault;

use super::super::param_widgets::EditedScalars;
use super::super::scene_overlay::send_key_button;
use super::super::ui_rows::draw_rows;
use super::body_rows::{body_action_rows, debug_rows, emitter_rows, flame_param_rows};
use super::fit_browser::{build_texture_fit_browser, canonical_dir_or};
use super::head_rows::{fit_rows, head_rows, style_rows};
use super::section_view::{
    draw_fit_path_status, read_flame_body_view, read_flame_section_view, scan_flame_assets,
    send_flame_body_changes, send_flame_requests, write_back_flame_ui,
};

fn flame_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            FLAME_DOMAIN
                .property_type_for_cli_name(name)
                .map(|property_type| (property_type, *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

pub(super) fn build_flame_section(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::component::FlameEffect;

    if section_header(ui, ecs_world, "Flame", SectionDefault::Closed) {
        let _section_id = ui.push_id("flame");
        let flames = ecs_world.entities_with::<FlameEffect>();
        let selected_flame_entity = crate::ecs::systems::resolve_selected_flame(ecs_world);
        let shown = read_flame_section_view(ecs_world, &flames, selected_flame_entity);
        let mut view = shown.clone();
        scan_flame_assets(&mut view.flame_ui);
        draw_rows(ui, ecs_world, &head_rows(&view), &mut view, |_, _| {});
        draw_rows(ui, ecs_world, &fit_rows(&view), &mut view, |_, _| {});
        draw_fit_path_status(ui, &mut view.flame_ui);
        if view.flame_ui.texture_fit_browser_open && !shown.flame_ui.texture_fit_browser_open {
            view.flame_ui.texture_fit_browser_dir =
                canonical_dir_or(&view.flame_ui.texture_fit_browser_dir);
        }
        build_texture_fit_browser(ui, &mut view.flame_ui);
        draw_rows(ui, ecs_world, &style_rows(&view), &mut view, |_, _| {});
        let effect_applied_this_frame = send_flame_requests(ecs_world, &shown, &view);
        write_back_flame_ui(ecs_world, view.flame_ui);
        let Some(selected_flame) = selected_flame_entity else {
            return;
        };
        let Some(effect) = ecs_world
            .get_component::<FlameEffect>(selected_flame)
            .map(|effect| effect.clone())
        else {
            return;
        };
        let shown_body = read_flame_body_view(ecs_world, selected_flame, effect);
        let mut body = shown_body.clone();
        draw_rows(ui, ecs_world, &emitter_rows(&body), &mut body, |_, _| {});
        draw_rows(
            ui,
            ecs_world,
            &flame_param_rows(),
            &mut body.effect,
            |ui, edited| {
                flame_key_button(ui, ecs_world, edited);
            },
        );
        if (body.effect.color.base, body.effect.color.tip)
            != (shown_body.effect.color.base, shown_body.effect.color.tip)
        {
            body.effect.color.use_blackbody = false;
        }
        draw_rows(ui, ecs_world, &body_action_rows(), &mut body, |_, _| {});
        draw_rows(ui, ecs_world, &debug_rows(&body), &mut body, |_, _| {});
        send_flame_body_changes(
            ecs_world,
            selected_flame,
            &shown_body,
            &body,
            effect_applied_this_frame,
        );
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "flame",
    order: 4,
    section: |_| Some(Box::new(crate::platform::ui::form_window::CanvasSection {
        draw: build_flame_section
    })),
});
