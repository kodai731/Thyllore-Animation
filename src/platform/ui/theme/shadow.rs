use crate::platform::ui::theme::colors::srgb_to_linear;

const SHADOW_SPREAD: f32 = 12.0;
const SHADOW_LAYER_COUNT: usize = 8;
const SHADOW_BASE_ALPHA: f32 = 0.35;
const SHADOW_Y_OFFSET: f32 = 3.0;

#[derive(Clone, Copy, Debug)]
pub struct ShadowLayer {
    pub offset: f32,
    pub alpha: f32,
}

pub fn shadow_layers(spread: f32, layer_count: usize) -> Vec<ShadowLayer> {
    if spread <= 0.0 || layer_count == 0 {
        return Vec::new();
    }

    let mut layers = Vec::with_capacity(layer_count);
    for i in 0..layer_count {
        let offset = (i as f32 / (layer_count - 1) as f32) * spread;
        let sigma = spread * 0.5;
        let gaussian = (-((offset / sigma).powi(2))).exp();
        let alpha = SHADOW_BASE_ALPHA * gaussian / layer_count as f32;
        layers.push(ShadowLayer { offset, alpha });
    }
    layers
}

pub fn draw_window_shadow(ui: &imgui::Ui, rounding: f32) {
    let draw_list = ui.get_window_draw_list();
    let window_pos = ui.window_pos();
    let window_size = ui.window_size();

    let shadow_min = [window_pos[0] - SHADOW_SPREAD, window_pos[1] - SHADOW_SPREAD];
    let shadow_max = [
        window_pos[0] + window_size[0] + SHADOW_SPREAD,
        window_pos[1] + window_size[1] + SHADOW_SPREAD,
    ];

    draw_list.with_clip_rect(shadow_min, shadow_max, || {
        let layers = shadow_layers(SHADOW_SPREAD, SHADOW_LAYER_COUNT);
        for layer in layers {
            let layer_min = [
                window_pos[0] - layer.offset,
                window_pos[1] - layer.offset + SHADOW_Y_OFFSET,
            ];
            let layer_max = [
                window_pos[0] + window_size[0] + layer.offset,
                window_pos[1] + window_size[1] + layer.offset + SHADOW_Y_OFFSET,
            ];
            let shadow_color = srgb_to_linear([0.0, 0.0, 0.0, layer.alpha]);
            draw_list
                .add_rect(layer_min, layer_max, shadow_color)
                .rounding(rounding + layer.offset)
                .thickness(1.5)
                .build();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_count_matches() {
        let layers = shadow_layers(12.0, 8);
        assert_eq!(layers.len(), 8);
    }

    #[test]
    fn test_offsets_monotonically_increasing() {
        let layers = shadow_layers(12.0, 8);
        for i in 1..layers.len() {
            assert!(
                layers[i].offset > layers[i - 1].offset,
                "offset at index {} ({}) should be greater than at index {} ({})",
                i,
                layers[i].offset,
                i - 1,
                layers[i - 1].offset
            );
        }
    }

    #[test]
    fn test_alphas_monotonically_decreasing() {
        let layers = shadow_layers(12.0, 8);
        for i in 1..layers.len() {
            assert!(
                layers[i].alpha < layers[i - 1].alpha,
                "alpha at index {} ({}) should be less than at index {} ({})",
                i,
                layers[i].alpha,
                i - 1,
                layers[i - 1].alpha
            );
        }
    }

    #[test]
    fn test_spread_zero_returns_empty() {
        let layers = shadow_layers(0.0, 8);
        assert!(layers.is_empty());
    }

    #[test]
    fn test_layer_count_zero_returns_empty() {
        let layers = shadow_layers(12.0, 0);
        assert!(layers.is_empty());
    }
}
