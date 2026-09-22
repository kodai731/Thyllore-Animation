// Closed-form optical depth of the tornado along a ray: the shell and puff knots of
// include/volume_shell.slang and include/volume_puffs.slang, with the streak and eddy modulation
// sampled on a ray-wide cell grid and applied linearly inside every piece.
module wind_integral;

import "wind/include/component.slang";
import "wind/include/field.slang";
import "include/math.slang";
import "include/shadow_rays.slang";
import "include/volume_puffs.slang";
import "include/volume_shell.slang";

// One cell grid spans the whole ray so a knot splitting a piece never moves a sample; the cell
// length comes from the active length (shell or puff pieces) so the budget is spent on density.
public static const int WIND_MODULATION_CELLS = 64;
public static const int WIND_ACTIVE_CELLS_MIN = 16;
public static const float WIND_MODULATION_SAMPLE_FRACTION = 0.125;
public static const float WIND_EMPTY_INTERVAL_EPSILON = 1e-6;
public static const float WIND_SHADOW_RAY_T_MAX = 1e4;

public struct WindPuffs : IPuffSource {
    public int count() { return windPuffCount(); }
    public float4 sphere(int index) { return wind.puffs[index]; }
};

public int windRayKnots(
    VolumeShell shell, float3 o, float3 d, float tNear, float tFar,
    out float knots[RAY_MAX_KNOTS], out RayPuffs puffs) {
    int count = rayBeginKnots(tNear, tFar, knots);
    shellCollectKnots(shell, o, d, tNear, tFar, knots, count);
    WindPuffs source;
    puffsCollectKnots(source, o, d, tNear, tFar, knots, count, puffs);
    raySortKnots(knots, count);
    return count;
}

public float windModulationAt(float3 p, float3 stepAhead) {
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
public float windStreakWavelength() {
    float angular = windStreakOrder() / max(windWallRadiusBase(), 1e-3);
    float vertical = windStreakRiseTime() - windStreakTwist();
    return TWO_PI / max(sqrt(angular * angular + vertical * vertical), 1e-3);
}

public bool windPieceIsActive(VolumeShell shell, float3 o, float3 d, RayPuffs puffs, float s0, float s1) {
    return shellPieceHolds(shell, o, d, s0, s1) || puffsPieceHolds(puffs, s0, s1);
}

// Length of the ray inside the shell support or a puff: continuous in the ray, so the cell
// length derived from it never jumps when a knot appears.
public float windActiveLength(
    VolumeShell shell, float3 o, float3 d, float knots[RAY_MAX_KNOTS], int knotCount, RayPuffs puffs) {
    float total = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        if (windPieceIsActive(shell, o, d, puffs, knots[i - 1], knots[i])) {
            total += knots[i] - knots[i - 1];
        }
    }
    return total;
}

// Cell length along the ray: a fraction of the finest active modulation feature, bounded so the
// active length holds between WIND_ACTIVE_CELLS_MIN and WIND_MODULATION_CELLS cells.
public float windModulationStep(float3 d, float activeLength) {
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

public float windPuffPieceOpticalDepth(VolumeShell shell, RayPuffs puffs, float3 o, float3 d, float s0, float s1) {
    WindPuffs source;
    float integral = puffsPieceIntegral(source, puffs, o, d, s0, s1);
    return max(shell.sigmaT * windPuffStrength() * shell.strength * integral, 0.0);
}

// Shadow ray of include/shadow_rays.slang: wall + envelope depth up to the cone boundary.
public struct WindShadowMedium : IShadowMedium {
    public float shadowOpticalDepthToward(float3 origin, float3 direction) {
        return shellOpticalDepthToward(windShell(), origin, direction, WIND_SHADOW_RAY_T_MAX);
    }
};
