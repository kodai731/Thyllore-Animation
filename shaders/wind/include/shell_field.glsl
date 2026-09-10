#ifndef WIND_SHELL_FIELD_GLSL
#define WIND_SHELL_FIELD_GLSL

// Density field of the tornado: a compact-support polynomial shell in q = x^2 + z^2
// (wall around P(h) = (base + slope * h)^2) times a height envelope, streak and eddy modulation.
// Mirrored in thyllore-effect-core/src/wind/analytic/shell_integral.rs.
// Must be included after component.glsl.

#include "include/noise.glsl"

const float WIND_LINEAR_COEFFICIENT_EPSILON = 1e-7;
const float WIND_EDDY_MIN_RADIUS_SQ = 1e-4;
const int WIND_EDDY_OCTAVE_COUNT = 3;
const float WIND_EDDY_LAYER_A_SPEED_OFFSET = 0.25;
const float WIND_EDDY_LAYER_B_SPEED_OFFSET = -0.25;

float windHeight() { return wind.shape.x; }
float windWallRadiusBase() { return wind.shape.y; }
float windWallRadiusSlope() { return wind.shape.z; }
float windWallWidthQ() { return wind.shape.w; }
float windWallStrength() { return wind.wall.x; }
float windTopFade() { return wind.wall.y; }
float windSigmaT() { return wind.optics.x; }
float windSkyBrightness() { return wind.optics.y; }
float windHTop() { return wind.optics.w; }
float windSpreadOffset() { return wind.albedo.w; }
float windPhaseG() { return wind.lighting.x; }
float windSunIntensity() { return wind.lighting.y; }
float windCirculation() { return wind.lighting.z; }
float windSpreadRate() { return wind.lighting.w; }
float windStreakOrder() { return wind.streak.x; }
float windStreakTwist() { return wind.streak.y; }
float windStreakRiseSpeed() { return wind.streak.z; }
float windStreakAmplitude() { return wind.streak.w; }
float windStreakPhase() { return wind.streak2.x; }
float windStreakRiseTime() { return wind.streak2.y; }
float windEddySpeedSpread() { return wind.streak2.z; }
float windSpreadStart() { return wind.streak2.w; }
float windEddyAmplitude() { return wind.eddy.x; }
float windEddyCellTheta() { return wind.eddy.y; }
float windEddyCellHeight() { return wind.eddy.z; }
float windEddyCellRadial() { return wind.eddy.w; }
float windEddyShear() { return wind.eddy2.x; }
float windEddyRiseSpeed() { return wind.eddy2.y; }
float windEddyReseedPeriod() { return wind.eddy2.z; }
float windEddyErosion() { return wind.eddy2.w; }
float windTime() { return wind.optics.z; }

float windFadeStart() {
    return 1.0 - windTopFade();
}

float windWallRadius(float h) {
    return windWallRadiusBase() + windWallRadiusSlope() * h;
}

float windRotationPhase(float h) {
    float radius_sq = windWallRadius(h) * windWallRadius(h);
    float gamma_over_2pi = windCirculation() / (2.0 * 3.14159265359);
    float a = windSpreadRate();
    if (a > 0.0) {
        float ts = windSpreadStart();
        float t_clamped = min(windTime(), ts);
        float t_plus = max(windTime() - ts, 0.0);
        return gamma_over_2pi * (t_clamped / radius_sq + (1.0 / (2.0 * a)) * log((radius_sq + 2.0 * a * t_plus) / radius_sq));
    } else {
        return gamma_over_2pi * windTime() / radius_sq;
    }
}

float windStreakSigma(vec3 local) {
    float angle = windStreakOrder() * (atan(local.z, local.x) - windRotationPhase(local.y))
        - windStreakTwist() * local.y + windStreakRiseTime() * local.y;
    return 1.0 + windStreakAmplitude() * cos(angle);
}

float windWallRadiusSq(float h) {
    float radius = windWallRadius(h);
    return radius * radius + windSpreadOffset();
}

float windQuinticFade(float t) {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

vec3 windLatticeGradient(vec3 cell) {
    vec3 g = vec3(
        2.0 * hash13(cell) - 1.0,
        2.0 * hash13(cell + vec3(17.1, 9.3, 4.7)) - 1.0,
        2.0 * hash13(cell + vec3(31.7, 2.9, 12.3)) - 1.0);
    return g / max(length(g), 1e-4);
}

float windCornerDot(vec3 cell, vec3 f, vec3 corner) {
    return dot(windLatticeGradient(cell + corner), f - corner);
}

float windGradientNoise(vec3 p) {
    vec3 cell = floor(p);
    vec3 f = p - cell;
    vec3 w = vec3(windQuinticFade(f.x), windQuinticFade(f.y), windQuinticFade(f.z));

    float nx00 = mix(windCornerDot(cell, f, vec3(0.0, 0.0, 0.0)), windCornerDot(cell, f, vec3(1.0, 0.0, 0.0)), w.x);
    float nx10 = mix(windCornerDot(cell, f, vec3(0.0, 1.0, 0.0)), windCornerDot(cell, f, vec3(1.0, 1.0, 0.0)), w.x);
    float nx01 = mix(windCornerDot(cell, f, vec3(0.0, 0.0, 1.0)), windCornerDot(cell, f, vec3(1.0, 0.0, 1.0)), w.x);
    float nx11 = mix(windCornerDot(cell, f, vec3(0.0, 1.0, 1.0)), windCornerDot(cell, f, vec3(1.0, 1.0, 1.0)), w.x);
    float nxy0 = mix(nx00, nx10, w.y);
    float nxy1 = mix(nx01, nx11, w.y);
    return mix(nxy0, nxy1, w.z);
}

const mat3 WIND_OCTAVE_ROTATION = mat3(
    0.784750, 0.509329, -0.353201,
    -0.045714, 0.615862, 0.786527,
    0.618124, -0.601081, 0.506581);

vec3 windRotateAndDouble(vec3 p) {
    return 2.0 * (WIND_OCTAVE_ROTATION * p);
}

// Difference against the antipode (theta + pi) has an exact zero mean around every ring,
// so no height can become a uniformly dense or empty band.
float windAntipodalOctave(vec3 p, vec3 antipode) {
    return (windGradientNoise(p) - windGradientNoise(antipode)) * 0.70710678;
}

struct WindEddyOctaveRings {
    vec3 point[WIND_EDDY_OCTAVE_COUNT];
    vec3 antipode[WIND_EDDY_OCTAVE_COUNT];
};

float windEddyNoiseFBM(WindEddyOctaveRings rings) {
    float sum = 0.0;
    float amplitude = 0.5;
    for (int octave = 0; octave < WIND_EDDY_OCTAVE_COUNT; ++octave) {
        vec3 p = WIND_OCTAVE_ROTATION * rings.point[octave];
        vec3 antipode = WIND_OCTAVE_ROTATION * rings.antipode[octave];
        for (int doubling = 0; doubling < octave; ++doubling) {
            p = windRotateAndDouble(p);
            antipode = windRotateAndDouble(antipode);
        }
        sum += amplitude * windAntipodalOctave(p, antipode);
        amplitude *= 0.5;
    }
    return clamp(0.5 + sum * (1.0 / 0.875), 0.0, 1.0);
}

struct WindEddyGeometry {
    float theta;
    float height;
    float radius;
    float radialCoord;
    float ringRadius;
};

WindEddyGeometry windEddyGeometry(vec3 local) {
    float r = length(local.xz);
    float theta = atan(local.z, local.x);
    float h = local.y;
    float wallRadius = windWallRadius(h);

    WindEddyGeometry geometry;
    geometry.theta = theta;
    geometry.height = h;
    geometry.radius = r;
    geometry.radialCoord = (r - wallRadius) / windEddyCellRadial();
    geometry.ringRadius = wallRadius / windEddyCellTheta();
    return geometry;
}

WindEddyOctaveRings windEddyLayerCoords(WindEddyGeometry geometry, float age, float seed, float layerSpeedOffset) {
    float phase = windRotationPhase(geometry.height);
    float shear = pow(
        windWallRadiusSq(geometry.height) / max(geometry.radius * geometry.radius, WIND_EDDY_MIN_RADIUS_SQ),
        windEddyShear());

    float rho = geometry.ringRadius + geometry.radialCoord;
    float uH = (geometry.height - windEddyRiseSpeed() * age) / windEddyCellHeight();

    WindEddyOctaveRings rings;
    for (int octave = 0; octave < WIND_EDDY_OCTAVE_COUNT; ++octave) {
        float spreadCoefficient = (float(octave) - 1.0) * 0.5 + layerSpeedOffset;
        float speedFactor = 1.0 + windEddySpeedSpread() * spreadCoefficient;
        float shearedTheta = geometry.theta - phase * shear * speedFactor;
        float ringX = rho * cos(shearedTheta);
        float ringY = rho * sin(shearedTheta);

        rings.point[octave] = vec3(ringX + seed, ringY + seed * 0.37, uH + seed * 0.61);
        rings.antipode[octave] = vec3(-ringX + seed, -ringY + seed * 0.37, uH + seed * 0.61);
    }
    return rings;
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

    WindEddyOctaveRings ringsA = windEddyLayerCoords(geometry, ageA, 17.0 * kA + 3.0, WIND_EDDY_LAYER_A_SPEED_OFFSET);
    WindEddyOctaveRings ringsB = windEddyLayerCoords(geometry, ageB, 17.0 * kB + 3.0, WIND_EDDY_LAYER_B_SPEED_OFFSET);
    float NA = windEddyNoiseFBM(ringsA);
    float NB = windEddyNoiseFBM(ringsB);

    float N = wA * NA + wB * NB;
    float eroded = clamp((N - windEddyErosion()) / (1.0 - windEddyErosion()), 0.0, 1.0);
    return 1.0 + windEddyAmplitude() * (2.0 * eroded - 1.0);
}

float windEnvelopeRadius(float h) {
    return sqrt(max(windWallRadiusSq(h), 0.0)) + sqrt(windWallWidthQ());
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
    return 1.0 - v * v * v * (10.0 - v * (15.0 - 6.0 * v));
}

float windBiweight(float u) {
    float inside = max(1.0 - u * u, 0.0);
    return inside * inside;
}

float windDensityAt(vec3 p) {
    float h = p.y / windHeight();
    float envelope = windEnvelopeHeight(h);
    if (envelope <= 0.0) {
        return 0.0;
    }
    float q = p.x * p.x + p.z * p.z;

    float wall = windWallStrength() * windBiweight((q - windWallRadiusSq(h)) / windWallWidthQ());
    return windSigmaT() * envelope * wall * windStreakSigma(p) * windEddySigma(p);
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

bool clampToWindCone(vec3 o, vec3 d, inout float tNear, inout float tFar) {
    float topY = windHTop() * windHeight();
    float radiusBase = windEnvelopeRadius(0.0);
    float radiusTop = windEnvelopeRadius(windHTop());
    return clampToConeFrustum(radiusBase, radiusTop, topY, o, d, tNear, tFar);
}

#endif
