#ifndef WIND_SHELL_INTEGRAL_GLSL
#define WIND_SHELL_INTEGRAL_GLSL

// Closed-form optical depth of the shell field along a ray: the ray is cut at the
// shell support boundaries and the envelope break (knots), and inside each piece
// the density is one polynomial in the piece-local variable sigma in [0, 1] whose
// integral is a sum of power-rule moments.
// Mirrored in thyllore-effect-core/src/wind/analytic/shell_integral.rs.
// Must be included after wind_shell_field.glsl.

const int WIND_MAX_KNOTS = 56;
const int WIND_POLY_TERMS = 16;
// Fixed so the node set is a continuous function of the ray (no seams where a count would change).
const int WIND_EDDY_SPLITS = 8;
const int WIND_PUFFS_PER_RAY = 20;
const float WIND_EMPTY_INTERVAL_EPSILON = 1e-6;

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

int windRayKnots(vec3 o, vec3 d, float tNear, float tFar, bool includePuffs, out float knots[WIND_MAX_KNOTS]) {
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

    if (includePuffs) {
        int adopted = 0;
        for (int i = 0; i < windPuffCount(); ++i) {
            if (adopted >= WIND_PUFFS_PER_RAY) break;
            vec3 c = wind.puffs[i].xyz;
            float r = wind.puffs[i].w;
            if (r <= 0.0) continue;
            vec3 dx = o - c;
            float a = dot(d, d);
            float b = 2.0 * dot(dx, d);
            float cVal = dot(dx, dx) - r * r;
            float discriminant = b * b - 4.0 * a * cVal;
            if (discriminant <= 0.0) continue;
            float sqrtDiscriminant = sqrt(discriminant);
            float t0 = (-b - sqrtDiscriminant) / (2.0 * a);
            float t1 = (-b + sqrtDiscriminant) / (2.0 * a);
            windPushKnot(knots, count, t0, tNear, tFar);
            windPushKnot(knots, count, t1, tNear, tFar);
            adopted += 1;
        }
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
    envelope[0] = 1.0 - 3.0 * v0 * v0 + 2.0 * v0 * v0 * v0;
    envelope[1] = -6.0 * v0 * v1 + 6.0 * v0 * v0 * v1;
    envelope[2] = -3.0 * v1 * v1 + 6.0 * v0 * v1 * v1;
    envelope[3] = 2.0 * v1 * v1 * v1;
}

float windPieceOpticalDepth(vec3 o, vec3 d, float s0, float s1, bool includePuffs) {
    float pieceLength = s1 - s0;
    if (pieceLength <= WIND_EMPTY_INTERVAL_EPSILON) {
        return 0.0;
    }
    vec3 start = o + d * s0;
    float invHeight = 1.0 / windHeight();
    float h0 = start.y * invHeight;
    float h1 = pieceLength * d.y * invHeight;
    float hMid = h0 + 0.5 * h1;
    if (hMid < 0.0) {
        return 0.0;
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

    float shell[WIND_POLY_TERMS];
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        shell[k] = 0.0;
    }
    if (abs(uMid) < 1.0) {
        float wall[WIND_POLY_TERMS];
        windBiweightPoly(u, wall);
        for (int k = 0; k < WIND_POLY_TERMS; ++k) {
            shell[k] += windWallStrength() * wall[k];
        }
    }
    float puffPoly[WIND_POLY_TERMS];
    for (int k = 0; k < WIND_POLY_TERMS; ++k) {
        puffPoly[k] = 0.0;
    }
    if (includePuffs) {
        int adopted = 0;
        for (int i = 0; i < windPuffCount(); ++i) {
            if (adopted >= WIND_PUFFS_PER_RAY) break;
            vec3 c = wind.puffs[i].xyz;
            float r = wind.puffs[i].w;
            if (r <= 0.0) continue;
            vec3 dx = start - c;
            float rSq = r * r;
            float u0 = dot(dx, dx) / rSq;
            float u1 = 2.0 * pieceLength * dot(dx, d) / rSq;
            float u2 = pieceLength * pieceLength * dot(d, d) / rSq;
            float discU = u1 * u1 - 4.0 * u2 * (u0 - 1.0);
            if (discU <= 0.0) continue;
            float uMid = u0 + 0.5 * u1 + 0.25 * u2;
            if (uMid < 1.0) {
                float u[WIND_POLY_TERMS];
                windPolyFromQuadratic(u0, u1, u2, u);
                float bw[WIND_POLY_TERMS];
                windBiweightPoly(u, bw);
                float weight = windPuffStrength() * windWallStrength();
                for (int k = 0; k < WIND_POLY_TERMS; ++k) {
                    puffPoly[k] += weight * bw[k];
                }
            }
            adopted += 1;
        }
    }

    float envelope[WIND_POLY_TERMS];
    float invHTop = 1.0 / windHTop();
    windEnvelopePoly(h0 * invHTop, h1 * invHTop, hMid * invHTop, envelope);
    float density[WIND_POLY_TERMS];
    windPolyMul(envelope, shell, density);

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
        float puffTotal = 0.0;
        for (int n = 0; n < WIND_POLY_TERMS; ++n) {
            puffTotal += puffPoly[n] / float(n + 1);
        }
        return max(pieceLength * windSigmaT() * (total + puffTotal), 0.0);
    }

    float momentSum = 0.0;
    for (int n = 0; n < WIND_POLY_TERMS; ++n) {
        momentSum += density[n] / float(n + 1);
    }
    float puffTotal = 0.0;
    for (int n = 0; n < WIND_POLY_TERMS; ++n) {
        puffTotal += puffPoly[n] / float(n + 1);
    }
    return max(pieceLength * windSigmaT() * (momentSum + puffTotal), 0.0);
}

float windOpticalDepth(vec3 o, vec3 d, float tNear, float tFar, bool includePuffs, out int knotCount) {
    knotCount = 0;
    if (tFar <= tNear) {
        return 0.0;
    }
    float knots[WIND_MAX_KNOTS];
    knotCount = windRayKnots(o, d, tNear, tFar, includePuffs, knots);
    float total = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        total += windPieceOpticalDepth(o, d, knots[i - 1], knots[i], includePuffs);
    }
    return total;
}

#endif
