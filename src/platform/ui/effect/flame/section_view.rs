use crate::ecs::resource::{FlameUIState, GpuPassTimings, ViewportInput};
use crate::ecs::systems::flame::{FlameUiCommand, FLAMES_STYLE_DIR, FLAMES_TEXTURE_DIR};
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::FLAME_SPAWN_HOOK;
use crate::ecs::world::Entity;
use crate::ecs::World;

use super::body_rows::FlameBodyView;
use super::fit_browser::validate_texture_fit_path;
use crate::platform::ui::effect::rows::effect_instances;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(super) struct FlameRequests {
    pub apply_fit: bool,
    pub apply_style: bool,
    pub save_style: bool,
}

#[derive(Clone)]
pub(super) struct FlameSectionView {
    pub instances: Vec<String>,
    pub instance_index: usize,
    pub preset: Option<String>,
    pub flame_ui: FlameUIState,
    pub applied_style: Option<(String, u32)>,
    pub selected: Option<Entity>,
    pub body: FlameBodyView,
    pub requests: FlameRequests,
}

fn scan_flame_assets(flame_ui: &mut FlameUIState) {
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
}

pub(super) fn read_flame_section_view(
    world: &World,
    flames: &[Entity],
    selected: Option<Entity>,
) -> FlameSectionView {
    let selected_index = selected
        .and_then(|entity| flames.iter().position(|&e| e == entity))
        .unwrap_or(0);
    let (instances, instance_index) = effect_instances(world, flames, selected_index, "Flame");

    let preset = selected.and_then(|entity| {
        world
            .get_component::<crate::ecs::component::AppliedFlamePreset>(entity)
            .map(|p| p.name.clone())
    });

    let applied_style = selected.and_then(|entity| {
        world
            .get_component::<crate::ecs::component::AppliedFlameStyle>(entity)
            .map(|s| (s.name.clone(), s.version))
    });

    let mut flame_ui = world.resource::<FlameUIState>().clone();
    scan_flame_assets(&mut flame_ui);
    validate_fit_path(&mut flame_ui);

    FlameSectionView {
        instances,
        instance_index,
        preset,
        applied_style,
        selected,
        body: read_flame_body_view(world, selected),
        flame_ui,
        requests: FlameRequests::default(),
    }
}

fn validate_fit_path(flame_ui: &mut FlameUIState) {
    if flame_ui.texture_fit_path != flame_ui.texture_fit_path_validated {
        flame_ui.texture_fit_path_info = validate_texture_fit_path(&flame_ui.texture_fit_path);
        flame_ui.texture_fit_path_validated = flame_ui.texture_fit_path.clone();
    }
}

pub(super) fn draw_fit_path_status(ui: &imgui::Ui, flame_ui: &FlameUIState) {
    if flame_ui.texture_fit_path.is_empty() {
        ui.text_disabled("enter a texture path");
    } else if flame_ui.texture_fit_path_info.starts_with("ok:") {
        ui.text_colored([0.3, 0.9, 0.3, 1.0], &flame_ui.texture_fit_path_info);
    } else {
        ui.text_colored([0.9, 0.3, 0.3, 1.0], &flame_ui.texture_fit_path_info);
    }
}

fn chosen_preset<'a>(shown: &FlameSectionView, view: &'a FlameSectionView) -> Option<&'a String> {
    view.preset.as_ref().filter(|_| view.preset != shown.preset)
}

fn requested_style_path(view: &FlameSectionView) -> Option<String> {
    if !view.requests.apply_style {
        return None;
    }
    let flame_ui = &view.flame_ui;
    flame_ui
        .style_scan
        .get(flame_ui.style_index)
        .map(|name| format!("{}/{}", FLAMES_STYLE_DIR, name))
}

fn requested_save_name(view: &FlameSectionView) -> Option<String> {
    let name = view.flame_ui.style_save_name.trim();
    (view.requests.save_style && !name.is_empty()).then(|| name.to_string())
}

fn is_apply_sent(shown: &FlameSectionView, view: &FlameSectionView) -> bool {
    view.selected.is_some()
        && (chosen_preset(shown, view).is_some()
            || view.requests.apply_fit
            || requested_style_path(view).is_some())
}

pub(super) fn send_flame_requests(
    world: &World,
    shown: &FlameSectionView,
    view: &FlameSectionView,
) -> bool {
    if view.instance_index != shown.instance_index {
        world.send_command(OverlayEvent::SelectEffectInstance {
            key: FLAME_SPAWN_HOOK.key,
            index: view.instance_index,
        });
    }
    if view.selected.is_none() {
        return false;
    }

    if let Some(name) = chosen_preset(shown, view) {
        world.send_command(ScalarCurveEvent::ClearScalarKeys);
        world.send_command(FlameUiCommand::ApplyPreset(name.clone()));
    }

    let flame_ui = &view.flame_ui;
    if view.requests.apply_fit {
        let groups = thyllore_effect_core::TextureFitGroups {
            silhouette: flame_ui.texture_fit_groups[0],
            color: flame_ui.texture_fit_groups[1],
            turbulence: flame_ui.texture_fit_groups[2],
            tilt: flame_ui.texture_fit_groups[3],
        };
        world.send_command(FlameUiCommand::ApplyTextureFit {
            path: flame_ui.texture_fit_path.clone(),
            blend: flame_ui.texture_fit_blend,
            groups: [
                groups.silhouette,
                groups.color,
                groups.turbulence,
                groups.tilt,
            ],
            profile: flame_ui.texture_fit_profile,
        });
    }

    if let Some(path) = requested_style_path(view) {
        world.send_command(FlameUiCommand::ApplyStyle {
            path,
            groups: flame_ui.style_groups,
        });
    }

    if let Some(name) = requested_save_name(view) {
        world.send_command(FlameUiCommand::SaveStyle { name });
    }

    is_apply_sent(shown, view)
}

fn write_back_flame_ui(world: &World, flame_ui: &FlameUIState) {
    if let Some(mut resource) = world.get_resource_mut::<FlameUIState>() {
        *resource = flame_ui.clone();
    }
}

fn read_flame_body_view(world: &World, flame: Option<Entity>) -> FlameBodyView {
    let Some(flame) = flame else {
        return FlameBodyView::default();
    };
    let effect = world
        .get_component::<crate::ecs::component::FlameEffect>(flame)
        .cloned()
        .unwrap_or_default();
    let trail = world
        .get_component::<crate::ecs::component::FlameTrail>(flame)
        .map(|t| (t.state.enabled, t.state.fade_seconds))
        .unwrap_or((false, 0.8));
    let render = world
        .get_resource::<crate::ecs::resource::FlameRenderSettings>()
        .map(|s| *s);
    let jitter = thyllore_effect_core::flame_wave::read_env_wave_jitter();
    let jitter_freq = thyllore_effect_core::flame_wave::read_env_wave_jitter_freq();
    let viewport_size = world
        .get_resource::<ViewportInput>()
        .map(|viewport| viewport.size)
        .unwrap_or_default();
    let gpu_timings = match world.get_resource::<GpuPassTimings>() {
        Some(t) => t.passes.clone(),
        None => Vec::new(),
    };
    FlameBodyView {
        effect,
        trail_enabled: trail.0,
        trail_fade: trail.1,
        render: render.unwrap_or_default(),
        jitter,
        jitter_freq,
        viewport_size,
        gpu_timings,
    }
}

pub(super) fn commit_flame_view(world: &World, shown: &FlameSectionView, view: &FlameSectionView) {
    let effect_applied_this_frame = send_flame_requests(world, shown, view);
    if view.flame_ui != shown.flame_ui {
        write_back_flame_ui(world, &view.flame_ui);
    }
    if let Some(flame) = view.selected {
        send_flame_body_changes(
            world,
            flame,
            &shown.body,
            &view.body,
            effect_applied_this_frame,
        );
    }
}

fn edited_effect(
    shown: &FlameBodyView,
    view: &FlameBodyView,
) -> Option<crate::ecs::component::FlameEffect> {
    if view.effect == shown.effect {
        return None;
    }
    let mut effect = view.effect.clone();
    let color_edited =
        (effect.color.base, effect.color.tip) != (shown.effect.color.base, shown.effect.color.tip);
    if color_edited {
        effect.color.use_blackbody = false;
    }
    Some(effect)
}

fn send_flame_body_changes(
    world: &World,
    flame: Entity,
    shown: &FlameBodyView,
    view: &FlameBodyView,
    effect_applied_this_frame: bool,
) {
    if !effect_applied_this_frame {
        if let Some(effect) = edited_effect(shown, view) {
            world.send_command(FlameUiCommand::UpdateEffect {
                entity: flame,
                effect: Box::new(effect),
            });
        }
    }
    if view.trail_enabled != shown.trail_enabled {
        world.send_command(FlameUiCommand::UpdateTrailEnabled(view.trail_enabled));
    }
    if (view.trail_fade - shown.trail_fade).abs() > 0.01 {
        world.send_command(FlameUiCommand::UpdateTrailFade(view.trail_fade));
    }
    if view.render != shown.render {
        world.send_command(FlameUiCommand::UpdateRenderSettings(view.render.clone()));
    }
    if view.jitter != shown.jitter {
        thyllore_effect_core::flame_wave::set_wave_jitter(view.jitter);
    }
    if view.jitter_freq != shown.jitter_freq {
        thyllore_effect_core::flame_wave::set_wave_jitter_freq(view.jitter_freq);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected_view(preset: Option<&str>) -> FlameSectionView {
        FlameSectionView {
            instances: Vec::new(),
            instance_index: 0,
            preset: preset.map(str::to_string),
            flame_ui: FlameUIState::default(),
            applied_style: None,
            selected: Some(0),
            body: FlameBodyView::default(),
            requests: FlameRequests::default(),
        }
    }

    #[test]
    fn unchanged_view_sends_no_apply() {
        let shown = selected_view(Some("candle"));
        let view = selected_view(Some("candle"));
        assert!(!is_apply_sent(&shown, &view));
    }

    #[test]
    fn changed_preset_is_an_apply_only_with_a_selection() {
        let shown = selected_view(Some("candle"));
        let mut view = selected_view(Some("torch"));
        assert_eq!(
            chosen_preset(&shown, &view).map(String::as_str),
            Some("torch")
        );
        assert!(is_apply_sent(&shown, &view));

        view.selected = None;
        assert!(!is_apply_sent(&shown, &view));
    }

    #[test]
    fn texture_fit_request_is_an_apply() {
        let shown = selected_view(None);
        let mut view = selected_view(None);
        view.requests.apply_fit = true;
        assert!(is_apply_sent(&shown, &view));
    }

    #[test]
    fn style_request_needs_a_scanned_style() {
        let shown = selected_view(None);
        let mut view = selected_view(None);
        view.requests.apply_style = true;
        assert_eq!(requested_style_path(&view), None);
        assert!(!is_apply_sent(&shown, &view));

        view.flame_ui.style_scan = vec!["a.style.ron".to_string()];
        assert_eq!(
            requested_style_path(&view),
            Some(format!("{}/a.style.ron", FLAMES_STYLE_DIR))
        );
        assert!(is_apply_sent(&shown, &view));
    }

    #[test]
    fn save_style_is_trimmed_and_not_an_apply() {
        let shown = selected_view(None);
        let mut view = selected_view(None);
        view.requests.save_style = true;
        view.flame_ui.style_save_name = "   ".to_string();
        assert_eq!(requested_save_name(&view), None);

        view.flame_ui.style_save_name = "  ember ".to_string();
        assert_eq!(requested_save_name(&view).as_deref(), Some("ember"));
        assert!(!is_apply_sent(&shown, &view));
    }

    #[test]
    fn read_view_points_at_the_selected_flame() {
        let mut world = World::new();
        world.insert_resource(FlameUIState::default());
        let flames: Vec<Entity> = vec![3, 7];

        let view = read_flame_section_view(&world, &flames, Some(7));
        assert_eq!(view.instance_index, 1);
        assert_eq!(view.selected, Some(7));
        assert_eq!(view.requests, FlameRequests::default());

        let view = read_flame_section_view(&world, &flames, None);
        assert_eq!(view.instance_index, 0);
        assert_eq!(view.selected, None);
    }

    #[test]
    fn edited_effect_drops_blackbody_only_when_color_changes() {
        let shown = FlameBodyView::default();
        assert!(edited_effect(&shown, &shown.clone()).is_none());

        let mut view = shown.clone();
        view.effect.color.use_blackbody = true;
        view.effect.wave_segments += 1;
        let effect = edited_effect(&shown, &view).expect("changed effect");
        assert!(effect.color.use_blackbody);

        let mut view = shown.clone();
        view.effect.color.use_blackbody = true;
        view.effect.color.tip[0] += 0.1;
        let effect = edited_effect(&shown, &view).expect("changed effect");
        assert!(!effect.color.use_blackbody);
    }
}
