use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::FlameParam;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::FlameUIState;
use crate::ecs::systems::flame::{FlameUiCommand, FLAMES_STYLE_DIR, FLAMES_TEXTURE_DIR};
use crate::ecs::systems::FLAME_SPAWN_HOOK;
use crate::ecs::World;

use super::param_widgets::{draw_preset_combo, draw_tiered_params, EditedScalars};
use super::scene_overlay::send_key_button;
use super::SceneOverlayState;

fn flame_key_button(ui: &imgui::Ui, ui_events: &mut UIEventQueue, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            FlameParam::from_cli_name(name).map(|param| (param.property_type(), *value))
        })
        .collect();
    send_key_button(ui, ui_events, edited, keys);
}

fn draw_flame_manual_params(ui: &imgui::Ui, effect: &mut crate::ecs::component::FlameEffect) {
    let mut noise_sharpness =
        thyllore_effect_core::shaping_scale_to_noise_sharpness(effect.noise.shaping_scale);
    if ui
        .slider_config("Noise Sharpness", 0.0, 1.0)
        .display_format("%.2f")
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
        .display_format("%.2f")
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
    if ui.input_int("Branch Seed", &mut branch_seed).build() {
        effect.branch.seed = branch_seed.max(0) as u32;
    }
}

pub(super) fn build_flame_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    overlay_state: &mut SceneOverlayState,
    ecs_world: &World,
) {
    use crate::ecs::component::FlameEffect;

    if ui.collapsing_header("Flame", imgui::TreeNodeFlags::empty()) {
        let _section_id = ui.push_id("flame");
        let flames = ecs_world.entities_with::<FlameEffect>();
        let selected_flame_entity = crate::ecs::systems::resolve_selected_flame(ecs_world);
        let clamped_index = selected_flame_entity
            .and_then(|entity| flames.iter().position(|&e| e == entity))
            .unwrap_or(0);

        // Instance selector combo (when >1 flame)
        if flames.len() > 1 {
            let mut current = clamped_index;
            let items: Vec<String> = flames
                .iter()
                .enumerate()
                .map(|(i, &entity)| {
                    ecs_world
                        .get_component::<crate::ecs::world::Name>(entity)
                        .map(|n| n.0.clone())
                        .unwrap_or_else(|| format!("Flame {}", i + 1))
                })
                .collect();
            if ui.combo_simple_string("Instance", &mut current, &items) {
                ui_events.send(UIEvent::SelectEffectInstance {
                    key: FLAME_SPAWN_HOOK.key,
                    index: current,
                });
            }
        }

        let applied_preset = selected_flame_entity.and_then(|entity| {
            ecs_world
                .get_component::<crate::ecs::component::AppliedFlamePreset>(entity)
                .map(|preset| preset.name.clone())
        });
        // The slider block below re-sends the (pre-apply) effect every frame;
        // that send must be skipped on the frame an Apply button fires or it
        // overwrites the applied preset/fit in the same dispatch.
        let mut effect_applied_this_frame = false;
        {
            let mut flame_ui = ecs_world.resource_mut::<FlameUIState>();
            if let Some(chosen) = draw_preset_combo(
                ui,
                "Flame Preset",
                thyllore_effect_core::FLAME_PRESET_NAMES,
                applied_preset.as_deref(),
            ) {
                if selected_flame_entity.is_some() {
                    // Keyed scalar curves re-stamp their channels every
                    // frame and would silently pin the old look, so a
                    // preset stamp also clears them (undo restores).
                    ui_events.send(UIEvent::ClearScalarKeys);
                    ui_events.send(UIEvent::Effect {
                        key: FLAME_SPAWN_HOOK.key,
                        command: std::rc::Rc::new(FlameUiCommand::ApplyPreset(chosen)),
                    });
                    effect_applied_this_frame = true;
                }
            }

            ui.separator();
            ui.text("Texture Fit");

            // Scan textures on first frame
            if !flame_ui.texture_fit_scan_done {
                let scan_dir = std::path::Path::new(FLAMES_TEXTURE_DIR);
                if let Ok(entries) = std::fs::read_dir(scan_dir) {
                    for entry in entries.flatten() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "png" {
                                if let Some(name) = entry.file_name().to_str() {
                                    flame_ui.texture_fit_scan.push(name.to_string());
                                }
                            }
                        }
                    }
                }
                flame_ui.texture_fit_scan_done = true;
            }

            // Combo box for texture selection
            let mut scan_items: Vec<String> = vec!["(custom path)".to_string()];
            scan_items.extend(flame_ui.texture_fit_scan.iter().cloned());
            let mut scan_selected = 0usize;
            if ui.combo_simple_string("Fit Texture", &mut scan_selected, &scan_items) {
                if scan_selected > 0 {
                    let name = &flame_ui.texture_fit_scan[scan_selected - 1];
                    flame_ui.texture_fit_path = format!("{}/{}", FLAMES_TEXTURE_DIR, name);
                } else {
                    flame_ui.texture_fit_path.clear();
                }
            }
            ui.same_line();
            if ui.small_button("Rescan") {
                flame_ui.texture_fit_scan_done = false;
                flame_ui.texture_fit_scan.clear();
            }

            ui.input_text("Fit Image (png)", &mut flame_ui.texture_fit_path)
                .build();
            if ui.small_button("Browse...") {
                flame_ui.texture_fit_browser_open = true;
                if flame_ui.texture_fit_browser_dir.is_empty() {
                    flame_ui.texture_fit_browser_dir = FLAMES_TEXTURE_DIR.to_string();
                }
                flame_ui.texture_fit_browser_dir =
                    canonical_dir_or(&flame_ui.texture_fit_browser_dir);
            }
            ui.same_line();

            // Validation indicator: existence plus a lightweight PNG header read,
            // cached per path so the header is only parsed when the path changes.
            if flame_ui.texture_fit_path != flame_ui.texture_fit_path_validated {
                flame_ui.texture_fit_path_info =
                    validate_texture_fit_path(&flame_ui.texture_fit_path);
                flame_ui.texture_fit_path_validated = flame_ui.texture_fit_path.clone();
            }
            if flame_ui.texture_fit_path.is_empty() {
                ui.text_disabled("enter a texture path");
            } else if flame_ui.texture_fit_path_info.starts_with("ok:") {
                ui.text_colored([0.3, 0.9, 0.3, 1.0], &flame_ui.texture_fit_path_info);
            } else {
                ui.text_colored([0.9, 0.3, 0.3, 1.0], &flame_ui.texture_fit_path_info);
            }

            build_texture_fit_browser(ui, &mut flame_ui);

            ui.slider("Fit Blend", 0.0, 1.0, &mut flame_ui.texture_fit_blend);
            ui.checkbox("Silhouette", &mut flame_ui.texture_fit_groups[0]);
            ui.checkbox("Color", &mut flame_ui.texture_fit_groups[1]);
            {
                let _disabled = ui.begin_disabled(flame_ui.texture_fit_profile);
                ui.checkbox("Turbulence", &mut flame_ui.texture_fit_groups[2]);
            }
            if flame_ui.texture_fit_profile && ui.is_item_hovered() {
                ui.tooltip_text(
                    "Ignored in profile (reproduction) mode: the turbulence \
                     estimate is far below the calibrated pattern amplitude \
                     and would crush the noise",
                );
            }
            ui.checkbox("Tilt", &mut flame_ui.texture_fit_groups[3]);

            // Fidelity radio button
            let mut fidelity_mode: i32 = if flame_ui.texture_fit_profile { 1 } else { 0 };
            if ui.radio_button("statistics (projection)", &mut fidelity_mode, 0) {
                flame_ui.texture_fit_profile = false;
            }
            ui.same_line();
            if ui.radio_button("profile (reproduction)", &mut fidelity_mode, 1) {
                flame_ui.texture_fit_profile = true;
            }

            if ui.button("Apply Texture Fit") {
                let path = flame_ui.texture_fit_path.clone();
                let blend = flame_ui.texture_fit_blend;
                let groups = thyllore_effect_core::TextureFitGroups {
                    silhouette: flame_ui.texture_fit_groups[0],
                    color: flame_ui.texture_fit_groups[1],
                    turbulence: flame_ui.texture_fit_groups[2],
                    tilt: flame_ui.texture_fit_groups[3],
                };
                if selected_flame_entity.is_some() {
                    ui_events.send(UIEvent::Effect {
                        key: FLAME_SPAWN_HOOK.key,
                        command: std::rc::Rc::new(FlameUiCommand::ApplyTextureFit {
                            path: path.clone(),
                            blend,
                            groups: [
                                groups.silhouette,
                                groups.color,
                                groups.turbulence,
                                groups.tilt,
                            ],
                            profile: flame_ui.texture_fit_profile,
                        }),
                    });
                    effect_applied_this_frame = true;
                }
            }

            ui.separator();
            ui.text("Style");

            if !flame_ui.style_scan_done {
                let scan_dir = std::path::Path::new(FLAMES_STYLE_DIR);
                if let Ok(entries) = std::fs::read_dir(scan_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".style.ron") {
                                flame_ui.style_scan.push(name.to_string());
                            }
                        }
                    }
                    flame_ui.style_scan.sort();
                }
                flame_ui.style_scan_done = true;
            }

            if flame_ui.style_scan.is_empty() {
                ui.text_disabled(format!("no styles in {}", FLAMES_STYLE_DIR));
            } else {
                let mut style_index = flame_ui.style_index.min(flame_ui.style_scan.len() - 1);
                ui.combo_simple_string("Style File", &mut style_index, &flame_ui.style_scan);
                flame_ui.style_index = style_index;
            }
            ui.same_line();
            if ui.small_button("Rescan##style") {
                flame_ui.style_scan_done = false;
                flame_ui.style_scan.clear();
            }

            ui.checkbox("Motion##style", &mut flame_ui.style_groups[0]);
            ui.same_line();
            ui.checkbox("Texture##style", &mut flame_ui.style_groups[1]);
            ui.same_line();
            ui.checkbox("Optics##style", &mut flame_ui.style_groups[2]);

            if ui.button("Apply Style") {
                if let Some(name) = flame_ui.style_scan.get(flame_ui.style_index) {
                    if selected_flame_entity.is_some() {
                        ui_events.send(UIEvent::Effect {
                            key: FLAME_SPAWN_HOOK.key,
                            command: std::rc::Rc::new(FlameUiCommand::ApplyStyle {
                                path: format!("{}/{}", FLAMES_STYLE_DIR, name),
                                groups: flame_ui.style_groups,
                            }),
                        });
                        effect_applied_this_frame = true;
                    }
                }
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(
                    "Apply the selected style file's parameters (Style-owned only, \
                     dimensionless: lengths scale with the flame's radius, opacity \
                     via tau0). Fields the style leaves unset keep their current \
                     values",
                );
            }
            if let Some(selected_flame) = selected_flame_entity {
                if let Some(applied) = ecs_world
                    .get_component::<crate::ecs::component::AppliedFlameStyle>(selected_flame)
                {
                    ui.same_line();
                    ui.text_disabled(format!("applied: {} v{}", applied.name, applied.version));
                }
            }

            ui.input_text("Save As##style", &mut flame_ui.style_save_name)
                .build();
            ui.same_line();
            if ui.small_button("Save Style") {
                let name = flame_ui.style_save_name.trim().to_string();
                if !name.is_empty() && selected_flame_entity.is_some() {
                    ui_events.send(UIEvent::Effect {
                        key: FLAME_SPAWN_HOOK.key,
                        command: std::rc::Rc::new(FlameUiCommand::SaveStyle { name }),
                    });
                }
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(
                    "Save the selected flame's current look as \
                     assets/flames/styles/<name>.style.ron (all Style-owned \
                     parameters, lengths over r0, opacity as tau0)",
                );
            }

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
                            .display_format("%.2f")
                            .build(&mut effect_copy.emitter.ring_major_radius);
                        ui.same_line();
                        let mut ring_speed = effect_copy.emitter.ring_angular_speed;
                        ui.slider_config("Ring Speed", 0.0, 6.28)
                            .display_format("%.2f")
                            .build(&mut ring_speed);
                        effect_copy.emitter.ring_angular_speed = ring_speed;
                    }

                    let colors_before = (effect_copy.color.base, effect_copy.color.tip);
                    let advanced_open = draw_tiered_params(
                        ui,
                        thyllore_effect_core::FLAME_UI_PARAMS,
                        thyllore_effect_core::FLAME_SCALAR_PARAMS,
                        &mut effect_copy,
                        &[],
                        |ui, edited| flame_key_button(ui, ui_events, edited),
                    );
                    if advanced_open {
                        draw_flame_manual_params(ui, &mut effect_copy);
                    }
                    if (effect_copy.color.base, effect_copy.color.tip) != colors_before {
                        effect_copy.color.use_blackbody = false;
                    }

                    if ui.button("Clear Flame Keys") {
                        ui_events.send(UIEvent::ClearScalarKeys);
                    }
                    ui.same_line();
                    if ui.button("Random Keys (Debug)") {
                        let seed = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        ui_events.send(UIEvent::InsertScalarDebugKeys { seed });
                    }
                    if ui.button("Curves") {
                        ui_events.send(UIEvent::OpenScalarCurveEditor);
                    }
                    if ui.button("Add Flame") {
                        ui_events.send(UIEvent::AddEffect(FLAME_SPAWN_HOOK.key));
                    }

                    // Trail checkbox and slider
                    let trail_state = ecs_world
                        .get_component::<crate::ecs::component::FlameTrail>(selected_flame)
                        .map(|t| (t.state.enabled, t.state.fade_seconds))
                        .unwrap_or((false, 0.8));
                    let mut trail_enabled = trail_state.0;
                    let mut trail_fade = trail_state.1;
                    if ui.checkbox("Trail", &mut trail_enabled) {
                        ui_events.send(UIEvent::Effect {
                            key: FLAME_SPAWN_HOOK.key,
                            command: std::rc::Rc::new(FlameUiCommand::UpdateTrailEnabled(
                                trail_enabled,
                            )),
                        });
                    }
                    ui.slider_config("Trail Fade", 0.1, 5.0)
                        .build(&mut trail_fade);
                    if (trail_fade - trail_state.1).abs() > 0.01 {
                        ui_events.send(UIEvent::Effect {
                            key: FLAME_SPAWN_HOOK.key,
                            command: std::rc::Rc::new(FlameUiCommand::UpdateTrailFade(trail_fade)),
                        });
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
                        ui_events.send(UIEvent::Effect {
                            key: FLAME_SPAWN_HOOK.key,
                            command: std::rc::Rc::new(FlameUiCommand::UpdateEffect {
                                entity: selected_flame,
                                effect: Box::new(effect_copy),
                            }),
                        });
                    }

                    if ui.collapsing_header("Flame Debug", imgui::TreeNodeFlags::empty()) {
                        draw_flame_render_settings(ui, ui_events, ecs_world);
                        if ui.button("Dump Probe") {
                            ui_events.send(UIEvent::Effect {
                                key: FLAME_SPAWN_HOOK.key,
                                command: std::rc::Rc::new(FlameUiCommand::DumpWallProbe {
                                    viewport_size: overlay_state.viewport.size,
                                }),
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

fn draw_flame_render_settings(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::resource::{FlameDebugView, FlameRenderSettings, FlameShadingMode};
    use thyllore_effect_core::flame_wave::{
        read_env_wave_jitter, read_env_wave_jitter_freq, set_wave_jitter, set_wave_jitter_freq,
    };

    let Some(settings) = ecs_world.get_resource::<FlameRenderSettings>() else {
        return;
    };
    let mut settings_copy = *settings;
    drop(settings);

    if let Some(_token) = ui.begin_combo("Shading Mode", settings_copy.shading_mode.label()) {
        for mode in FlameShadingMode::ALL {
            if ui
                .selectable_config(mode.label())
                .selected(mode == settings_copy.shading_mode)
                .build()
            {
                settings_copy.shading_mode = mode;
            }
        }
    }
    if let Some(_token) = ui.begin_combo("Debug View", settings_copy.debug_view.label()) {
        for view in FlameDebugView::ALL {
            if ui
                .selectable_config(view.label())
                .selected(view == settings_copy.debug_view)
                .build()
            {
                settings_copy.debug_view = view;
            }
        }
    }

    let mut jitter = read_env_wave_jitter();
    if ui
        .slider_config("Jitter Depth", 0.0f32, 2.0f32)
        .build(&mut jitter)
    {
        set_wave_jitter(jitter);
    }
    let mut jitter_freq = read_env_wave_jitter_freq();
    if ui
        .slider_config("Jitter Freq", 0.25f32, 6.0f32)
        .build(&mut jitter_freq)
    {
        set_wave_jitter_freq(jitter_freq);
    }

    match settings_copy.shading_mode {
        FlameShadingMode::ReferenceRaymarch => {
            let mut steps = settings_copy.reference_step_count as i32;
            ui.slider_config("Reference Steps", 8, 512)
                .build(&mut steps);
            settings_copy.reference_step_count = steps.max(1) as u32;
        }
        FlameShadingMode::NoiseRaymarch => {
            let mut steps = settings_copy.noise_step_count as i32;
            ui.slider_config("Noise Steps", 4, 64).build(&mut steps);
            settings_copy.noise_step_count = steps.max(1) as u32;
        }
        FlameShadingMode::Analytic
        | FlameShadingMode::DebugThickness
        | FlameShadingMode::DebugDepthClamp => {}
    }

    ui_events.send(UIEvent::Effect {
        key: FLAME_SPAWN_HOOK.key,
        command: std::rc::Rc::new(FlameUiCommand::UpdateRenderSettings(settings_copy)),
    });
}

/// Canonicalized directory, falling back to the typed text when the path
/// cannot be resolved (broken symlink, permissions, not yet existing).
fn canonical_dir_or(dir: &str) -> String {
    std::fs::canonicalize(dir)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| dir.to_string())
}

/// Lightweight validation for the fit path indicator: existence plus a PNG
/// header read (no pixel decode). "ok: ..." prefixed on success.
fn validate_texture_fit_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    if !std::path::Path::new(path).is_file() {
        return String::from("file not found");
    }
    match std::fs::File::open(path)
        .map_err(|e| e.to_string())
        .and_then(|file| {
            png::Decoder::new(file)
                .read_info()
                .map_err(|e| e.to_string())
        }) {
        Ok(reader) => {
            let info = reader.info();
            format!("ok: {}x{} {:?}", info.width, info.height, info.color_type)
        }
        Err(error) => format!("not a readable png: {error}"),
    }
}

const TEXTURE_FIT_BROWSER_MAX_ENTRIES: usize = 2000;

/// In-app file browser for the fit texture (G9): breadcrumb + direct path
/// input, png filter, directory-first listing, double-click to descend /
/// confirm. Selection only fills the path field — applying stays on the
/// explicit Apply button. Unreadable entries render disabled instead of
/// failing the listing.
fn build_texture_fit_browser(ui: &imgui::Ui, flame_ui: &mut FlameUIState) {
    if !flame_ui.texture_fit_browser_open {
        return;
    }
    let mut open = true;
    let mut confirmed: Option<String> = None;
    ui.window("Select Fit Texture")
        .size([560.0, 430.0], imgui::Condition::FirstUseEver)
        .opened(&mut open)
        .build(|| {
            let dir_now = flame_ui.texture_fit_browser_dir.clone();
            let mut jump: Option<String> = None;

            if ui.small_button("/") {
                jump = Some(String::from("/"));
            }
            let mut accumulated = String::new();
            for (index, part) in dir_now.split('/').filter(|p| !p.is_empty()).enumerate() {
                accumulated.push('/');
                accumulated.push_str(part);
                ui.same_line();
                if ui.small_button(format!("{part}##crumb{index}")) {
                    jump = Some(accumulated.clone());
                }
            }

            ui.input_text("##fit_browser_dir", &mut flame_ui.texture_fit_browser_dir)
                .build();
            ui.same_line();
            if ui.small_button("Go") {
                jump = Some(flame_ui.texture_fit_browser_dir.clone());
            }
            ui.checkbox("all files", &mut flame_ui.texture_fit_browser_show_all);
            ui.same_line();
            ui.checkbox("hidden", &mut flame_ui.texture_fit_browser_show_hidden);
            ui.same_line();
            if ui.small_button("Up") {
                if let Some(parent) = std::path::Path::new(&dir_now).parent() {
                    jump = Some(parent.display().to_string());
                }
            }

            ui.child_window("##fit_browser_list")
                .size([0.0, -34.0])
                .build(|| {
                    let read = match std::fs::read_dir(&dir_now) {
                        Ok(read) => read,
                        Err(error) => {
                            ui.text_colored(
                                [0.9, 0.3, 0.3, 1.0],
                                format!("cannot read directory: {error}"),
                            );
                            return;
                        }
                    };
                    let mut rows: Vec<(String, bool, Option<u64>)> = Vec::new();
                    let mut truncated = false;
                    for entry in read.flatten() {
                        let name = match entry.file_name().into_string() {
                            Ok(name) => name,
                            Err(_) => continue,
                        };
                        if !flame_ui.texture_fit_browser_show_hidden && name.starts_with('.') {
                            continue;
                        }
                        let metadata = entry.metadata().ok();
                        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
                        if !is_dir
                            && !flame_ui.texture_fit_browser_show_all
                            && !name.to_ascii_lowercase().ends_with(".png")
                        {
                            continue;
                        }
                        if rows.len() >= TEXTURE_FIT_BROWSER_MAX_ENTRIES {
                            truncated = true;
                            break;
                        }
                        rows.push((name, is_dir, metadata.map(|m| m.len())));
                    }
                    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                    for (name, is_dir, size) in &rows {
                        let label = if *is_dir {
                            format!("{name}/")
                        } else if let Some(size) = size {
                            format!("{name}  ({:.1} KB)", *size as f64 / 1024.0)
                        } else {
                            format!("{name}  (unreadable)")
                        };
                        if size.is_none() && !is_dir {
                            ui.text_disabled(label);
                            continue;
                        }
                        let selected = !is_dir && *name == flame_ui.texture_fit_browser_selected;
                        let clicked = ui.selectable_config(&label).selected(selected).build();
                        let double_clicked = ui.is_item_hovered()
                            && ui.is_mouse_double_clicked(imgui::MouseButton::Left);
                        if *is_dir {
                            if double_clicked {
                                jump = Some(format!("{}/{}", dir_now.trim_end_matches('/'), name));
                            }
                        } else {
                            if clicked {
                                flame_ui.texture_fit_browser_selected = name.clone();
                            }
                            if double_clicked {
                                confirmed =
                                    Some(format!("{}/{}", dir_now.trim_end_matches('/'), name));
                            }
                        }
                    }
                    if truncated {
                        ui.text_colored(
                            [0.9, 0.7, 0.3, 1.0],
                            format!("listing capped at {TEXTURE_FIT_BROWSER_MAX_ENTRIES} entries"),
                        );
                    }
                });

            let has_selection = !flame_ui.texture_fit_browser_selected.is_empty();
            ui.enabled(has_selection, || {
                if ui.button("Open") {
                    confirmed = Some(format!(
                        "{}/{}",
                        dir_now.trim_end_matches('/'),
                        flame_ui.texture_fit_browser_selected
                    ));
                }
            });
            ui.same_line();
            if ui.button("Cancel") {
                flame_ui.texture_fit_browser_open = false;
            }

            if let Some(target) = jump {
                flame_ui.texture_fit_browser_dir = canonical_dir_or(&target);
                flame_ui.texture_fit_browser_selected.clear();
            }
        });
    if let Some(path) = confirmed {
        flame_ui.texture_fit_path = path;
        flame_ui.texture_fit_browser_open = false;
    }
    if !open {
        flame_ui.texture_fit_browser_open = false;
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "flame",
    order: 4,
    draw: build_flame_section,
});
