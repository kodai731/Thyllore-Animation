#ifndef WIND_SHELL_INTEGRAL_GLSL
#define WIND_SHELL_INTEGRAL_GLSL

// Closed-form optical depth of the shell field along a ray: the ray is cut at the
// shell support boundaries and the envelope break (knots), and inside each piece
// the density is one polynomial in the piece-local variable sigma in [0, 1] whose
// integral is a sum of power-rule moments. Puffs add their own entry/exit knots and
// contribute a quartic that is integrated only on the pieces inside the puff.
// Mirrored in thyllore-effect-core/src/wind/analytic/shell_integral.rs.
// Must be included after shell_field.glsl.

#include "include/common.glsl"
#include "include/compact_support.glsl"
#include "include/polynomial.glsl"
#include "include/shadow_rays.glsl"

const int WIND_MAX_KNOTS = 56;
// One cell grid spans the whole ray so a knot splitting a piece never moves a sample; the cell
// length comes from the active length (shell or puff pieces) so the budget is spent on density.
const int WIND_MODULATION_CELLS = 64;
const int WIND_ACTIVE_CELLS_MIN = 16;
const float WIND_MODULATION_SAMPLE_FRACTION = 0.125;
const int WIND_PUFFS_PER_RAY = 20;
const float WIND_EMPTY_INTERVAL_EPSILON = 1e-6;
const float WIND_SHADOW_RAY_T_MAX = 1e4;

struct WindRayPuffs {
    int count;
    int index[WIND_PUFFS_PER_RAY];
    float enter[WIND_PUFFS_PER_RAY];
    float exit[WIND_PUFFS_PER_RAY];
};

void windPushKnot(inout float knots[WIND_MAX_KNOTS], inout int count, float t, float lo, float hi) {
    if (t <= lo || t >= hi || count >= WIND_MAX_KNOTS) {
        return;
    }
    knots[count] = t;
    count += 1;
}

void windPushQuadraticRoots(
    float a, float b, float c, float lo, float hi,
    inout float knots[WIND_MAX_KNOTS], inout int count) {
    if (abs(a) < WIND_LINEAR_COEFFICIENT_EPSILON) {
        if (abs(b) >= WIND_LINEAR_COEFFICIENT_EPSILON) {
            windPushKnot(knots, count, -c / b, lo, hi);
        }
        return;
    }
    float discriminant = b * b - 4.0 * a * c;
    if (discriminant < 0.0) {
        return;
    }
    float sqrtDiscriminant = sqrt(discriminant);
    windPushKnot(knots, count, (-b - sqrtDiscriminant) / (2.0 * a), lo, hi);
    windPushKnot(knots, count, (-b + sqrtDiscriminant) / (2.0 * a), lo, hi);
}

void windSortKnots(inout float knots[WIND_MAX_KNOTS], int count) {
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

int windCollectShellKnots(vec3 o, vec3 d, float tNear, float tFar, out float knots[WIND_MAX_KNOTS]) {
    int count = 2;
    knots[0] = tNear;
    knots[1] = tFar;

    float qA = dot(d.xz, d.xz);
    float qB = 2.0 * dot(o.xz, d.xz);
    float qC = dot(o.xz, o.xz);

    float invHeight = 1.0 / windHeight();
    float radius0 = windWallRadius(o.y * invHeight);
    float radius1 = windWallRadiusSlope() * d.y * invHeight;
    float deltaA = qA - radius1 * radius1;
    float deltaB = qB - 2.0 * radius0 * radius1;
    float deltaC = qC - radius0 * radius0 - windSpreadOffset();
    windPushQuadraticRoots(deltaA, deltaB, deltaC - windWallWidthQ(), tNear, tFar, knots, count);
    windPushQuadraticRoots(deltaA, deltaB, deltaC + windWallWidthQ(), tNear, tFar, knots, count);

    if (abs(d.y) >= WIND_LINEAR_COEFFICIENT_EPSILON) {
        float fadeY = windFadeStart() * windHTop() * windHeight();
        windPushKnot(knots, count, (fadeY - o.y) / d.y, tNear, tFar);
    }
    return count;
}

int windShellKnots(vec3 o, vec3 d, float tNear, float tFar, out float knots[WIND_MAX_KNOTS]) {
    int count = windCollectShellKnots(o, d, tNear, tFar, knots);
    windSortKnots(knots, count);
    return count;
}

int windRayKnots(
    vec3 o, vec3 d, float tNear, float tFar,
    out float knots[WIND_MAX_KNOTS], out WindRayPuffs puffs) {
    int count = windCollectShellKnots(o, d, tNear, tFar, knots);

    puffs.count = 0;
    float a = dot(d, d);
    for (int i = 0; i < windPuffCount(); ++i) {
        if (puffs.count >= WIND_PUFFS_PER_RAY) break;
        vec3 c = wind.puffs[i].xyz;
        float r = wind.puffs[i].w;
        if (r <= 0.0) continue;
        vec3 dx = o - c;
        float b = 2.0 * dot(dx, d);
        float cVal = dot(dx, dx) - r * r;
        float discriminant = b * b - 4.0 * a * cVal;
        if (discriminant <= 0.0) continue;
        float sqrtDiscriminant = sqrt(discriminant);
        float t0 = (-b - sqrtDiscriminant) / (2.0 * a);
        float t1 = (-b + sqrtDiscriminant) / (2.0 * a);
        if (t1 <= tNear || t0 >= tFar) continue;
        windPushKnot(knots, count, t0, tNear, tFar);
        windPushKnot(knots, count, t1, tNear, tFar);
        puffs.index[puffs.count] = i;
        puffs.enter[puffs.count] = t0;
        puffs.exit[puffs.count] = t1;
        puffs.count += 1;
    }

    windSortKnots(knots, count);
    return count;
}

void windEnvelopePoly(float h0, float h1, float hMid, out float envelope[POLY_TERMS]) {
    float fadeStart = windFadeStart();
    if (hMid <= fadeStart) {
        polyZero(envelope);
        envelope[0] = 1.0;
        return;
    }
    oneMinusSmootherstepPoly((h0 - fadeStart) / windTopFade(), h1 / windTopFade(), envelope);
}

float windPieceShellDistanceAtMid(vec3 start, vec3 d, float pieceLength, float hMid) {
    vec3 mid = start + 0.5 * pieceLength * d;
    float radiusMid = windWallRadius(hMid);
    return (dot(mid.xz, mid.xz) - radiusMid * radiusMid - windSpreadOffset()) / windWallWidthQ();
}

// True when the piece [s0, s1], which must not cross a knot, lies inside the shell support.
bool windPieceHoldsShell(vec3 o, vec3 d, float s0, float s1) {
    float pieceLength = s1 - s0;
    if (pieceLength <= WIND_EMPTY_INTERVAL_EPSILON) {
        return false;
    }
    vec3 start = o + d * s0;
    float hMid = (start.y + 0.5 * pieceLength * d.y) / windHeight();
    if (hMid < 0.0 || hMid > windHTop()) {
        return false;
    }
    return abs(windPieceShellDistanceAtMid(start, d, pieceLength, hMid)) < 1.0;
}

// Envelope times wall on the piece as a polynomial in sigma. False when the piece holds no shell.
bool windShellPiecePoly(vec3 o, vec3 d, float s0, float s1, out float density[POLY_TERMS]) {
    polyZero(density);
    if (!windPieceHoldsShell(o, d, s0, s1)) {
        return false;
    }
    float pieceLength = s1 - s0;
    vec3 start = o + d * s0;
    float invHeight = 1.0 / windHeight();
    float h0 = start.y * invHeight;
    float h1 = pieceLength * d.y * invHeight;
    float hMid = h0 + 0.5 * h1;

    float q0 = dot(start.xz, start.xz);
    float q1 = 2.0 * pieceLength * dot(start.xz, d.xz);
    float q2 = pieceLength * pieceLength * dot(d.xz, d.xz);

    float radius0 = windWallRadius(h0);
    float radius1 = windWallRadiusSlope() * h1;
    float invWidth = 1.0 / windWallWidthQ();
    float u[POLY_TERMS];
    polyFromQuadratic(
        (q0 - radius0 * radius0 - windSpreadOffset()) * invWidth,
        (q1 - 2.0 * radius0 * radius1) * invWidth,
        (q2 - radius1 * radius1) * invWidth,
        u);

    float wall[POLY_TERMS];
    biweightPoly(u, wall);
    float envelope[POLY_TERMS];
    float invHTop = 1.0 / windHTop();
    windEnvelopePoly(h0 * invHTop, h1 * invHTop, hMid * invHTop, envelope);
    polyMul(envelope, wall, density);
    polyScale(density, windWallStrength());
    return true;
}

// Wall and envelope only: the streak and eddy modulation is dropped for shadow rays.
float windShadowPieceOpticalDepth(vec3 o, vec3 d, float s0, float s1) {
    float density[POLY_TERMS];
    if (!windShellPiecePoly(o, d, s0, s1, density)) {
        return 0.0;
    }
    return max((s1 - s0) * windSigmaT() * polyMoments(density), 0.0);
}

float windModulationAt(vec3 p, vec3 stepAhead) {
    float modulation = 1.0;
    if (windStreakAmplitude() > 0.0) {
        modulation *= windStreakSigma(p);
    }
    if (windEddyAmplitude() > 0.0) {
        modulation *= windEddySigma(p, stepAhead);
    }
    return modulation;
}

// Shortest length along any ray over which the streak pattern completes one period.
float windStreakWavelength() {
    float angular = windStreakOrder() / max(windWallRadiusBase(), 1e-3);
    float vertical = windStreakRiseTime() - windStreakTwist();
    return TWO_PI / max(sqrt(angular * angular + vertical * vertical), 1e-3);
}

bool windPieceHoldsPuff(WindRayPuffs puffs, float s0, float s1) {
    float sMid = 0.5 * (s0 + s1);
    for (int k = 0; k < puffs.count; ++k) {
        if (sMid > puffs.enter[k] && sMid < puffs.exit[k]) {
            return true;
        }
    }
    return false;
}

bool windPieceIsActive(vec3 o, vec3 d, WindRayPuffs puffs, float s0, float s1) {
    return windPieceHoldsShell(o, d, s0, s1) || windPieceHoldsPuff(puffs, s0, s1);
}

// Length of the ray inside the shell support or a puff: continuous in the ray, so the cell
// length derived from it never jumps when a knot appears.
float windActiveLength(vec3 o, vec3 d, float knots[WIND_MAX_KNOTS], int knotCount, WindRayPuffs puffs) {
    float total = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        if (windPieceIsActive(o, d, puffs, knots[i - 1], knots[i])) {
            total += knots[i] - knots[i - 1];
        }
    }
    return total;
}

// Cell length along the ray: a fraction of the finest active modulation feature, bounded so the
// active length holds between WIND_ACTIVE_CELLS_MIN and WIND_MODULATION_CELLS cells.
float windModulationStep(vec3 d, float activeLength) {
    float span = max(activeLength, WIND_EMPTY_INTERVAL_EPSILON);
    float finestFeature = span * length(d);
    if (windStreakAmplitude() > 0.0) {
        finestFeature = min(finestFeature, 0.5 * windStreakWavelength());
    }
    if (windEddyAmplitude() > 0.0) {
        float finestOctaveScale = exp2(float(WIND_EDDY_OCTAVE_COUNT - 1));
        float finestCell = min(windEddyCellHeight(), min(windEddyCellTheta(), windEddyCellRadial()));
        finestFeature = min(finestFeature, finestCell / finestOctaveScale);
    }
    return clamp(
        WIND_MODULATION_SAMPLE_FRACTION * finestFeature / length(d),
        span / float(WIND_MODULATION_CELLS),
        span / float(WIND_ACTIVE_CELLS_MIN));
}

// [s0, s1] must not cross a knot; the modulation is linear on it with the given end values.
float windPieceOpticalDepth(vec3 o, vec3 d, float s0, float s1, float modulation0, float modulation1) {
    float density[POLY_TERMS];
    if (!windShellPiecePoly(o, d, s0, s1, density)) {
        return 0.0;
    }
    float total = polyLinearWeightedMoments(density, modulation0, modulation1);
    return max((s1 - s0) * windSigmaT() * total, 0.0);
}

// Puffs are integrated only on the pieces between their own entry and exit knots.
float windPuffPieceOpticalDepth(WindRayPuffs puffs, vec3 o, vec3 d, float s0, float s1) {
    float pieceLength = s1 - s0;
    if (pieceLength <= WIND_EMPTY_INTERVAL_EPSILON || puffs.count == 0) {
        return 0.0;
    }
    float sMid = 0.5 * (s0 + s1);
    vec3 start = o + d * s0;

    float total = 0.0;
    for (int k = 0; k < puffs.count; ++k) {
        if (sMid <= puffs.enter[k] || sMid >= puffs.exit[k]) continue;
        vec4 puff = wind.puffs[puffs.index[k]];
        total += biweightSpherePieceIntegral(puff.xyz, puff.w, start, d, pieceLength);
    }
    return max(pieceLength * windSigmaT() * windPuffStrength() * windWallStrength() * total, 0.0);
}

float windShadowOpticalDepth(vec3 o, vec3 d, float tNear, float tFar) {
    if (tFar <= tNear) {
        return 0.0;
    }
    float knots[WIND_MAX_KNOTS];
    int knotCount = windShellKnots(o, d, tNear, tFar, knots);
    float total = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        total += windShadowPieceOpticalDepth(o, d, knots[i - 1], knots[i]);
    }
    return total;
}

// Shadow ray of include/shadow_rays.glsl: wall + envelope depth up to the cone boundary.
float shadowOpticalDepthToward(vec3 origin, vec3 direction) {
    float tNear = 0.0;
    float tFar = WIND_SHADOW_RAY_T_MAX;
    if (!clampToWindCone(origin, direction, tNear, tFar)) {
        return 0.0;
    }
    tNear = max(tNear, 0.0);
    return windShadowOpticalDepth(origin, direction, tNear, tFar);
}

#endif
