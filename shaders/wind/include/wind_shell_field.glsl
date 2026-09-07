#ifndef WIND_SHELL_FIELD_GLSL
#define WIND_SHELL_FIELD_GLSL

// Density field of the tornado: compact-support polynomial shells in q = x^2 + z^2
// (wall around P(h) = (base + slope * h)^2, core around the axis, ground ring around
// Pr faded over its own height) times a height envelope.
// Mirrored in thyllore-effect-core/src/wind/analytic/shell_integral.rs.
// Must be included after wind_component.glsl.

#include "include/noise.glsl"

const float WIND_LINEAR_COEFFICIENT_EPSILON = 1e-7;

float windHeight() { return wind.shape.x; }
float windWallRadiusBase() { return wind.shape.y; }
float windWallRadiusSlope() { return wind.shape.z; }
float windWallWidthQ() { return wind.shape.w; }
float windCoreRadiusSq() { return wind.core.x; }
float windCoreStrength() { return wind.core.y; }
float windWallStrength() { return wind.core.z; }
float windTopFade() { return wind.core.w; }
float windSigmaT() { return wind.optics.x; }
float windSkyBrightness() { return wind.optics.y; }
float windHTop() { return wind.optics.w; }
float windSpreadOffset() { return wind.albedo.w; }
float windRingHeight() { return wind.ring.x; }
float windRingRadiusSq() { return wind.ring.y; }
float windRingWidthQ() { return wind.ring.z; }
float windRingStrength() { return wind.ring.w; }
float windPhaseG() { return wind.lighting.x; }
float windSunIntensity() { return wind.lighting.y; }
float windStreakOrder() { return wind.streak.x; }
float windStreakTwist() { return wind.streak.y; }
float windStreakRiseSpeed() { return wind.streak.z; }
float windStreakAmplitude() { return wind.streak.w; }
float windStreakPhase() { return wind.streak2.x; }
float windStreakRiseTime() { return wind.streak2.y; }
float windEddyAmplitude() { return wind.eddy.x; }
float windEddyCellTheta() { return wind.eddy.y; }
float windEddyCellHeight() { return wind.eddy.z; }
float windEddyCellRadial() { return wind.eddy.w; }
float windEddyShear() { return wind.eddy2.x; }
float windEddyRiseSpeed() { return wind.eddy2.y; }
float windEddyReseedPeriod() { return wind.eddy2.z; }
float windTime() { return wind.optics.z; }

bool windCoreActive() {
    return windCoreRadiusSq() > 1e-8 && windCoreStrength() > 0.0;
}

bool windRingActive() {
    return windRingStrength() > 0.0;
}

float windRingBoundsRadius() {
    return windRingActive() ? sqrt(max(windRingRadiusSq() + windRingWidthQ(), 0.0)) : 0.0;
}

float windRingTopY() {
    return windRingHeight() * windHeight();
}

float windFadeStart() {
    return 1.0 - windTopFade();
}

float windWallRadius(float h) {
    return windWallRadiusBase() + windWallRadiusSlope() * h;
}

float windStreakSigma(vec3 local) {
    float angle = windStreakOrder() * atan(local.z, local.x) - windStreakTwist() * local.y
        - windStreakPhase() + windStreakRiseTime() * local.y;
    return 1.0 + windStreakAmplitude() * cos(angle);
}

float windWallRadiusSq(float h) {
    float radius = windWallRadius(h);
    return radius * radius + windSpreadOffset();
}

float windEddyPeriodicNoise(vec3 u, float periodTheta) {
    vec3 cell = floor(u);
    vec3 f = u - cell;
    vec3 w = f * f * (3.0 - 2.0 * f);

    int p = int(periodTheta);
    int cx = int(mod(cell.x, float(p)));
    int cx1 = int(mod(cell.x + 1.0, float(p)));

    float n000 = hash13(vec3(float(cx), cell.y, cell.z));
    float n100 = hash13(vec3(float(cx1), cell.y, cell.z));
    float n010 = hash13(vec3(float(cx), cell.y + 1.0, cell.z));
    float n110 = hash13(vec3(float(cx1), cell.y + 1.0, cell.z));
    float n001 = hash13(vec3(float(cx), cell.y, cell.z + 1.0));
    float n101 = hash13(vec3(float(cx1), cell.y, cell.z + 1.0));
    float n011 = hash13(vec3(float(cx), cell.y + 1.0, cell.z + 1.0));
    float n111 = hash13(vec3(float(cx1), cell.y + 1.0, cell.z + 1.0));

    float nx00 = mix(n000, n100, w.x);
    float nx10 = mix(n010, n110, w.x);
    float nx01 = mix(n001, n101, w.x);
    float nx11 = mix(n011, n111, w.x);
    float nxy0 = mix(nx00, nx10, w.y);
    float nxy1 = mix(nx01, nx11, w.y);
    return mix(nxy0, nxy1, w.z);
}

float windEddyPeriodicNoiseOctave(vec3 u, float periodTheta, float freq) {
    return windEddyPeriodicNoise(freq * u, periodTheta * freq);
}

float windEddyPeriodicNoiseFBM(vec3 u, float periodTheta) {
    float sum = 0.5 * windEddyPeriodicNoiseOctave(u, periodTheta, 1.0);
    sum += 0.25 * windEddyPeriodicNoiseOctave(u, periodTheta, 2.0);
    sum += 0.125 * windEddyPeriodicNoiseOctave(u, periodTheta, 4.0);
    return sum * (1.0 / 0.875);
}

struct WindEddyGeometry {
    float theta;
    float height;
    float shearRate;
    float radialCoord;
    float periodTheta;
};

WindEddyGeometry windEddyGeometry(vec3 local) {
    float r = length(local.xz);
    float theta = atan(local.z, local.x);
    float h = local.y;
    float wallRadius = windWallRadius(h);

    float omega = (windStreakPhase() / max(windTime(), 1e-3))
        * (windWallRadiusSq(h) / max(r * r, windCoreRadiusSq()));
    float nTheta = max(round(2.0 * 3.14159265 * wallRadius / windEddyCellTheta()), 1.0);

    WindEddyGeometry geometry;
    geometry.theta = theta;
    geometry.height = h;
    geometry.shearRate = omega;
    geometry.radialCoord = (r - wallRadius) / windEddyCellRadial();
    geometry.periodTheta = nTheta;
    return geometry;
}

vec3 windEddyLayerCoords(WindEddyGeometry geometry, float age, float seed) {
    float phi = mix(windStreakPhase(), geometry.shearRate * age, windEddyShear());

    float uTheta = geometry.periodTheta * (geometry.theta - phi) / (2.0 * 3.14159265);
    float uH = (geometry.height - windEddyRiseSpeed() * age) / windEddyCellHeight();

    return vec3(uTheta + seed, uH + seed * 0.37, geometry.radialCoord + seed * 0.61);
}

float windEddySigma(vec3 local) {
    float T = windEddyReseedPeriod();
    float t = windTime();

    float kA = floor(t / T);
    float ageA = t - kA * T;
    float wA = 1.0 - abs(2.0 * ageA / T - 1.0);

    float kB = floor(t / T + 0.5);
    float ageB = t + 0.5 * T - kB * T;
    float wB = 1.0 - wA;

    WindEddyGeometry geometry = windEddyGeometry(local);

    float NA = windEddyPeriodicNoiseFBM(windEddyLayerCoords(geometry, ageA, 17.0 * kA + 3.0), geometry.periodTheta);
    float NB = windEddyPeriodicNoiseFBM(windEddyLayerCoords(geometry, ageB, 17.0 * kB + 3.0), geometry.periodTheta);

    float N = wA * NA + wB * NB;
    return 1.0 + windEddyAmplitude() * (2.0 * N - 1.0);
}

float windEnvelopeRadius(float h) {
    return sqrt(max(max(windWallRadiusSq(h), windCoreRadiusSq()), 0.0)) + sqrt(windWallWidthQ());
}

float windEnvelopeHeight(float h) {
    if (h < 0.0 || h > windHTop()) {
        return 0.0;
    }
    float normalizedHeight = h / windHTop();
    float fadeStart = windFadeStart();
    if (normalizedHeight <= fadeStart) {
        return 1.0;
    }
    float v = (normalizedHeight - fadeStart) / windTopFade();
    return 1.0 - v * v * (3.0 - 2.0 * v);
}

float windBiweight(float u) {
    float inside = max(1.0 - u * u, 0.0);
    return inside * inside;
}

float windRingFade(float v) {
    if (v >= 1.0) {
        return 0.0;
    }
    return 1.0 - v * v * (3.0 - 2.0 * v);
}

float windDensityAt(vec3 p) {
    float h = p.y / windHeight();
    float envelope = windEnvelopeHeight(h);
    if (envelope <= 0.0) {
        return 0.0;
    }
    float q = p.x * p.x + p.z * p.z;

    float wall = windWallStrength() * windBiweight((q - windWallRadiusSq(h)) / windWallWidthQ());
    float core = windCoreActive() ? windCoreStrength() * windBiweight(q / windCoreRadiusSq()) : 0.0;
    float ring = windRingActive()
        ? windRingStrength() * windRingFade(h / windRingHeight())
              * windBiweight((q - windRingRadiusSq()) / windRingWidthQ())
        : 0.0;
    return windSigmaT() * envelope * (wall + core + ring) * windStreakSigma(p) * windEddySigma(p);
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

    if (abs(a) < WIND_LINEAR_COEFFICIENT_EPSILON) {
        if (abs(b) < WIND_LINEAR_COEFFICIENT_EPSILON) {
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

    if (abs(d.y) < WIND_LINEAR_COEFFICIENT_EPSILON) {
        if (o.y < 0.0 || o.y > topY) return false;
    } else {
        float tY0 = -o.y / d.y;
        float tY1 = (topY - o.y) / d.y;
        tNear = max(tNear, min(tY0, tY1));
        tFar = min(tFar, max(tY0, tY1));
    }

    return tNear <= tFar;
}

// Cone frustum (radius linear in height between the envelope radii) x height slab.
// When ring is active, the cone is the union of the wall frustum and the ring frustum.
bool clampToWindCone(vec3 o, vec3 d, inout float tNear, inout float tFar) {
    float topY = windHTop() * windHeight();
    float radiusBase = windEnvelopeRadius(0.0);
    float radiusTop = windEnvelopeRadius(windHTop());

    float savedNear = tNear;
    float savedFar = tFar;
    bool wallHit = clampToConeFrustum(radiusBase, radiusTop, topY, o, d, tNear, tFar);

    if (!windRingActive()) {
        return wallHit;
    }

    float ringRadius = windRingBoundsRadius();
    float ringTopY = windRingTopY();
    tNear = savedNear;
    tFar = savedFar;
    bool ringHit = clampToConeFrustum(ringRadius, ringRadius, ringTopY, o, d, tNear, tFar);

    if (!wallHit) {
        return ringHit;
    }
    float wallNear = tNear;
    float wallFar = tFar;
    tNear = savedNear;
    tFar = savedFar;
    clampToConeFrustum(radiusBase, radiusTop, topY, o, d, tNear, tFar);
    float finalWallNear = tNear;
    float finalWallFar = tFar;
    tNear = savedNear;
    tFar = savedFar;
    clampToConeFrustum(ringRadius, ringRadius, ringTopY, o, d, tNear, tFar);

    tNear = min(finalWallNear, tNear);
    tFar = max(finalWallFar, tFar);
    return true;
}

#endif
