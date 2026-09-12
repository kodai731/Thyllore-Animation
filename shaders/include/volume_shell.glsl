#ifndef VOLUME_SHELL_GLSL
#define VOLUME_SHELL_GLSL

// Wall of a participating medium as a compact-support shell in q = x^2 + z^2 around a radius
// linear in the normalised height h = y / height:
//   density = strength * B((q - P(h)) / widthQ),  P(h) = (radiusBase + radiusSlope * h)^2 + radiusOffsetQ
// with B(u) = (1 - u^2)^2 on |u| < 1, times a smootherstep envelope that fades over the top
// `topFade` fraction of hTop. Along a ray q is quadratic and h linear in the parameter, so on
// every piece between knots the density is one polynomial integrated by exact power moments.
// Mirrored in thyllore-effect-core/src/volume/shell.rs.

#include "include/common.glsl"
#include "include/compact_support.glsl"
#include "include/polynomial.glsl"
#include "include/ray_knots.glsl"

const float SHELL_EMPTY_INTERVAL_EPSILON = 1e-6;

struct VolumeShell {
    float height;
    float radiusBase;
    float radiusSlope;
    float radiusOffsetQ;
    float widthQ;
    float strength;
    float hTop;
    float topFade;
    float sigmaT;
};

float shellFadeStart(VolumeShell shell) {
    return 1.0 - shell.topFade;
}

float shellWallRadius(VolumeShell shell, float h) {
    return shell.radiusBase + shell.radiusSlope * h;
}

float shellWallRadiusSq(VolumeShell shell, float h) {
    float radius = shellWallRadius(shell, h);
    return radius * radius + shell.radiusOffsetQ;
}

float shellEnvelopeRadius(VolumeShell shell, float h) {
    return sqrt(max(shellWallRadiusSq(shell, h), 0.0)) + sqrt(shell.widthQ);
}

float shellEnvelopeHeight(VolumeShell shell, float h) {
    if (h < 0.0 || h > shell.hTop) {
        return 0.0;
    }
    float normalizedHeight = h / shell.hTop;
    float fadeStart = shellFadeStart(shell);
    if (normalizedHeight <= fadeStart) {
        return 1.0;
    }
    float v = (normalizedHeight - fadeStart) / shell.topFade;
    return 1.0 - v * v * v * (10.0 - v * (15.0 - 6.0 * v));
}

float shellWallAt(VolumeShell shell, float q, float h) {
    return shell.strength * biweight((q - shellWallRadiusSq(shell, h)) / shell.widthQ);
}

float shellDensityAt(VolumeShell shell, vec3 p) {
    float h = p.y / shell.height;
    float envelope = shellEnvelopeHeight(shell, h);
    if (envelope <= 0.0) {
        return 0.0;
    }
    float q = p.x * p.x + p.z * p.z;
    return shell.sigmaT * envelope * shellWallAt(shell, q, h);
}

bool clampToConeFrustum(
    float radiusBase, float radiusTop, float topY,
    vec3 o, vec3 d, inout float tNear, inout float tFar) {
    float slopePerUnitY = (radiusTop - radiusBase) / topY;
    float m = radiusBase + slopePerUnitY * o.y;
    float n = slopePerUnitY * d.y;
    float a = dot(d.xz, d.xz) - n * n;
    float b = 2.0 * (dot(o.xz, d.xz) - m * n);
    float c = dot(o.xz, o.xz) - m * m;

    if (abs(a) < RAY_LINEAR_COEFFICIENT_EPSILON) {
        if (abs(b) < RAY_LINEAR_COEFFICIENT_EPSILON) {
            if (c > 0.0) return false;
        } else {
            float tRoot = -c / b;
            if (b > 0.0) {
                tFar = min(tFar, tRoot);
            } else {
                tNear = max(tNear, tRoot);
            }
        }
    } else {
        float discriminant = b * b - 4.0 * a * c;
        if (discriminant < 0.0) {
            if (a > 0.0) return false;
        } else if (a > 0.0) {
            float sqrtDiscriminant = sqrt(discriminant);
            float t0 = (-b - sqrtDiscriminant) / (2.0 * a);
            float t1 = (-b + sqrtDiscriminant) / (2.0 * a);
            tNear = max(tNear, min(t0, t1));
            tFar = min(tFar, max(t0, t1));
        }
    }

    if (abs(d.y) < RAY_LINEAR_COEFFICIENT_EPSILON) {
        if (o.y < 0.0 || o.y > topY) return false;
    } else {
        float tY0 = -o.y / d.y;
        float tY1 = (topY - o.y) / d.y;
        tNear = max(tNear, min(tY0, tY1));
        tFar = min(tFar, max(tY0, tY1));
    }

    return tNear <= tFar;
}

bool clampToShellCone(VolumeShell shell, vec3 o, vec3 d, inout float tNear, inout float tFar) {
    float topY = shell.hTop * shell.height;
    float radiusBase = shellEnvelopeRadius(shell, 0.0);
    float radiusTop = shellEnvelopeRadius(shell, shell.hTop);
    return clampToConeFrustum(radiusBase, radiusTop, topY, o, d, tNear, tFar);
}

// Wall support boundaries and the envelope break of the ray, appended unsorted.
void shellCollectKnots(
    VolumeShell shell, vec3 o, vec3 d, float tNear, float tFar,
    inout float knots[RAY_MAX_KNOTS], inout int count) {
    float qA = dot(d.xz, d.xz);
    float qB = 2.0 * dot(o.xz, d.xz);
    float qC = dot(o.xz, o.xz);

    float invHeight = 1.0 / shell.height;
    float radius0 = shellWallRadius(shell, o.y * invHeight);
    float radius1 = shell.radiusSlope * d.y * invHeight;
    float deltaA = qA - radius1 * radius1;
    float deltaB = qB - 2.0 * radius0 * radius1;
    float deltaC = qC - radius0 * radius0 - shell.radiusOffsetQ;
    rayPushQuadraticRoots(deltaA, deltaB, deltaC - shell.widthQ, tNear, tFar, knots, count);
    rayPushQuadraticRoots(deltaA, deltaB, deltaC + shell.widthQ, tNear, tFar, knots, count);

    if (abs(d.y) >= RAY_LINEAR_COEFFICIENT_EPSILON) {
        float fadeY = shellFadeStart(shell) * shell.hTop * shell.height;
        rayPushKnot(knots, count, (fadeY - o.y) / d.y, tNear, tFar);
    }
}

int shellKnots(VolumeShell shell, vec3 o, vec3 d, float tNear, float tFar, out float knots[RAY_MAX_KNOTS]) {
    int count = rayBeginKnots(tNear, tFar, knots);
    shellCollectKnots(shell, o, d, tNear, tFar, knots, count);
    raySortKnots(knots, count);
    return count;
}

void shellEnvelopePoly(VolumeShell shell, float h0, float h1, float hMid, out float envelope[POLY_TERMS]) {
    float fadeStart = shellFadeStart(shell);
    if (hMid <= fadeStart) {
        polyZero(envelope);
        envelope[0] = 1.0;
        return;
    }
    oneMinusSmootherstepPoly((h0 - fadeStart) / shell.topFade, h1 / shell.topFade, envelope);
}

float shellPieceDistanceAtMid(VolumeShell shell, vec3 start, vec3 d, float pieceLength, float hMid) {
    vec3 mid = start + 0.5 * pieceLength * d;
    float radiusMid = shellWallRadius(shell, hMid);
    return (dot(mid.xz, mid.xz) - radiusMid * radiusMid - shell.radiusOffsetQ) / shell.widthQ;
}

// True when the piece [s0, s1], which must not cross a knot, lies inside the wall support.
bool shellPieceHolds(VolumeShell shell, vec3 o, vec3 d, float s0, float s1) {
    float pieceLength = s1 - s0;
    if (pieceLength <= SHELL_EMPTY_INTERVAL_EPSILON) {
        return false;
    }
    vec3 start = o + d * s0;
    float hMid = (start.y + 0.5 * pieceLength * d.y) / shell.height;
    if (hMid < 0.0 || hMid > shell.hTop) {
        return false;
    }
    return abs(shellPieceDistanceAtMid(shell, start, d, pieceLength, hMid)) < 1.0;
}

// Envelope times wall on the piece as a polynomial in sigma. False when the piece holds no wall.
bool shellPiecePoly(VolumeShell shell, vec3 o, vec3 d, float s0, float s1, out float density[POLY_TERMS]) {
    polyZero(density);
    if (!shellPieceHolds(shell, o, d, s0, s1)) {
        return false;
    }
    float pieceLength = s1 - s0;
    vec3 start = o + d * s0;
    float invHeight = 1.0 / shell.height;
    float h0 = start.y * invHeight;
    float h1 = pieceLength * d.y * invHeight;
    float hMid = h0 + 0.5 * h1;

    float q0 = dot(start.xz, start.xz);
    float q1 = 2.0 * pieceLength * dot(start.xz, d.xz);
    float q2 = pieceLength * pieceLength * dot(d.xz, d.xz);

    float radius0 = shellWallRadius(shell, h0);
    float radius1 = shell.radiusSlope * h1;
    float invWidth = 1.0 / shell.widthQ;
    float u[POLY_TERMS];
    polyFromQuadratic(
        (q0 - radius0 * radius0 - shell.radiusOffsetQ) * invWidth,
        (q1 - 2.0 * radius0 * radius1) * invWidth,
        (q2 - radius1 * radius1) * invWidth,
        u);

    float wall[POLY_TERMS];
    biweightPoly(u, wall);
    float envelope[POLY_TERMS];
    float invHTop = 1.0 / shell.hTop;
    shellEnvelopePoly(shell, h0 * invHTop, h1 * invHTop, hMid * invHTop, envelope);
    polyMul(envelope, wall, density);
    polyScale(density, shell.strength);
    return true;
}

float shellPieceOpticalDepth(VolumeShell shell, vec3 o, vec3 d, float s0, float s1) {
    float density[POLY_TERMS];
    if (!shellPiecePoly(shell, o, d, s0, s1, density)) {
        return 0.0;
    }
    return max((s1 - s0) * shell.sigmaT * polyMoments(density), 0.0);
}

// A multiplicative modulation linear on the piece between the given end values.
float shellPieceOpticalDepthModulated(
    VolumeShell shell, vec3 o, vec3 d, float s0, float s1, float modulation0, float modulation1) {
    float density[POLY_TERMS];
    if (!shellPiecePoly(shell, o, d, s0, s1, density)) {
        return 0.0;
    }
    float total = polyLinearWeightedMoments(density, modulation0, modulation1);
    return max((s1 - s0) * shell.sigmaT * total, 0.0);
}

float shellOpticalDepth(VolumeShell shell, vec3 o, vec3 d, float tNear, float tFar) {
    if (tFar <= tNear) {
        return 0.0;
    }
    float knots[RAY_MAX_KNOTS];
    int knotCount = shellKnots(shell, o, d, tNear, tFar, knots);
    float total = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        total += shellPieceOpticalDepth(shell, o, d, knots[i - 1], knots[i]);
    }
    return total;
}

float shellOpticalDepthToward(VolumeShell shell, vec3 origin, vec3 direction, float tMax) {
    float tNear = 0.0;
    float tFar = tMax;
    if (!clampToShellCone(shell, origin, direction, tNear, tFar)) {
        return 0.0;
    }
    tNear = max(tNear, 0.0);
    return shellOpticalDepth(shell, origin, direction, tNear, tFar);
}

#endif
