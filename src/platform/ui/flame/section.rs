use crate::platform::ui::numeric_format::numeric_format;
use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::FLAME_DOMAIN;
use crate::ecs::resource::ViewportInput;
use crate::ecs::systems::flame::FlameUiCommand;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::World;
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::SectionDefault;

use super::super::param_widgets::{rows_from_params, EditedScalars};
use super::super::scene_overlay::send_key_button;
use super::super::ui_rows::draw_rows;
use super::fit_browser::{build_texture_fit_browser, canonical_dir_or};
use super::head_rows::{fit_rows, head_rows, style_rows};
use super::render_settings::draw_flame_render_settings;
use super::section_view::{
    draw_fit_path_status, read_flame_section_view, scan_flame_assets, send_flame_requests,
    write_back_flame_ui,
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

fn draw_flame_manual_params(ui: &imgui::Ui, effect: &mut crate::ecs::component::FlameEffect) {
    let mut noise_sharpness =
        thyllore_effect_core::shaping_scale_to_noise_sharpness(effect.noise.shaping_scale);
    if ui
        .slider_config("Noise Sharpness", 0.0, 1.0)
        .display_format(numeric_format("%.2f"))
        .build(&mut noise_sharpness)
    {
        effect.noise.shaping_scale =
            thyllore_effect_core::noise_sharpness_to_shaping_scale(noise_sharpness);
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Crispness of the noise pattern: log remap of the tanh shaping \
             scale, harder edges to the right (small scale saturates the \
             tanh into near-binary blobs; ~0.78 = scale 0.25, the measured \
             perceptual sweet spot). Stateless — noise_shaping_scale stays \
             the source of truth (0 = built-in 0.6)",
        );
    }

    let mut wave_segments = effect.wave_segments as i32;
    if ui
        .slider_config(
            "Noise Segments",
            thyllore_effect_core::WAVE_SEGMENTS_MIN as i32,
            thyllore_effect_core::WAVE_SEGMENTS_MAX as i32,
        )
        .build(&mut wave_segments)
    {
        effect.wave_segments = wave_segments as u32;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Closed-form segments per ray: the noise grid aliases into a \
             pixel hatch above Noise Frequency ~2 at 64; 128 resolves \
             frequency ~4 at twice the cost",
        );
    }

    let mut vortex =
        (effect.twist.gain / thyllore_effect_core::VORTEX_MACRO_MAX_GAIN).clamp(0.0, 1.0);
    if ui
        .slider_config("Vortex", 0.0, 1.0)
        .display_format(numeric_format("%.2f"))
        .build(&mut vortex)
    {
        let (gain, speed) = thyllore_effect_core::vortex_macro_parameters(vortex);
        effect.twist.gain = gain;
        effect.twist.speed = speed;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Vortex macro: one knob writing both twist parameters along a \
             faster-and-deeper curve (stateless; the fine sliders \
             stay the source of truth)",
        );
    }

    let mut branch_seed = effect.branch.seed as i32;
    if ui.input_int("Branch Seed", &mut branch_seed) {
        effect.branch.seed = branch_seed.max(0) as u32;
    }
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
        {
            if let Some(selected_flame) = selected_flame_entity {
                if let Some(effect) = ecs_world.get_component::<FlameEffect>(selected_flame) {
                    let mut effect_copy = effect.clone();

                    let mut position = [
                        effect_copy.position.x,
                        effect_copy.position.y,
                        effect_copy.position.z,
                    ];
                    if ui.input_float3("Position", &mut position).build() {
                        effect_copy.position =
                            cgmath::Vector3::new(position[0], position[1], position[2]);
                    }

                    let emitter_labels: [&str; 3] = ["Cylinder", "Ring", "Mesh SDF"];
                    let mut emitter_selected = effect_copy.emitter.kind as usize;
                    if ui.combo_simple_string("Emitter", &mut emitter_selected, &emitter_labels) {
                        effect_copy.emitter.kind = emitter_selected as u32;
                    }

                    if effect_copy.emitter.kind == 1 {
                        ui.slider_config("Ring Radius", 0.2, 5.0)
                            .display_format(numeric_format("%.2f"))
                            .build(&mut effect_copy.emitter.ring_major_radius);
                        ui.same_line();
                        let mut ring_speed = effect_copy.emitter.ring_angular_speed;
                        ui.slider_config("Ring Speed", 0.0, 6.28)
                            .display_format(numeric_format("%.2f"))
                            .build(&mut ring_speed);
                        effect_copy.emitter.ring_angular_speed = ring_speed;
                    }

                    let colors_before = (effect_copy.color.base, effect_copy.color.tip);
                    let rows = rows_from_params::<thyllore_effect_core::FlameEffect>(
                        &thyllore_effect_core::FLAME_UI_PARAMS,
                        &thyllore_effect_core::FLAME_SCALAR_PARAMS,
                        &[],
                    );
                    draw_rows(ui, ecs_world, &rows, &mut effect_copy, |ui, edited| {
                        flame_key_button(ui, ecs_world, edited);
                    });
                    let advanced_open = ui.get_id("Advanced").raw();
                    let is_advanced_open = match ecs_world
                        .get_resource::<crate::ecs::resource::UiWidgetState>()
                    {
                        Some(state) => *state.section_open.get(&advanced_open).unwrap_or(&false),
                        None => false,
                    };
                    if is_advanced_open {
                        draw_flame_manual_params(ui, &mut effect_copy);
                    }
                    if (effect_copy.color.base, effect_copy.color.tip) != colors_before {
                        effect_copy.color.use_blackbody = false;
                    }

                    if ui.button("Clear Flame Keys") {
                        ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
                    }
                    ui.same_line();
                    if ui.button("Random Keys (Debug)") {
                        let seed = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        ecs_world.send_command(ScalarCurveEvent::InsertScalarDebugKeys { seed });
                    }
                    if ui.button("Curves") {
                        ecs_world.send_command(ScalarCurveEvent::OpenScalarCurveEditor);
                    }

                    // Trail checkbox and slider
                    let trail_state = ecs_world
                        .get_component::<crate::ecs::component::FlameTrail>(selected_flame)
                        .map(|t| (t.state.enabled, t.state.fade_seconds))
                        .unwrap_or((false, 0.8));
                    let mut trail_enabled = trail_state.0;
                    let mut trail_fade = trail_state.1;
                    if ui.checkbox("Trail", &mut trail_enabled) {
                        ecs_world.send_command(FlameUiCommand::UpdateTrailEnabled(trail_enabled));
                    }
                    ui.slider_config("Trail Fade", 0.1, 5.0)
                        .build(&mut trail_fade);
                    if (trail_fade - trail_state.1).abs() > 0.01 {
                        ecs_world.send_command(FlameUiCommand::UpdateTrailFade(trail_fade));
                    }

                    // GPU Timings section (read-only)
                    let timings = ecs_world.get_resource::<crate::ecs::resource::GpuPassTimings>();
                    if let Some(timings) = timings {
                        if !timings.passes.is_empty() {
                            ui.separator();
                            ui.text("GPU Timings");
                            for (label, ms) in &timings.passes {
                                ui.text(format!("  {} {:.3} ms", label, ms));
                            }
                        }
                    }

                    if !effect_applied_this_frame {
                        ecs_world.send_command(FlameUiCommand::UpdateEffect {
                            entity: selected_flame,
                            effect: Box::new(effect_copy),
                        });
                    }

                    if section_header(ui, ecs_world, "Flame Debug", SectionDefault::Closed) {
                        draw_flame_render_settings(ui, ecs_world);
                        if ui.button("Dump Probe") {
                            ecs_world.send_command(FlameUiCommand::DumpWallProbe {
                                viewport_size: ecs_world.resource::<ViewportInput>().size,
                            });
                        }
                        if ui.is_item_hovered() {
                            ui.tooltip_text(
                                "Dump camera pose + wall-regime ray diagnostics to log/flame/",
                            );
                        }
                    }
                }
            }
        }
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "flame",
    order: 4,
    draw: build_flame_section,
});
