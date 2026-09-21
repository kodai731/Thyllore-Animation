// Wall of a participating medium as a compact-support shell in q = x^2 + z^2 around a radius
// linear in the normalised height h = y / height:
//   density = strength * B((q - P(h)) / widthQ),  P(h) = (radiusBase + radiusSlope * h)^2 + radiusOffsetQ
// with B(u) = (1 - u^2)^2 on |u| < 1, times a smootherstep envelope that fades over the top
// `topFade` fraction of hTop. Along a ray q is quadratic and h linear in the parameter, so on
// every piece between knots the density is one polynomial integrated by exact power moments.
// Single source for the SPIR-V (rchit), GLSL (Blender) and C++ (CPU tests / pick) targets.
module volume_shell;

public static const int POLY_TERMS = 16;
public static const int RAY_MAX_KNOTS = 56;
public static const float RAY_LINEAR_COEFFICIENT_EPSILON = 1e-7;
public static const float SHELL_EMPTY_INTERVAL_EPSILON = 1e-6;

public void polyZero(out float poly[POLY_TERMS]) {
    for (int k = 0; k < POLY_TERMS; ++k) {
        poly[k] = 0.0;
    }
}

public void polyFromQuadratic(float c0, float c1, float c2, out float poly[POLY_TERMS]) {
    polyZero(poly);
    poly[0] = c0;
    poly[1] = c1;
    poly[2] = c2;
}

// Product truncated to POLY_TERMS coefficients.
public void polyMul(float a[POLY_TERMS], float b[POLY_TERMS], out float product[POLY_TERMS]) {
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

public void polyScale(inout float poly[POLY_TERMS], float factor) {
    for (int k = 0; k < POLY_TERMS; ++k) {
        poly[k] *= factor;
    }
}

// Integral of the polynomial over sigma in [0, 1].
public float polyMoments(float poly[POLY_TERMS]) {
    float sum = 0.0;
    for (int n = 0; n < POLY_TERMS; ++n) {
        sum += poly[n] / float(n + 1);
    }
    return sum;
}

// Integral over sigma in [0, 1] of the polynomial times the linear weight w0 + (w1 - w0) sigma.
public float polyLinearWeightedMoments(float poly[POLY_TERMS], float w0, float w1) {
    float sum = 0.0;
    for (int n = 0; n < POLY_TERMS; ++n) {
        sum += poly[n] * (w0 / float(n + 1) + (w1 - w0) / float(n + 2));
    }
    return sum;
}

// Coefficients of 1 - S(v0 + v1 sigma) for the quintic smootherstep S(v) = 10v^3 - 15v^4 + 6v^5.
public void oneMinusSmootherstepPoly(float v0, float v1, out float poly[POLY_TERMS]) {
    polyZero(poly);
    poly[0] = 1.0 - 10.0 * v0 * v0 * v0 + 15.0 * v0 * v0 * v0 * v0 - 6.0 * v0 * v0 * v0 * v0 * v0;
    poly[1] = v1 * (-30.0 * v0 * v0 + 60.0 * v0 * v0 * v0 - 30.0 * v0 * v0 * v0 * v0);
    poly[2] = v1 * v1 * (-30.0 * v0 + 90.0 * v0 * v0 - 60.0 * v0 * v0 * v0);
    poly[3] = v1 * v1 * v1 * (-10.0 + 60.0 * v0 - 60.0 * v0 * v0);
    poly[4] = v1 * v1 * v1 * v1 * (15.0 - 30.0 * v0);
    poly[5] = -6.0 * v1 * v1 * v1 * v1 * v1;
}

public float biweightProfile(float uSquared) {
    float inside = max(1.0 - uSquared, 0.0);
    return inside * inside;
}

public float biweight(float u) {
    return biweightProfile(u * u);
}

// (1 - u(sigma)^2)^2 for a polynomial u, without the support clamp: valid on pieces inside the support.
public void biweightPoly(float u[POLY_TERMS], out float result[POLY_TERMS]) {
    float inside[POLY_TERMS];
    polyMul(u, u, inside);
    for (int k = 0; k < POLY_TERMS; ++k) {
        inside[k] = -inside[k];
    }
    inside[0] += 1.0;
    polyMul(inside, inside, result);
}

public void rayPushKnot(inout float knots[RAY_MAX_KNOTS], inout int count, float t, float lo, float hi) {
    if (t <= lo || t >= hi || count >= RAY_MAX_KNOTS) {
        return;
    }
    knots[count] = t;
    count += 1;
}

public void rayPushQuadraticRoots(
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

public void raySortKnots(inout float knots[RAY_MAX_KNOTS], int count) {
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

public int rayBeginKnots(float tNear, float tFar, out float knots[RAY_MAX_KNOTS]) {
    knots[0] = tNear;
    knots[1] = tFar;
    return 2;
}

public struct VolumeShell {
    public float height;
    public float radiusBase;
    public float radiusSlope;
    public float radiusOffsetQ;
    public float widthQ;
    public float strength;
    public float hTop;
    public float topFade;
    public float sigmaT;
};

public float shellFadeStart(VolumeShell shell) {
    return 1.0 - shell.topFade;
}

public float shellWallRadius(VolumeShell shell, float h) {
    return shell.radiusBase + shell.radiusSlope * h;
}

public float shellWallRadiusSq(VolumeShell shell, float h) {
    float radius = shellWallRadius(shell, h);
    return radius * radius + shell.radiusOffsetQ;
}

public float shellEnvelopeRadius(VolumeShell shell, float h) {
    return sqrt(max(shellWallRadiusSq(shell, h), 0.0)) + sqrt(shell.widthQ);
}

public float shellEnvelopeHeight(VolumeShell shell, float h) {
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

public float shellWallAt(VolumeShell shell, float q, float h) {
    return shell.strength * biweight((q - shellWallRadiusSq(shell, h)) / shell.widthQ);
}

public float shellDensityAt(VolumeShell shell, float3 p) {
    float h = p.y / shell.height;
    float envelope = shellEnvelopeHeight(shell, h);
    if (envelope <= 0.0) {
        return 0.0;
    }
    float q = p.x * p.x + p.z * p.z;
    return shell.sigmaT * envelope * shellWallAt(shell, q, h);
}

public bool clampToConeFrustum(
    float radiusBase, float radiusTop, float topY,
    float3 o, float3 d, inout float tNear, inout float tFar) {
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

public bool clampToShellCone(VolumeShell shell, float3 o, float3 d, inout float tNear, inout float tFar) {
    float topY = shell.hTop * shell.height;
    float radiusBase = shellEnvelopeRadius(shell, 0.0);
    float radiusTop = shellEnvelopeRadius(shell, shell.hTop);
    return clampToConeFrustum(radiusBase, radiusTop, topY, o, d, tNear, tFar);
}

// Wall support boundaries and the envelope break of the ray, appended unsorted.
public void shellCollectKnots(
    VolumeShell shell, float3 o, float3 d, float tNear, float tFar,
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

public int shellKnots(VolumeShell shell, float3 o, float3 d, float tNear, float tFar, out float knots[RAY_MAX_KNOTS]) {
    int count = rayBeginKnots(tNear, tFar, knots);
    shellCollectKnots(shell, o, d, tNear, tFar, knots, count);
    raySortKnots(knots, count);
    return count;
}

public void shellEnvelopePoly(VolumeShell shell, float h0, float h1, float hMid, out float envelope[POLY_TERMS]) {
    float fadeStart = shellFadeStart(shell);
    if (hMid <= fadeStart) {
        polyZero(envelope);
        envelope[0] = 1.0;
        return;
    }
    oneMinusSmootherstepPoly((h0 - fadeStart) / shell.topFade, h1 / shell.topFade, envelope);
}

public float shellPieceDistanceAtMid(VolumeShell shell, float3 start, float3 d, float pieceLength, float hMid) {
    float3 mid = start + 0.5 * pieceLength * d;
    float radiusMid = shellWallRadius(shell, hMid);
    return (dot(mid.xz, mid.xz) - radiusMid * radiusMid - shell.radiusOffsetQ) / shell.widthQ;
}

// True when the piece [s0, s1], which must not cross a knot, lies inside the wall support.
public bool shellPieceHolds(VolumeShell shell, float3 o, float3 d, float s0, float s1) {
    float pieceLength = s1 - s0;
    if (pieceLength <= SHELL_EMPTY_INTERVAL_EPSILON) {
        return false;
    }
    float3 start = o + d * s0;
    float hMid = (start.y + 0.5 * pieceLength * d.y) / shell.height;
    if (hMid < 0.0 || hMid > shell.hTop) {
        return false;
    }
    return abs(shellPieceDistanceAtMid(shell, start, d, pieceLength, hMid)) < 1.0;
}

// Envelope times wall on the piece as a polynomial in sigma. False when the piece holds no wall.
public bool shellPiecePoly(VolumeShell shell, float3 o, float3 d, float s0, float s1, out float density[POLY_TERMS]) {
    polyZero(density);
    if (!shellPieceHolds(shell, o, d, s0, s1)) {
        return false;
    }
    float pieceLength = s1 - s0;
    float3 start = o + d * s0;
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

public float shellPieceOpticalDepth(VolumeShell shell, float3 o, float3 d, float s0, float s1) {
    float density[POLY_TERMS];
    if (!shellPiecePoly(shell, o, d, s0, s1, density)) {
        return 0.0;
    }
    return max((s1 - s0) * shell.sigmaT * polyMoments(density), 0.0);
}

// A multiplicative modulation linear on the piece between the given end values.
public float shellPieceOpticalDepthModulated(
    VolumeShell shell, float3 o, float3 d, float s0, float s1, float modulation0, float modulation1) {
    float density[POLY_TERMS];
    if (!shellPiecePoly(shell, o, d, s0, s1, density)) {
        return 0.0;
    }
    float total = polyLinearWeightedMoments(density, modulation0, modulation1);
    return max((s1 - s0) * shell.sigmaT * total, 0.0);
}

public float shellOpticalDepth(VolumeShell shell, float3 o, float3 d, float tNear, float tFar) {
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

public float shellOpticalDepthToward(VolumeShell shell, float3 origin, float3 direction, float tMax) {
    float tNear = 0.0;
    float tFar = tMax;
    if (!clampToShellCone(shell, origin, direction, tNear, tFar)) {
        return 0.0;
    }
    tNear = max(tNear, 0.0);
    return shellOpticalDepth(shell, origin, direction, tNear, tFar);
}
