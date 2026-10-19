use crate::ecs::systems::flame::FlameUiCommand;
use crate::ecs::World;

pub(super) fn draw_flame_render_settings(ui: &imgui::Ui, ecs_world: &World) {
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

    ecs_world.send_command(FlameUiCommand::UpdateRenderSettings(settings_copy));
}
