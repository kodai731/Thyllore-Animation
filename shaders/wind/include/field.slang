// Density field of the tornado: the wall shell of include/volume_shell.slang built from the wind
// UBO, modulated by rotating streaks and eddies.
module wind_field;

import "wind/include/component.slang";
import "include/noise.slang";
import "include/math.slang";
import "include/volume_shell.slang";

public static const float WIND_EDDY_MIN_RADIUS_SQ = 1e-4;
public static const int WIND_EDDY_OCTAVE_COUNT = 3;
// Lattice distance between consecutive cell nodes at which an octave starts to fade and is gone (Nyquist = 0.5).
public static const float WIND_EDDY_FADE_START = 0.25;
public static const float WIND_EDDY_FADE_END = 0.5;
public static const float WIND_EDDY_LAYER_A_SPEED_OFFSET = 0.25;
public static const float WIND_EDDY_LAYER_B_SPEED_OFFSET = -0.25;

public float windHeight() { return wind.shape.x; }
public float windWallRadiusBase() { return wind.shape.y; }
public float windWallRadiusSlope() { return wind.shape.z; }
public float windWallWidthQ() { return wind.shape.w; }
public float windWallStrength() { return wind.wall.x; }
public float windTopFade() { return wind.wall.y; }
public float windSigmaT() { return wind.optics.x; }
public float windSkyBrightness() { return wind.optics.y; }
public float windHTop() { return wind.optics.w; }
public float windSpreadOffset() { return wind.albedo.w; }
public float windPhaseG() { return wind.lighting.x; }
public float windSunIntensity() { return wind.lighting.y; }
public float windCirculation() { return wind.lighting.z; }
public float windSpreadRate() { return wind.lighting.w; }
public float windStreakOrder() { return wind.streak.x; }
public float windStreakTwist() { return wind.streak.y; }
public float windStreakRiseSpeed() { return wind.streak.z; }
public float windStreakAmplitude() { return wind.streak.w; }
public float windStreakPhase() { return wind.streak2.x; }
public float windStreakRiseTime() { return wind.streak2.y; }
public float windEddySpeedSpread() { return wind.streak2.z; }
public float windSpreadStart() { return wind.streak2.w; }
public float windEddyAmplitude() { return wind.eddy.x; }
public float windEddyCellTheta() { return wind.eddy.y; }
public float windEddyCellHeight() { return wind.eddy.z; }
public float windEddyCellRadial() { return wind.eddy.w; }
public float windEddyShear() { return wind.eddy2.x; }
public float windEddyRiseSpeed() { return wind.eddy2.y; }
public float windEddyReseedPeriod() { return wind.eddy2.z; }
public float windEddyErosion() { return wind.eddy2.w; }
public float windTime() { return wind.optics.z; }

public VolumeShell windShell() {
    VolumeShell shell;
    shell.height = windHeight();
    shell.radiusBase = windWallRadiusBase();
    shell.radiusSlope = windWallRadiusSlope();
    shell.radiusOffsetQ = windSpreadOffset();
    shell.widthQ = windWallWidthQ();
    shell.strength = windWallStrength();
    shell.hTop = windHTop();
    shell.topFade = windTopFade();
    shell.sigmaT = windSigmaT();
    return shell;
}

public float windWallRadius(float h) {
    return shellWallRadius(windShell(), h);
}

public float windRotationPhase(float h) {
    float radius_sq = windWallRadius(h) * windWallRadius(h);
    float gamma_over_2pi = windCirculation() / TWO_PI;
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

public float windStreakSigma(float3 local) {
    float angle = windStreakOrder() * (atan2(local.z, local.x) - windRotationPhase(local.y))
        - windStreakTwist() * local.y + windStreakRiseTime() * local.y;
    return 1.0 + windStreakAmplitude() * cos(angle);
}

public float windWallRadiusSq(float h) {
    return shellWallRadiusSq(windShell(), h);
}

// Same coefficients as the GLSL mat3 (column-major constructor): applied as a row vector product.
static const float3x3 WIND_OCTAVE_ROTATION = float3x3(
    0.784750, 0.509329, -0.353201,
    -0.045714, 0.615862, 0.786527,
    0.618124, -0.601081, 0.506581);

float3 windRotate(float3 p) {
    return mul(p, WIND_OCTAVE_ROTATION);
}

float3 windRotateAndDouble(float3 p) {
    return 2.0 * windRotate(p);
}

// Difference against the antipode (theta + pi) has an exact zero mean around every ring,
// so no height can become a uniformly dense or empty band.
float windAntipodalOctave(float3 p, float3 antipode) {
    return (gradientNoise3(p) - gradientNoise3(antipode)) * 0.70710678;
}

struct WindEddyOctaveRings {
    float3 point[WIND_EDDY_OCTAVE_COUNT];
    float3 antipode[WIND_EDDY_OCTAVE_COUNT];
};

// Octaves whose lattice step over one cell exceeds the fade band are dropped; the rest are rescaled
// to keep the variance, so the eroded mean does not drift with the cell length.
float windEddyOctaveWeight(float3 point, float3 pointAhead, int octave) {
    float latticeStep = exp2(float(octave)) * distance(point, pointAhead);
    return 1.0 - smoothstep(WIND_EDDY_FADE_START, WIND_EDDY_FADE_END, latticeStep);
}

float windEddyNoiseFBM(WindEddyOctaveRings rings, WindEddyOctaveRings ringsAhead) {
    float sum = 0.0;
    float amplitude = 0.5;
    float fullVariance = 0.0;
    float keptVariance = 0.0;
    for (int octave = 0; octave < WIND_EDDY_OCTAVE_COUNT; ++octave) {
        fullVariance += amplitude * amplitude;
        float weight = windEddyOctaveWeight(rings.point[octave], ringsAhead.point[octave], octave);
        if (weight > 0.0) {
            float3 p = windRotate(rings.point[octave]);
            float3 antipode = windRotate(rings.antipode[octave]);
            for (int doubling = 0; doubling < octave; ++doubling) {
                p = windRotateAndDouble(p);
                antipode = windRotateAndDouble(antipode);
            }
            sum += weight * amplitude * windAntipodalOctave(p, antipode);
            keptVariance += weight * amplitude * weight * amplitude;
        }
        amplitude *= 0.5;
    }
    if (keptVariance > 0.0) {
        sum *= sqrt(fullVariance / keptVariance);
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

WindEddyGeometry windEddyGeometry(float3 local) {
    float r = length(local.xz);
    float theta = atan2(local.z, local.x);
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

        rings.point[octave] = float3(ringX + seed, ringY + seed * 0.37, uH + seed * 0.61);
        rings.antipode[octave] = float3(-ringX + seed, -ringY + seed * 0.37, uH + seed * 0.61);
    }
    return rings;
}

// stepAhead is the ray step to the next cell node; zero means the pointwise field with every octave kept.
public float windEddySigma(float3 local, float3 stepAhead) {
    float T = windEddyReseedPeriod();
    float t = windTime();

    float kA = floor(t / T);
    float ageA = t - kA * T;
    float wA = 1.0 - abs(2.0 * ageA / T - 1.0);

    float kB = floor(t / T + 0.5);
    float ageB = t + 0.5 * T - kB * T;
    float wB = 1.0 - wA;

    WindEddyGeometry geometry = windEddyGeometry(local);
    WindEddyGeometry geometryAhead = windEddyGeometry(local + stepAhead);
    float seedA = 17.0 * kA + 3.0;
    float seedB = 17.0 * kB + 3.0;

    WindEddyOctaveRings ringsA = windEddyLayerCoords(geometry, ageA, seedA, WIND_EDDY_LAYER_A_SPEED_OFFSET);
    WindEddyOctaveRings ringsB = windEddyLayerCoords(geometry, ageB, seedB, WIND_EDDY_LAYER_B_SPEED_OFFSET);
    WindEddyOctaveRings ringsAAhead = windEddyLayerCoords(geometryAhead, ageA, seedA, WIND_EDDY_LAYER_A_SPEED_OFFSET);
    WindEddyOctaveRings ringsBAhead = windEddyLayerCoords(geometryAhead, ageB, seedB, WIND_EDDY_LAYER_B_SPEED_OFFSET);
    float NA = windEddyNoiseFBM(ringsA, ringsAAhead);
    float NB = windEddyNoiseFBM(ringsB, ringsBAhead);

    float N = wA * NA + wB * NB;
    float eroded = clamp((N - windEddyErosion()) / (1.0 - windEddyErosion()), 0.0, 1.0);
    return 1.0 + windEddyAmplitude() * (2.0 * eroded - 1.0);
}

public float windDensityAt(float3 p) {
    return shellDensityAt(windShell(), p) * windStreakSigma(p) * windEddySigma(p, float3(0.0));
}

public bool clampToWindCone(float3 o, float3 d, inout float tNear, inout float tFar) {
    return clampToShellCone(windShell(), o, d, tNear, tFar);
}
