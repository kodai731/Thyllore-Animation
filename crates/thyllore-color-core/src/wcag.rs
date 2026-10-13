use crate::srgb::{luminance, srgb_to_linear};

/// WCAG 2.x adds this viewing-flare term to both luminances so pure black keeps a finite ratio.
pub const WCAG_FLARE_LUMINANCE: f32 = 0.05;
/// Minimum contrast WCAG 2.x level AA requires for normal-size text.
pub const WCAG_AA_TEXT_CONTRAST: f32 = 4.5;

pub fn relative_luminance(srgb: [f32; 3]) -> f32 {
    luminance(srgb.map(srgb_to_linear))
}

pub fn contrast_ratio(srgb_a: [f32; 3], srgb_b: [f32; 3]) -> f32 {
    let luminance_a = relative_luminance(srgb_a);
    let luminance_b = relative_luminance(srgb_b);
    let lighter = luminance_a.max(luminance_b);
    let darker = luminance_a.min(luminance_b);
    (lighter + WCAG_FLARE_LUMINANCE) / (darker + WCAG_FLARE_LUMINANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_on_black_is_the_maximum_ratio() {
        let ratio = contrast_ratio([1.0, 1.0, 1.0], [0.0, 0.0, 0.0]);
        assert!((ratio - 21.0).abs() < 1e-3, "{ratio}");
    }

    #[test]
    fn contrast_ratio_is_symmetric() {
        let a = [0.2, 0.5, 0.7];
        let b = [0.9, 0.3, 0.1];
        assert!((contrast_ratio(a, b) - contrast_ratio(b, a)).abs() < 1e-6);
    }
}
