//! Fixed-degree polynomials in a piece-local variable sigma in [0, 1] and their
//! power-rule integrals. Mirrored by `shaders/include/polynomial.glsl`.

pub const POLY_TERMS: usize = 16;

pub type Poly = [f32; POLY_TERMS];

pub fn poly_zero() -> Poly {
    [0.0; POLY_TERMS]
}

pub fn poly_from_quadratic(c0: f32, c1: f32, c2: f32) -> Poly {
    let mut poly = poly_zero();
    poly[0] = c0;
    poly[1] = c1;
    poly[2] = c2;
    poly
}

/// Product truncated to `POLY_TERMS` coefficients.
pub fn poly_mul(a: &Poly, b: &Poly) -> Poly {
    let mut product = poly_zero();
    for (i, &ai) in a.iter().enumerate() {
        if ai == 0.0 {
            continue;
        }
        for (j, &bj) in b.iter().enumerate() {
            if i + j >= POLY_TERMS {
                break;
            }
            product[i + j] += ai * bj;
        }
    }
    product
}

pub fn poly_scale(poly: &mut Poly, factor: f32) {
    for coefficient in poly.iter_mut() {
        *coefficient *= factor;
    }
}

/// Integral of the polynomial over sigma in [0, 1].
pub fn poly_moments(poly: &Poly) -> f32 {
    poly.iter()
        .enumerate()
        .map(|(n, coefficient)| coefficient / (n as f32 + 1.0))
        .sum()
}

/// Integral over sigma in [0, 1] of the polynomial times the linear weight
/// `w0 + (w1 - w0) sigma`.
pub fn poly_linear_weighted_moments(poly: &Poly, w0: f32, w1: f32) -> f32 {
    poly.iter()
        .enumerate()
        .map(|(n, coefficient)| coefficient * (w0 / (n + 1) as f32 + (w1 - w0) / (n + 2) as f32))
        .sum()
}

/// Coefficients of `1 - S(v0 + v1 sigma)` for the quintic smootherstep
/// `S(v) = 10 v^3 - 15 v^4 + 6 v^5`, expanded in sigma.
pub fn one_minus_smootherstep_poly(v0: f32, v1: f32) -> Poly {
    let mut poly = poly_zero();
    poly[0] = 1.0 - 10.0 * v0 * v0 * v0 + 15.0 * v0 * v0 * v0 * v0 - 6.0 * v0 * v0 * v0 * v0 * v0;
    poly[1] = v1 * (-30.0 * v0 * v0 + 60.0 * v0 * v0 * v0 - 30.0 * v0 * v0 * v0 * v0);
    poly[2] = v1 * v1 * (-30.0 * v0 + 90.0 * v0 * v0 - 60.0 * v0 * v0 * v0);
    poly[3] = v1 * v1 * v1 * (-10.0 + 60.0 * v0 - 60.0 * v0 * v0);
    poly[4] = v1 * v1 * v1 * v1 * (15.0 - 30.0 * v0);
    poly[5] = -6.0 * v1 * v1 * v1 * v1 * v1;
    poly
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluate(poly: &Poly, sigma: f32) -> f32 {
        poly.iter()
            .rev()
            .fold(0.0, |acc, coefficient| acc * sigma + coefficient)
    }

    fn smootherstep(v: f32) -> f32 {
        v * v * v * (10.0 - v * (15.0 - 6.0 * v))
    }

    #[test]
    fn product_matches_pointwise_evaluation() {
        let a = poly_from_quadratic(0.5, -1.25, 2.0);
        let b = poly_from_quadratic(-0.75, 0.5, 1.5);
        let product = poly_mul(&a, &b);
        for i in 0..=10 {
            let sigma = i as f32 / 10.0;
            let expected = evaluate(&a, sigma) * evaluate(&b, sigma);
            assert!((evaluate(&product, sigma) - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn moments_match_midpoint_quadrature() {
        let poly = poly_mul(
            &poly_from_quadratic(1.0, -2.0, 0.5),
            &poly_from_quadratic(0.25, 1.0, -0.5),
        );
        let steps = 20000;
        let ds = 1.0 / steps as f64;
        let mut plain = 0.0f64;
        let mut weighted = 0.0f64;
        for i in 0..steps {
            let sigma = (i as f64 + 0.5) * ds;
            let value = evaluate(&poly, sigma as f32) as f64;
            plain += value * ds;
            weighted += value * (0.3 + (1.7 - 0.3) * sigma) * ds;
        }
        assert!((poly_moments(&poly) as f64 - plain).abs() < 1e-5);
        assert!((poly_linear_weighted_moments(&poly, 0.3, 1.7) as f64 - weighted).abs() < 1e-5);
    }

    #[test]
    fn smootherstep_expansion_matches_direct_evaluation() {
        let (v0, v1) = (0.15, 0.6);
        let poly = one_minus_smootherstep_poly(v0, v1);
        for i in 0..=10 {
            let sigma = i as f32 / 10.0;
            let expected = 1.0 - smootherstep(v0 + v1 * sigma);
            assert!((evaluate(&poly, sigma) - expected).abs() < 1e-5);
        }
    }
}
