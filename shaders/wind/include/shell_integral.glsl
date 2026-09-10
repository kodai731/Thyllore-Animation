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
#include "include/radiative_transfer.glsl"

const int WIND_MAX_KNOTS = 56;
const int WIND_POLY_TERMS = 16;
// Fixed so the node set is a continuous function of the ray (no seams where a count would change).
const int WIND_EDDY_SPLITS = 8;
const int WIND_PUFFS_PER_RAY = 20;
const float WIND_EMPTY_INTERVAL_EPSILON = 1e-6;
const float WIND_SHADOW_RAY_T_MAX = 1e4;
const int WIND_SKY_TILTED_DIRECTIONS = 6;
const float WIND_SKY_TILT_COSINE = 0.70710678;

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

void windPolyMul(float a[WIND_POLY_TERMS], float b[WIND_POLY_TERMS], out float product[WIND_POLY_TERMS]) {
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        product[k] = 0.0;
    }
    for (int i = 0; i < WIND_POLY_TERMS; ++i) {
        if (a[i] == 0.0) {
            continue;
        }
        for (int j = 0; i + j < WIND_POLY_TERMS; ++j) {
            product[i + j] += a[i] * b[j];
        }
    }
}

void windPolyFromQuadratic(float c0, float c1, float c2, out float poly[WIND_POLY_TERMS]) {
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        poly[k] = 0.0;
    }
    poly[0] = c0;
    poly[1] = c1;
    poly[2] = c2;
}

void windBiweightPoly(float u[WIND_POLY_TERMS], out float result[WIND_POLY_TERMS]) {
    float inside[WIND_POLY_TERMS];
    windPolyMul(u, u, inside);
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        inside[k] = -inside[k];
    }
    inside[0] += 1.0;
    windPolyMul(inside, inside, result);
}

void windEnvelopePoly(float h0, float h1, float hMid, out float envelope[WIND_POLY_TERMS]) {
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        envelope[k] = 0.0;
    }
    float fadeStart = windFadeStart();
    if (hMid <= fadeStart) {
        envelope[0] = 1.0;
        return;
    }
    float v0 = (h0 - fadeStart) / windTopFade();
    float v1 = h1 / windTopFade();
    envelope[0] = 1.0 - 10.0 * v0 * v0 * v0 + 15.0 * v0 * v0 * v0 * v0 - 6.0 * v0 * v0 * v0 * v0 * v0;
    envelope[1] = v1 * (-30.0 * v0 * v0 + 60.0 * v0 * v0 * v0 - 30.0 * v0 * v0 * v0 * v0);
    envelope[2] = v1 * v1 * (-30.0 * v0 + 90.0 * v0 * v0 - 60.0 * v0 * v0 * v0);
    envelope[3] = v1 * v1 * v1 * (-10.0 + 60.0 * v0 - 60.0 * v0 * v0);
    envelope[4] = v1 * v1 * v1 * v1 * (15.0 - 30.0 * v0);
    envelope[5] = -6.0 * v1 * v1 * v1 * v1 * v1;
}

float windPolyMoments(float poly[WIND_POLY_TERMS]) {
    float sum = 0.0;
    for (int n = 0; n < WIND_POLY_TERMS; ++n) {
        sum += poly[n] / float(n + 1);
    }
    return sum;
}

// Envelope times wall on the piece as a polynomial in sigma. False when the piece holds no shell.
bool windShellPiecePoly(vec3 o, vec3 d, float s0, float s1, out float density[WIND_POLY_TERMS]) {
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        density[k] = 0.0;
    }
    float pieceLength = s1 - s0;
    if (pieceLength <= WIND_EMPTY_INTERVAL_EPSILON) {
        return false;
    }
    vec3 start = o + d * s0;
    float invHeight = 1.0 / windHeight();
    float h0 = start.y * invHeight;
    float h1 = pieceLength * d.y * invHeight;
    float hMid = h0 + 0.5 * h1;
    if (hMid < 0.0 || hMid > windHTop()) {
        return false;
    }

    float q0 = dot(start.xz, start.xz);
    float q1 = 2.0 * pieceLength * dot(start.xz, d.xz);
    float q2 = pieceLength * pieceLength * dot(d.xz, d.xz);

    float radius0 = windWallRadius(h0);
    float radius1 = windWallRadiusSlope() * h1;
    float invWidth = 1.0 / windWallWidthQ();
    float u[WIND_POLY_TERMS];
    windPolyFromQuadratic(
        (q0 - radius0 * radius0 - windSpreadOffset()) * invWidth,
        (q1 - 2.0 * radius0 * radius1) * invWidth,
        (q2 - radius1 * radius1) * invWidth,
        u);
    float uMid = u[0] + 0.5 * u[1] + 0.25 * u[2];
    if (abs(uMid) >= 1.0) {
        return false;
    }

    float wall[WIND_POLY_TERMS];
    windBiweightPoly(u, wall);
    float envelope[WIND_POLY_TERMS];
    float invHTop = 1.0 / windHTop();
    windEnvelopePoly(h0 * invHTop, h1 * invHTop, hMid * invHTop, envelope);
    windPolyMul(envelope, wall, density);
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        density[k] *= windWallStrength();
    }
    return true;
}

// Wall and envelope only: the streak and eddy modulation is dropped for shadow rays.
float windShadowPieceOpticalDepth(vec3 o, vec3 d, float s0, float s1) {
    float density[WIND_POLY_TERMS];
    if (!windShellPiecePoly(o, d, s0, s1, density)) {
        return 0.0;
    }
    return max((s1 - s0) * windSigmaT() * windPolyMoments(density), 0.0);
}

float windPieceOpticalDepth(vec3 o, vec3 d, float s0, float s1) {
    float density[WIND_POLY_TERMS];
    if (!windShellPiecePoly(o, d, s0, s1, density)) {
        return 0.0;
    }
    float pieceLength = s1 - s0;
    vec3 start = o + d * s0;

    if (windStreakAmplitude() > 0.0) {
        float sigma0 = windStreakSigma(start);
        float sigma1 = windStreakSigma(start + pieceLength * d);

        float streakPoly[WIND_POLY_TERMS];
        for (int k = 0; k < WIND_POLY_TERMS; ++k) {
            streakPoly[k] = 0.0;
        }
        streakPoly[0] = sigma0;
        streakPoly[1] = sigma1 - sigma0;

        float modulated[WIND_POLY_TERMS];
        windPolyMul(density, streakPoly, modulated);
        for (int k = 0; k < WIND_POLY_TERMS; ++k) {
            density[k] = modulated[k];
        }
    }

    if (windEddyAmplitude() > 0.0) {
        float total = 0.0;
        float sigmaA = windEddySigma(start);
        for (int j = 0; j < WIND_EDDY_SPLITS; ++j) {
            float a = float(j) / float(WIND_EDDY_SPLITS);
            float b = float(j + 1) / float(WIND_EDDY_SPLITS);
            float sigmaB = windEddySigma(start + b * pieceLength * d);
            float slope = (sigmaB - sigmaA) / (b - a);
            float intercept = sigmaA - slope * a;
            float powA = 1.0;
            float powB = 1.0;
            for (int n = 0; n < WIND_POLY_TERMS; ++n) {
                float m1 = (powB * b * b - powA * a * a) / float(n + 2);
                float m0 = (powB * b - powA * a) / float(n + 1);
                total += density[n] * (intercept * m0 + slope * m1);
                powA *= a;
                powB *= b;
            }
            sigmaA = sigmaB;
        }
        return max(pieceLength * windSigmaT() * total, 0.0);
    }

    return max(pieceLength * windSigmaT() * windPolyMoments(density), 0.0);
}

// Integral of (1 - u^2)^2 over sigma in [0, 1] for u = u0 + u1 sigma + u2 sigma^2.
float windBiweightQuadraticIntegral(float u0, float u1, float u2) {
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

// Puffs are integrated only on the pieces between their own entry and exit knots.
float windPuffPieceOpticalDepth(WindRayPuffs puffs, vec3 o, vec3 d, float s0, float s1) {
    float pieceLength = s1 - s0;
    if (pieceLength <= WIND_EMPTY_INTERVAL_EPSILON || puffs.count == 0) {
        return 0.0;
    }
    float sMid = 0.5 * (s0 + s1);
    vec3 start = o + d * s0;
    float dd = dot(d, d);

    float total = 0.0;
    for (int k = 0; k < puffs.count; ++k) {
        if (sMid <= puffs.enter[k] || sMid >= puffs.exit[k]) continue;
        vec4 puff = wind.puffs[puffs.index[k]];
        vec3 dx = start - puff.xyz;
        float invRSq = 1.0 / (puff.w * puff.w);
        float u0 = dot(dx, dx) * invRSq;
        float u1 = 2.0 * pieceLength * dot(dx, d) * invRSq;
        float u2 = pieceLength * pieceLength * dd * invRSq;
        total += windBiweightQuadraticIntegral(u0, u1, u2);
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

float windOpticalDepthToward(vec3 origin, vec3 direction) {
    float tNear = 0.0;
    float tFar = WIND_SHADOW_RAY_T_MAX;
    if (!clampToWindCone(origin, direction, tNear, tFar)) {
        return 0.0;
    }
    tNear = max(tNear, 0.0);
    return windShadowOpticalDepth(origin, direction, tNear, tFar);
}

float windSunTransmittance(vec3 position, vec3 lightDir) {
    return rteTransmittanceFromOpticalDepth(windOpticalDepthToward(position, lightDir));
}

// Cosine-weighted average over the zenith and six directions tilted 45 degrees, so a uniform
// sky lights the shell from every side instead of through one vertical shadow ray.
float windSkyTransmittance(vec3 position) {
    float weightedSum = rteTransmittanceFromOpticalDepth(
        windOpticalDepthToward(position, vec3(0.0, 1.0, 0.0)));
    float weightSum = 1.0;
    for (int i = 0; i < WIND_SKY_TILTED_DIRECTIONS; ++i) {
        float azimuth = TWO_PI * float(i) / float(WIND_SKY_TILTED_DIRECTIONS);
        vec3 direction = vec3(
            cos(azimuth) * WIND_SKY_TILT_COSINE, WIND_SKY_TILT_COSINE, sin(azimuth) * WIND_SKY_TILT_COSINE);
        weightedSum += WIND_SKY_TILT_COSINE
            * rteTransmittanceFromOpticalDepth(windOpticalDepthToward(position, direction));
        weightSum += WIND_SKY_TILT_COSINE;
    }
    return weightedSum / weightSum;
}

#endif
