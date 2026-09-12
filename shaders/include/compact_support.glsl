#ifndef COMPACT_SUPPORT_GLSL
#define COMPACT_SUPPORT_GLSL

// Biweight kernel (1 - u^2)^2 on |u| < 1 and its closed-form integrals along a ray piece.
// Mirrored in thyllore-math-core/src/compact_support.rs.

#include "include/polynomial.glsl"

float biweightProfile(float uSquared) {
    float inside = max(1.0 - uSquared, 0.0);
    return inside * inside;
}

float biweight(float u) {
    return biweightProfile(u * u);
}

// (1 - u(sigma)^2)^2 for a polynomial u, without the support clamp: valid on pieces inside the support.
void biweightPoly(float u[POLY_TERMS], out float result[POLY_TERMS]) {
    float inside[POLY_TERMS];
    polyMul(u, u, inside);
    for (int k = 0; k < POLY_TERMS; ++k) {
        inside[k] = -inside[k];
    }
    inside[0] += 1.0;
    polyMul(inside, inside, result);
}

// Integral of (1 - u^2)^2 over sigma in [0, 1] for u = u0 + u1 sigma + u2 sigma^2, without the support clamp.
float biweightQuadraticIntegral(float u0, float u1, float u2) {
    float uSquared[5] = float[5](
        u0 * u0,
        2.0 * u0 * u1,
        u1 * u1 + 2.0 * u0 * u2,
        2.0 * u1 * u2,
        u2 * u2);
    float secondMoment = 0.0;
    float fourthMoment = 0.0;
    for (int i = 0; i < 5; ++i) {
        secondMoment += uSquared[i] / float(i + 1);
        for (int j = 0; j < 5; ++j) {
            fourthMoment += uSquared[i] * uSquared[j] / float(i + j + 1);
        }
    }
    return 1.0 - 2.0 * secondMoment + fourthMoment;
}

// Integral over sigma in [0, 1] of the biweight sphere (1 - |p - center|^2 / radius^2)^2 along
// p = start + d * pieceLength * sigma, which must lie inside the sphere.
float biweightSpherePieceIntegral(vec3 center, float radius, vec3 start, vec3 d, float pieceLength) {
    vec3 offset = start - center;
    float invRadiusSq = 1.0 / (radius * radius);
    float u0 = dot(offset, offset) * invRadiusSq;
    float u1 = 2.0 * pieceLength * dot(offset, d) * invRadiusSq;
    float u2 = pieceLength * pieceLength * dot(d, d) * invRadiusSq;
    return biweightQuadraticIntegral(u0, u1, u2);
}

#endif
