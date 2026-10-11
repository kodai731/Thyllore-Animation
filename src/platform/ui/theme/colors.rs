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

pub fn srgb_to_linear(color: [f32; 4]) -> [f32; 4] {
    let [r, g, b, a] = color;
    [
        if r <= 0.04045 {
            r / 12.92
        } else {
            ((r + 0.055) / 1.055).powf(2.4)
        },
        if g <= 0.04045 {
            g / 12.92
        } else {
            ((g + 0.055) / 1.055).powf(2.4)
        },
        if b <= 0.04045 {
            b / 12.92
        } else {
            ((b + 0.055) / 1.055).powf(2.4)
        },
        a,
    ]
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

fn relative_luminance(color: [f32; 4]) -> f32 {
    let [r, g, b, _a] = color;
    let r = if r <= 0.03928 {
        r / 12.92
    } else {
        ((r + 0.055) / 1.055).powf(2.4)
    };
    let g = if g <= 0.03928 {
        g / 12.92
    } else {
        ((g + 0.055) / 1.055).powf(2.4)
    };
    let b = if b <= 0.03928 {
        b / 12.92
    } else {
        ((b + 0.055) / 1.055).powf(2.4)
    };
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn contrast_ratio(fg: [f32; 4], bg: [f32; 4]) -> f32 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            contrast_ratio(TEXT_SECONDARY, SURFACE1) >= 4.5,
            "TEXT_SECONDARY on SURFACE1: {:.2}",
            contrast_ratio(TEXT_SECONDARY, SURFACE1)
        );
        assert!(
            contrast_ratio(TEXT, SURFACE0) >= 4.5,
            "TEXT on SURFACE0: {:.2}",
            contrast_ratio(TEXT, SURFACE0)
        );
    }

    #[test]
    fn test_srgb_to_linear_surface0() {
        let linear = srgb_to_linear(SURFACE0);
        assert!(
            (linear[0] - 0.0116).abs() < 0.001,
            "SURFACE0 R linear: {:.4}",
            linear[0]
        );
    }

    #[test]
    fn test_srgb_to_linear_white_unchanged() {
        let white = [1.0, 1.0, 1.0, 1.0];
        let linear = srgb_to_linear(white);
        assert!((linear[0] - 1.0).abs() < 1e-6);
        assert!((linear[1] - 1.0).abs() < 1e-6);
        assert!((linear[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_srgb_to_linear_black_unchanged() {
        let black = [0.0, 0.0, 0.0, 1.0];
        let linear = srgb_to_linear(black);
        assert!((linear[0]).abs() < 1e-6);
        assert!((linear[1]).abs() < 1e-6);
        assert!((linear[2]).abs() < 1e-6);
    }

    #[test]
    fn test_srgb_to_linear_alpha_unchanged() {
        let color = [0.5, 0.3, 0.7, 0.42];
        let linear = srgb_to_linear(color);
        assert!((linear[3] - 0.42).abs() < 1e-6);
    }
}
