#ifndef WIND_LIGHTING_GLSL
#define WIND_LIGHTING_GLSL

// Single scattering along the view ray: the ray is cut into equal cells, each cell's optical
// depth is the closed-form sum over the knot pieces inside it, and the in-scatter source is
// evaluated once at the cell midpoint, shadowed toward the sun and over the sky hemisphere.
// With WIND_SHADOW_VOLUME the transmittances come from the baked volume (shadowBake.comp),
// otherwise they are integrated inline.
// Must be included after shell_integral.glsl (and shadow_volume.glsl under WIND_SHADOW_VOLUME).

#include "include/radiative_transfer.glsl"

// x: transmittance toward the sun, y: cosine-weighted transmittance over the sky.
vec2 windShadowTransmittances(vec3 position, vec3 lightDir) {
#ifdef WIND_SHADOW_VOLUME
    return windShadowVolumeTransmittances(shadowVolumeSampler, position, windShadowSlot());
#else
    return vec2(windSunTransmittance(position, lightDir), windSkyTransmittance(position));
#endif
}

float windInScatterSource(vec3 position, vec3 lightPosition, vec3 viewDir) {
    vec3 lightDir = normalize(lightPosition - position);
    vec2 transmittances = windShadowTransmittances(position, lightDir);
    return windSunIntensity() * transmittances.x
            * rteHenyeyGreenstein(dot(viewDir, lightDir), windPhaseG())
        + windSkyBrightness() * transmittances.y;
}

bool windCellHoldsShell(vec3 o, vec3 d, float knots[WIND_MAX_KNOTS], int knotCount, float cellStart, float cellEnd) {
    for (int i = 1; i < knotCount; ++i) {
        if (windPieceHoldsShell(o, d, max(knots[i - 1], cellStart), min(knots[i], cellEnd))) {
            return true;
        }
    }
    return false;
}

float windCellOpticalDepth(
    vec3 o, vec3 d, float knots[WIND_MAX_KNOTS], int knotCount, WindRayPuffs puffs,
    float cellStart, float cellEnd, float step, float modulationA, float modulationB) {
    float total = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        float a = max(knots[i - 1], cellStart);
        float b = min(knots[i], cellEnd);
        if (b <= a) {
            continue;
        }
        float modulation0 = mix(modulationA, modulationB, (a - cellStart) / step);
        float modulation1 = mix(modulationA, modulationB, (b - cellStart) / step);
        total += windPieceOpticalDepth(o, d, a, b, modulation0, modulation1)
            + windPuffPieceOpticalDepth(puffs, o, d, a, b);
    }
    return total;
}

vec3 windSingleScatterRadiance(
    vec3 o, vec3 d, float tNear, float tFar, vec3 lightPosition,
    out float opticalDepth, out int knotCount) {
    opticalDepth = 0.0;
    knotCount = 0;
    if (tFar <= tNear) {
        return vec3(0.0);
    }

    float knots[WIND_MAX_KNOTS];
    WindRayPuffs puffs;
    knotCount = windRayKnots(o, d, tNear, tFar, knots, puffs);
    float step = windModulationStep(d, tNear, tFar);
    int cellCount = clamp(int(ceil((tFar - tNear) / step)), 1, WIND_MODULATION_CELLS);
    vec3 viewDir = normalize(d);

    float radiance = 0.0;
    float modulationB = 1.0;
    bool modulationBSampled = false;
    for (int c = 0; c < cellCount; ++c) {
        float cellStart = tNear + float(c) * step;
        float cellEnd = min(cellStart + step, tFar);
        bool holdsShell = windCellHoldsShell(o, d, knots, knotCount, cellStart, cellEnd);
        float modulationA = modulationB;
        if (holdsShell && !modulationBSampled) {
            modulationA = windModulationAt(o + d * cellStart);
        }
        modulationBSampled = holdsShell;
        if (holdsShell) {
            modulationB = windModulationAt(o + d * (cellStart + step));
        }
        float cellDepth = windCellOpticalDepth(
            o, d, knots, knotCount, puffs, cellStart, cellEnd, step, modulationA, modulationB);
        if (cellDepth <= 0.0) {
            continue;
        }

        vec3 node = o + d * (0.5 * (cellStart + cellEnd));
        float source = windInScatterSource(node, lightPosition, viewDir);
        radiance += rteTransmittanceFromOpticalDepth(opticalDepth) * source
            * (1.0 - rteTransmittanceFromOpticalDepth(cellDepth));
        opticalDepth += cellDepth;
    }
    return radiance * wind.albedo.rgb;
}

#endif
