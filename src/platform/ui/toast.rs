use imgui::{Condition, StyleVar};

use crate::asset::AssetStorage;
use crate::ecs::resource::{advance_toast, ToastMessage, UiToast, ViewportInput};
use crate::ecs::world::World;
use crate::platform::ui::theme::colors::{srgb_to_linear, SURFACE2, TEXT};
use crate::platform::ui::theme::shadow::draw_window_shadow;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

const TOAST_FADE_SECS: f32 = 0.3;
const TOAST_PADDING: [f32; 2] = [16.0, 8.0];
const TOAST_BOTTOM_MARGIN: f32 = 24.0;

fn compute_fade_alpha(remaining_secs: f32) -> f32 {
    (remaining_secs / TOAST_FADE_SECS).clamp(0.0, 1.0)
}

fn with_alpha(color: [f32; 4], alpha: f32) -> [f32; 4] {
    srgb_to_linear([color[0], color[1], color[2], color[3] * alpha])
}

fn draw_toast_pill(ui: &imgui::Ui, viewport: &ViewportInput, message: &ToastMessage) {
    let text_size = ui.calc_text_size(&message.text);
    let pill_size = [
        text_size[0] + TOAST_PADDING[0] * 2.0,
        text_size[1] + TOAST_PADDING[1] * 2.0,
    ];
    let pill_rounding = pill_size[1] * 0.5;
    let anchor = [
        viewport.position[0] + viewport.size[0] * 0.5,
        viewport.position[1] + viewport.size[1] - TOAST_BOTTOM_MARGIN,
    ];
    let alpha = compute_fade_alpha(message.remaining_secs);

    let _border = ui.push_style_var(StyleVar::WindowBorderSize(0.0));
    ui.window("##undo_toast")
        .position(anchor, Condition::Always)
        .position_pivot([0.5, 1.0])
        .size(pill_size, Condition::Always)
        .no_decoration()
        .no_inputs()
        .bg_alpha(0.0)
        .focus_on_appearing(false)
        .save_settings(false)
        .build(|| {
            draw_window_shadow(ui, pill_rounding);

            let draw_list = ui.get_window_draw_list();
            let pill_min = ui.window_pos();
            let pill_max = [pill_min[0] + pill_size[0], pill_min[1] + pill_size[1]];
            draw_list
                .add_rect(pill_min, pill_max, with_alpha(SURFACE2, alpha))
                .rounding(pill_rounding)
                .filled(true)
                .build();
            draw_list.add_text(
                [
                    pill_min[0] + TOAST_PADDING[0],
                    pill_min[1] + TOAST_PADDING[1],
                ],
                with_alpha(TEXT, alpha),
                &message.text,
            );
        });
}

fn build_toast_window(ui: &imgui::Ui, world: &World, _: &AssetStorage, _: &GraphicsResources) {
    let message = {
        let mut toast = world.resource_mut::<UiToast>();
        toast.message = advance_toast(toast.message.take(), ui.io().delta_time);
        toast.message.clone()
    };
    let Some(message) = message else {
        return;
    };

    draw_toast_pill(ui, &world.resource::<ViewportInput>(), &message);
}

crate::ui_window!("toast", Floating, 4, build_toast_window);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_alpha_is_opaque_before_fade_window() {
        assert_eq!(compute_fade_alpha(1.0), 1.0);
    }

    #[test]
    fn test_fade_alpha_is_proportional_inside_fade_window() {
        assert!((compute_fade_alpha(TOAST_FADE_SECS * 0.5) - 0.5).abs() < 1e-6);
    }
}
