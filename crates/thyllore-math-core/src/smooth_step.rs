/// Smooth step function S(x) = x*x*(3-2x), clamped to [0, 1].
pub fn smooth_step(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// GLSL `smoothstep(edge0, edge1, x)`.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// GLSL `mix(a, b, t)`: evaluated as `a * (1 - t) + b * t` so the two mirrors agree bit for bit.
pub fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_clamps_and_interpolates() {
        assert_eq!(smoothstep(1.0, 2.0, 0.5), 0.0);
        assert_eq!(smoothstep(1.0, 2.0, 3.0), 1.0);
        assert!((smoothstep(1.0, 2.0, 1.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mix_hits_both_ends() {
        assert_eq!(mix(2.0, 6.0, 0.0), 2.0);
        assert_eq!(mix(2.0, 6.0, 1.0), 6.0);
        assert!((mix(2.0, 6.0, 0.25) - 3.0).abs() < 1e-6);
    }
}
