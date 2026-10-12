pub use thyllore_color_core::srgb_to_linear_rgba;

const fn rgb_hex(hex: u32) -> [f32; 4] {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    [r, g, b, 1.0]
}

const fn rgba_hex(hex: u64) -> [f32; 4] {
    let a = ((hex >> 24) & 0xFF) as f32 / 255.0;
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    [r, g, b, a]
}

pub const SURFACE0: [f32; 4] = rgb_hex(0x1C1C1E);
pub const SURFACE1: [f32; 4] = rgb_hex(0x2C2C2E);
pub const SURFACE2: [f32; 4] = rgb_hex(0x3A3A3C);
pub const SURFACE3: [f32; 4] = rgb_hex(0x48484A);
pub const ACCENT: [f32; 4] = rgb_hex(0x0A84FF);
pub const TEXT: [f32; 4] = rgb_hex(0xF2F2F7);
pub const TEXT_SECONDARY: [f32; 4] = rgb_hex(0x98989D);
pub const OUTLINE: [f32; 4] = rgba_hex(0x14FFFFFF);
pub const AXIS_X: [f32; 4] = rgb_hex(0xFF453A);
pub const AXIS_Y: [f32; 4] = rgb_hex(0x30D158);
pub const AXIS_Z: [f32; 4] = rgb_hex(0x0A84FF);

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_color_core::{contrast_ratio, WCAG_AA_TEXT_CONTRAST};

    fn text_contrast(foreground: [f32; 4], background: [f32; 4]) -> f32 {
        let [fr, fg, fb, _] = foreground;
        let [br, bg, bb, _] = background;
        contrast_ratio([fr, fg, fb], [br, bg, bb])
    }

    #[test]
    fn test_rgb_hex_accent() {
        let [r, g, b, a] = rgb_hex(0x0A84FF);
        assert!((r - 10.0 / 255.0).abs() < 1e-6);
        assert!((g - 132.0 / 255.0).abs() < 1e-6);
        assert!((b - 255.0 / 255.0).abs() < 1e-6);
        assert!((a - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_wcag_aa_contrast_ratios() {
        assert!(
            text_contrast(TEXT_SECONDARY, SURFACE1) >= WCAG_AA_TEXT_CONTRAST,
            "TEXT_SECONDARY on SURFACE1: {:.2}",
            text_contrast(TEXT_SECONDARY, SURFACE1)
        );
        assert!(
            text_contrast(TEXT, SURFACE0) >= WCAG_AA_TEXT_CONTRAST,
            "TEXT on SURFACE0: {:.2}",
            text_contrast(TEXT, SURFACE0)
        );
    }

    #[test]
    fn test_srgb_to_linear_surface0() {
        let linear = srgb_to_linear_rgba(SURFACE0);
        assert!(
            (linear[0] - 0.0116).abs() < 0.001,
            "SURFACE0 R linear: {:.4}",
            linear[0]
        );
    }

    #[test]
    fn test_srgb_to_linear_alpha_unchanged() {
        let linear = srgb_to_linear_rgba([0.5, 0.3, 0.7, 0.42]);
        assert!((linear[3] - 0.42).abs() < 1e-6);
    }
}
