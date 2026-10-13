use std::rc::Rc;

use crate::ecs::component::{AppliedWaterPreset, WaterTorusEffect, WATER_DOMAIN};
use crate::ecs::resource::{WaterDebugCapture, WaterRenderSettings};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::{EffectEdit, WATER_SPAWN_HOOK};
use crate::ecs::World;

use super::effect_sections::{build_effect_section, EffectPanel};
use super::param_widgets::{ScalarParam, UiParam};

pub(super) struct WaterPanel;

impl EffectPanel for WaterPanel {
    type Component = WaterTorusEffect;
    type RenderSettings = WaterRenderSettings;

    const NAME: &'static str = "Water";
    const SPAWN_HOOK_KEY: &'static str = WATER_SPAWN_HOOK.key;
    const PRESET_NAMES: &'static [&'static str] = thyllore_effect_core::WATER_PRESET_NAMES;

    fn ui_params() -> &'static [UiParam] {
        &*thyllore_effect_core::WATER_UI_PARAMS
    }

    fn scalar_params() -> &'static [ScalarParam<Self::Component>] {
        &*thyllore_effect_core::WATER_SCALAR_PARAMS
    }

    fn domain() -> &'static crate::ecs::component::ScalarChannelDomain {
        &WATER_DOMAIN
    }

    fn applied_preset_name(applied: &AppliedWaterPreset) -> String {
        applied.name.clone()
    }

    fn draw_debug(ui: &imgui::Ui, ecs_world: &World) {
        draw_water_render_settings(ui, ecs_world);
        if ui.button("Dump Debug") {
            ecs_world.send_command(CameraEvent::CaptureNow(Rc::new(WaterDebugCapture)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(
                "Write water parameters, UBO, camera, render settings and a screenshot to log/water/",
            );
        }
    }
}

fn draw_water_render_settings(ui: &imgui::Ui, ecs_world: &World) {
    let Some(settings) = ecs_world.get_resource::<WaterRenderSettings>() else {
        return;
    };
    let mut settings_copy = *settings;
    drop(settings);

    if let Some(_token) = ui.begin_combo("Secondary Rays", settings_copy.secondary_rays.label()) {
        for mode in thyllore_effect_core::WaterSecondaryRays::ALL {
            if ui
                .selectable_config(mode.label())
                .selected(mode == settings_copy.secondary_rays)
                .build()
            {
                settings_copy.secondary_rays = mode;
            }
        }
    }
    let mut debug_view = settings_copy.debug_view as f32;
    if ui
        .slider_config("Debug View", 0.0f32, 1.0f32)
        .build(&mut debug_view)
    {
        settings_copy.debug_view = debug_view as i32;
    }
    ui.checkbox(
        "Animate when paused",
        &mut settings_copy.free_run_when_paused,
    );
    ecs_world.send_command(
        EffectEdit::<WaterTorusEffect, WaterRenderSettings>::UpdateRenderSettings(settings_copy),
    );
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "water",
    order: 1,
    draw: build_effect_section::<WaterPanel>,
});
