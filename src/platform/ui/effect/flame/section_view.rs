use crate::ecs::component::{AppliedFlameStyle, FlameEffect, FlameTrail};
use crate::ecs::resource::{FlameRenderSettings, FlameUIState, GpuPassTimings, ViewportInput};
use crate::ecs::systems::flame::{FlameUiCommand, FLAMES_STYLE_DIR, FLAMES_TEXTURE_DIR};
use crate::ecs::world::{Entity, World};

use super::fit_browser::validate_texture_fit_path;
use crate::platform::ui::effect::sections::EffectSectionView;

const TRAIL_FADE_SECONDS_WITHOUT_TRAIL: f32 = 0.8;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(super) struct FlameRequests {
    pub apply_fit: bool,
    pub apply_style: bool,
    pub save_style: bool,
}

#[derive(Clone, Default)]
pub(super) struct FlameExtra {
    pub flame_ui: FlameUIState,
    pub applied_style: Option<(String, u32)>,
    pub trail_enabled: bool,
    pub trail_fade: f32,
    pub jitter: f32,
    pub jitter_freq: f32,
    pub viewport_size: [f32; 2],
    pub gpu_timings: Vec<(String, f32)>,
    pub requests: FlameRequests,
}

pub(super) type FlameSectionView = EffectSectionView<FlameEffect, FlameRenderSettings, FlameExtra>;

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

fn validate_fit_path(flame_ui: &mut FlameUIState) {
    if flame_ui.texture_fit_path != flame_ui.texture_fit_path_validated {
        flame_ui.texture_fit_path_info = validate_texture_fit_path(&flame_ui.texture_fit_path);
        flame_ui.texture_fit_path_validated = flame_ui.texture_fit_path.clone();
    }
}

pub(super) fn read_flame_extra(world: &World, selected: Option<Entity>) -> FlameExtra {
    let mut flame_ui = world.resource::<FlameUIState>().clone();
    scan_flame_assets(&mut flame_ui);
    validate_fit_path(&mut flame_ui);

    let applied_style = selected.and_then(|entity| {
        world
            .get_component::<AppliedFlameStyle>(entity)
            .map(|style| (style.name.clone(), style.version))
    });
    let (trail_enabled, trail_fade) = selected
        .and_then(|entity| world.get_component::<FlameTrail>(entity))
        .map(|trail| (trail.state.enabled, trail.state.fade_seconds))
        .unwrap_or((false, TRAIL_FADE_SECONDS_WITHOUT_TRAIL));
    let viewport_size = world
        .get_resource::<ViewportInput>()
        .map(|viewport| viewport.size)
        .unwrap_or_default();
    let gpu_timings = world
        .get_resource::<GpuPassTimings>()
        .map(|timings| timings.passes.clone())
        .unwrap_or_default();

    FlameExtra {
        flame_ui,
        applied_style,
        trail_enabled,
        trail_fade,
        jitter: thyllore_effect_core::flame::read_env_wave_jitter(),
        jitter_freq: thyllore_effect_core::flame::read_env_wave_jitter_freq(),
        viewport_size,
        gpu_timings,
        requests: FlameRequests::default(),
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

fn requested_style_path(extra: &FlameExtra) -> Option<String> {
    if !extra.requests.apply_style {
        return None;
    }
    extra
        .flame_ui
        .style_scan
        .get(extra.flame_ui.style_index)
        .map(|name| format!("{}/{}", FLAMES_STYLE_DIR, name))
}

fn requested_save_name(extra: &FlameExtra) -> Option<String> {
    let name = extra.flame_ui.style_save_name.trim();
    (extra.requests.save_style && !name.is_empty()).then(|| name.to_string())
}

fn is_fit_or_style_applied(view: &FlameSectionView) -> bool {
    view.selected.is_some()
        && (view.extra.requests.apply_fit || requested_style_path(&view.extra).is_some())
}

pub(super) fn edited_flame_effect(
    shown: &FlameSectionView,
    view: &FlameSectionView,
) -> Option<FlameEffect> {
    if is_fit_or_style_applied(view) || view.effect == shown.effect {
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

fn send_flame_requests(world: &World, extra: &FlameExtra) {
    let flame_ui = &extra.flame_ui;
    if extra.requests.apply_fit {
        world.send_command(FlameUiCommand::ApplyTextureFit {
            path: flame_ui.texture_fit_path.clone(),
            blend: flame_ui.texture_fit_blend,
            groups: flame_ui.texture_fit_groups,
            profile: flame_ui.texture_fit_profile,
        });
    }

    if let Some(path) = requested_style_path(extra) {
        world.send_command(FlameUiCommand::ApplyStyle {
            path,
            groups: flame_ui.style_groups,
        });
    }

    if let Some(name) = requested_save_name(extra) {
        world.send_command(FlameUiCommand::SaveStyle { name });
    }
}

fn send_trail_and_jitter_changes(world: &World, shown: &FlameExtra, extra: &FlameExtra) {
    if extra.trail_enabled != shown.trail_enabled {
        world.send_command(FlameUiCommand::UpdateTrailEnabled(extra.trail_enabled));
    }
    if (extra.trail_fade - shown.trail_fade).abs() > 0.01 {
        world.send_command(FlameUiCommand::UpdateTrailFade(extra.trail_fade));
    }

    if extra.jitter != shown.jitter {
        thyllore_effect_core::flame::set_wave_jitter(extra.jitter);
    }
    if extra.jitter_freq != shown.jitter_freq {
        thyllore_effect_core::flame::set_wave_jitter_freq(extra.jitter_freq);
    }
}

pub(super) fn commit_flame_extra(world: &World, shown: &FlameSectionView, view: &FlameSectionView) {
    if view.extra.flame_ui != shown.extra.flame_ui {
        if let Some(mut resource) = world.get_resource_mut::<FlameUIState>() {
            *resource = view.extra.flame_ui.clone();
        }
    }

    if view.selected.is_none() {
        return;
    }
    send_flame_requests(world, &view.extra);
    send_trail_and_jitter_changes(world, &shown.extra, &view.extra);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;

    fn selected_view() -> FlameSectionView {
        FlameSectionView {
            instances: Vec::new(),
            instance_index: 0,
            preset: None,
            selected: Some(0),
            effect: FlameEffect::default(),
            render: FlameRenderSettings::default(),
            extra: FlameExtra::default(),
        }
    }

    fn committed_extra_command_names(
        shown: &FlameSectionView,
        view: &FlameSectionView,
    ) -> Vec<String> {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        commit_flame_extra(&world, shown, view);

        let names = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        names
    }

    #[test]
    fn texture_fit_request_is_an_apply_only_with_a_selection() {
        let mut view = selected_view();
        assert!(!is_fit_or_style_applied(&view));

        view.extra.requests.apply_fit = true;
        assert!(is_fit_or_style_applied(&view));

        view.selected = None;
        assert!(!is_fit_or_style_applied(&view));
    }

    #[test]
    fn style_request_needs_a_scanned_style() {
        let mut view = selected_view();
        view.extra.requests.apply_style = true;
        assert_eq!(requested_style_path(&view.extra), None);
        assert!(!is_fit_or_style_applied(&view));

        view.extra.flame_ui.style_scan = vec!["a.style.ron".to_string()];
        assert_eq!(
            requested_style_path(&view.extra),
            Some(format!("{}/a.style.ron", FLAMES_STYLE_DIR))
        );
        assert!(is_fit_or_style_applied(&view));
    }

    #[test]
    fn save_style_is_trimmed_and_not_an_apply() {
        let mut view = selected_view();
        view.extra.requests.save_style = true;
        view.extra.flame_ui.style_save_name = "   ".to_string();
        assert_eq!(requested_save_name(&view.extra), None);

        view.extra.flame_ui.style_save_name = "  ember ".to_string();
        assert_eq!(requested_save_name(&view.extra).as_deref(), Some("ember"));
        assert!(!is_fit_or_style_applied(&view));
    }

    #[test]
    fn read_extra_starts_without_requests() {
        let mut world = World::new();
        world.insert_resource(FlameUIState::default());

        let extra = read_flame_extra(&world, Some(7));
        assert_eq!(extra.requests, FlameRequests::default());
        assert_eq!(extra.applied_style, None);
        assert!(!extra.trail_enabled);
    }

    #[test]
    fn edited_effect_drops_blackbody_only_when_color_changes() {
        let shown = selected_view();
        assert!(edited_flame_effect(&shown, &shown.clone()).is_none());

        let mut view = shown.clone();
        view.effect.color.use_blackbody = true;
        view.effect.wave_segments += 1;
        let effect = edited_flame_effect(&shown, &view).expect("changed effect");
        assert!(effect.color.use_blackbody);

        let mut view = shown.clone();
        view.effect.color.use_blackbody = true;
        view.effect.color.tip[0] += 0.1;
        let effect = edited_flame_effect(&shown, &view).expect("changed effect");
        assert!(!effect.color.use_blackbody);
    }

    #[test]
    fn edited_effect_yields_to_a_texture_fit_applied_this_frame() {
        let shown = selected_view();
        let mut view = shown.clone();
        view.effect.wave_segments += 1;
        view.extra.requests.apply_fit = true;

        assert!(edited_flame_effect(&shown, &view).is_none());
    }

    #[test]
    fn commit_extra_sends_requests_only_with_a_selection() {
        let shown = selected_view();
        let mut view = shown.clone();
        view.extra.requests.apply_fit = true;
        view.extra.trail_enabled = true;

        let names = committed_extra_command_names(&shown, &view);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names[0].starts_with("ApplyTextureFit"), "{names:?}");
        assert!(names[1].starts_with("UpdateTrailEnabled"), "{names:?}");

        view.selected = None;
        assert!(committed_extra_command_names(&shown, &view).is_empty());
    }
}
