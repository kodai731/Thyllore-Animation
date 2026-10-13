use std::rc::Rc;

use crate::ecs::component::{AppliedWindPreset, WindTornadoEffect, WIND_DOMAIN};
use crate::ecs::resource::{WindDebugCapture, WindRenderSettings};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::{EffectEdit, WIND_SPAWN_HOOK};
use crate::ecs::World;

use super::effect_sections::{build_effect_section, EffectPanel};
use super::param_widgets::{ScalarParam, UiParam};

pub(super) struct WindPanel;

impl EffectPanel for WindPanel {
    type Component = WindTornadoEffect;
    type RenderSettings = WindRenderSettings;

    const NAME: &'static str = "Wind";
    const SPAWN_HOOK_KEY: &'static str = WIND_SPAWN_HOOK.key;
    const PRESET_NAMES: &'static [&'static str] = thyllore_effect_core::WIND_PRESET_NAMES;

    fn ui_params() -> &'static [UiParam] {
        &*thyllore_effect_core::WIND_UI_PARAMS
    }

    fn scalar_params() -> &'static [ScalarParam<Self::Component>] {
        &*thyllore_effect_core::WIND_SCALAR_PARAMS
    }

    fn domain() -> &'static crate::ecs::component::ScalarChannelDomain {
        &WIND_DOMAIN
    }

    fn applied_preset_name(applied: &AppliedWindPreset) -> String {
        applied.name.clone()
    }

    fn draw_debug(ui: &imgui::Ui, ecs_world: &World) {
        draw_wind_render_settings(ui, ecs_world);
        if ui.button("Dump Debug") {
            ecs_world.send_command(CameraEvent::CaptureNow(Rc::new(WindDebugCapture)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(
                "Write wind parameters, UBO, render settings, camera and a screenshot to log/wind/",
            );
        }
    }
}

fn draw_wind_render_settings(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::resource::{WindDebugView, WindShadingMode};

    let Some(settings) = ecs_world.get_resource::<WindRenderSettings>() else {
        return;
    };
    let mut settings_copy = *settings;
    drop(settings);

    if let Some(_token) = ui.begin_combo("Shading Mode", settings_copy.shading_mode.label()) {
        for mode in WindShadingMode::ALL {
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
        for view in WindDebugView::ALL {
            if ui
                .selectable_config(view.label())
                .selected(view == settings_copy.debug_view)
                .build()
            {
                settings_copy.debug_view = view;
            }
        }
    }
    let mut step_count = settings_copy.reference_step_count as i32;
    if ui
        .slider_config("Reference Steps", 16, 2048)
        .build(&mut step_count)
    {
        settings_copy.reference_step_count = step_count.max(1) as u32;
    }
    ui.checkbox(
        "Animate when paused",
        &mut settings_copy.free_run_when_paused,
    );
    ecs_world.send_command(
        EffectEdit::<WindTornadoEffect, WindRenderSettings>::UpdateRenderSettings(settings_copy),
    );
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "wind",
    order: 2,
    draw: build_effect_section::<WindPanel>,
});
