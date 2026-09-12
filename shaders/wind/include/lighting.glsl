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
    return shadowVolumeTransmittances(shadowVolumeSampler, position, windShadowSlot());
#else
    return vec2(sunTransmittance(position, lightDir), skyTransmittance(position));
#endif
}

float windInScatterSource(vec3 position, vec3 lightPosition, vec3 viewDir) {
    vec3 lightDir = normalize(lightPosition - position);
    vec2 transmittances = windShadowTransmittances(position, lightDir);
    return windSunIntensity() * transmittances.x
            * rteHenyeyGreenstein(dot(viewDir, lightDir), windPhaseG())
        + windSkyBrightness() * transmittances.y;
}

// Modulation at both nodes of a cell; a node shared with the previous cell is not re-evaluated.
struct WindCellModulation {
    bool sampled;
    int cell;
    float a;
    float b;
};

WindCellModulation windCellModulation(WindCellModulation previous, vec3 o, vec3 d, float tNear, float step, int cell) {
    if (previous.sampled && cell == previous.cell) {
        return previous;
    }
    float cellStart = tNear + float(cell) * step;
    WindCellModulation current;
    current.sampled = true;
    current.cell = cell;
    current.a = previous.sampled && cell == previous.cell + 1
        ? previous.b
        : windModulationAt(o + d * cellStart, d * step);
    current.b = windModulationAt(o + d * (cellStart + step), d * step);
    return current;
}

// Pieces are walked in ray order and each is cut by the cells it overlaps; a cell shared by two
// pieces composes exactly because the transmittance accumulates between the two parts.
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
    float activeLength = windActiveLength(o, d, knots, knotCount, puffs);
    if (activeLength <= WIND_EMPTY_INTERVAL_EPSILON) {
        return vec3(0.0);
    }
    float step = windModulationStep(d, activeLength);
    vec3 viewDir = normalize(d);

    float radiance = 0.0;
    WindCellModulation modulation;
    modulation.sampled = false;
    modulation.cell = 0;
    modulation.a = 1.0;
    modulation.b = 1.0;
    for (int i = 1; i < knotCount; ++i) {
        float pieceStart = knots[i - 1];
        float pieceEnd = knots[i];
        if (!windPieceIsActive(o, d, puffs, pieceStart, pieceEnd)) {
            continue;
        }
        int cellFirst = int(floor((pieceStart - tNear) / step));
        int cellLast = min(int(floor((pieceEnd - tNear) / step)), cellFirst + WIND_MODULATION_CELLS);
        for (int c = cellFirst; c <= cellLast; ++c) {
            float cellStart = tNear + float(c) * step;
            float s0 = max(pieceStart, cellStart);
            float s1 = min(pieceEnd, cellStart + step);
            if (s1 <= s0) {
                continue;
            }
            modulation = windCellModulation(modulation, o, d, tNear, step, c);
            float modulation0 = mix(modulation.a, modulation.b, (s0 - cellStart) / step);
            float modulation1 = mix(modulation.a, modulation.b, (s1 - cellStart) / step);
            float depth = windPieceOpticalDepth(o, d, s0, s1, modulation0, modulation1)
                + windPuffPieceOpticalDepth(puffs, o, d, s0, s1);
            if (depth <= 0.0) {
                continue;
            }

            vec3 node = o + d * (0.5 * (cellStart + min(cellStart + step, tFar)));
            float source = windInScatterSource(node, lightPosition, viewDir);
            radiance += rteTransmittanceFromOpticalDepth(opticalDepth) * source
                * (1.0 - rteTransmittanceFromOpticalDepth(depth));
            opticalDepth += depth;
        }
    }
    return radiance * wind.albedo.rgb;
}

#endif
