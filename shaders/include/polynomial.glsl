#ifndef POLYNOMIAL_GLSL
#define POLYNOMIAL_GLSL

// Fixed-degree polynomials in a piece-local variable sigma in [0, 1] and their power-rule
// integrals. Mirrored in thyllore-math-core/src/polynomial.rs.

const int POLY_TERMS = 16;

void polyZero(out float poly[POLY_TERMS]) {
    for (int k = 0; k < POLY_TERMS; ++k) {
        poly[k] = 0.0;
    }
}

void polyFromQuadratic(float c0, float c1, float c2, out float poly[POLY_TERMS]) {
    polyZero(poly);
    poly[0] = c0;
    poly[1] = c1;
    poly[2] = c2;
}

// Product truncated to POLY_TERMS coefficients.
void polyMul(float a[POLY_TERMS], float b[POLY_TERMS], out float product[POLY_TERMS]) {
    polyZero(product);
    for (int i = 0; i < POLY_TERMS; ++i) {
        if (a[i] == 0.0) {
            continue;
        }
        for (int j = 0; i + j < POLY_TERMS; ++j) {
            product[i + j] += a[i] * b[j];
        }
    }
}

void polyScale(inout float poly[POLY_TERMS], float factor) {
    for (int k = 0; k < POLY_TERMS; ++k) {
        poly[k] *= factor;
    }
}

// Integral of the polynomial over sigma in [0, 1].
float polyMoments(float poly[POLY_TERMS]) {
    float sum = 0.0;
    for (int n = 0; n < POLY_TERMS; ++n) {
        sum += poly[n] / float(n + 1);
    }
    return sum;
}

// Integral over sigma in [0, 1] of the polynomial times the linear weight w0 + (w1 - w0) sigma.
float polyLinearWeightedMoments(float poly[POLY_TERMS], float w0, float w1) {
    float sum = 0.0;
    for (int n = 0; n < POLY_TERMS; ++n) {
        sum += poly[n] * (w0 / float(n + 1) + (w1 - w0) / float(n + 2));
    }
    return sum;
}

// Coefficients of 1 - S(v0 + v1 sigma) for the quintic smootherstep S(v) = 10v^3 - 15v^4 + 6v^5.
void oneMinusSmootherstepPoly(float v0, float v1, out float poly[POLY_TERMS]) {
    polyZero(poly);
    poly[0] = 1.0 - 10.0 * v0 * v0 * v0 + 15.0 * v0 * v0 * v0 * v0 - 6.0 * v0 * v0 * v0 * v0 * v0;
    poly[1] = v1 * (-30.0 * v0 * v0 + 60.0 * v0 * v0 * v0 - 30.0 * v0 * v0 * v0 * v0);
    poly[2] = v1 * v1 * (-30.0 * v0 + 90.0 * v0 * v0 - 60.0 * v0 * v0 * v0);
    poly[3] = v1 * v1 * v1 * (-10.0 + 60.0 * v0 - 60.0 * v0 * v0);
    poly[4] = v1 * v1 * v1 * v1 * (15.0 - 30.0 * v0);
    poly[5] = -6.0 * v1 * v1 * v1 * v1 * v1;
}

#endif
