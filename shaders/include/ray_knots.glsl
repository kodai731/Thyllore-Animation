#ifndef RAY_KNOTS_GLSL
#define RAY_KNOTS_GLSL

// Sorted ray parameters at which a piecewise-polynomial medium changes form; every piece
// between two consecutive knots is integrated in closed form.
// Mirrored in thyllore-effect-core/src/volume/knots.rs.

const int RAY_MAX_KNOTS = 56;
const float RAY_LINEAR_COEFFICIENT_EPSILON = 1e-7;

void rayPushKnot(inout float knots[RAY_MAX_KNOTS], inout int count, float t, float lo, float hi) {
    if (t <= lo || t >= hi || count >= RAY_MAX_KNOTS) {
        return;
    }
    knots[count] = t;
    count += 1;
}

void rayPushQuadraticRoots(
    float a, float b, float c, float lo, float hi,
    inout float knots[RAY_MAX_KNOTS], inout int count) {
    if (abs(a) < RAY_LINEAR_COEFFICIENT_EPSILON) {
        if (abs(b) >= RAY_LINEAR_COEFFICIENT_EPSILON) {
            rayPushKnot(knots, count, -c / b, lo, hi);
        }
        return;
    }
    float discriminant = b * b - 4.0 * a * c;
    if (discriminant < 0.0) {
        return;
    }
    float sqrtDiscriminant = sqrt(discriminant);
    rayPushKnot(knots, count, (-b - sqrtDiscriminant) / (2.0 * a), lo, hi);
    rayPushKnot(knots, count, (-b + sqrtDiscriminant) / (2.0 * a), lo, hi);
}

void raySortKnots(inout float knots[RAY_MAX_KNOTS], int count) {
    for (int i = 1; i < count; ++i) {
        float value = knots[i];
        int j = i;
        while (j > 0 && knots[j - 1] > value) {
            knots[j] = knots[j - 1];
            j -= 1;
        }
        knots[j] = value;
    }
}

int rayBeginKnots(float tNear, float tFar, out float knots[RAY_MAX_KNOTS]) {
    knots[0] = tNear;
    knots[1] = tFar;
    return 2;
}

#endif
