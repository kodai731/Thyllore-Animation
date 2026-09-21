// Single scattering along the view ray: the ray is cut into equal cells, each cell's optical
// depth is the closed-form sum over the knot pieces inside it, and the in-scatter source is
// evaluated once at the cell midpoint, shadowed toward the sun and over the sky hemisphere.
// With WIND_SHADOW_VOLUME the transmittances come from the baked volume (shadowBake.slang),
// otherwise they are integrated inline.
module lighting;

import component;
import field;
import integral;
import radiative_transfer;
import shadow_cache;
import shadow_rays;
import shadow_volume;
import volume_puffs;
import volume_shell;

#ifdef WIND_SHADOW_VOLUME
[[vk::binding(2, 1)]] public Sampler3D shadowVolumeSampler;
#endif

// x: transmittance toward the sun, y: cosine-weighted transmittance over the sky.
float2 windShadowTransmittances(float3 position, float3 lightDir) {
#ifdef WIND_SHADOW_VOLUME
    WindShadowGrid grid;
    return shadowVolumeTransmittances(grid, shadowVolumeSampler, position, windShadowSlot());
#else
    WindShadowMedium medium;
    return float2(sunTransmittance(medium, position, lightDir), skyTransmittance(medium, position));
#endif
}

float windInScatterSource(float3 position, float3 lightPosition, float3 viewDir) {
    float3 lightDir = normalize(lightPosition - position);
    float2 transmittances = windShadowTransmittances(position, lightDir);
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

WindCellModulation windCellModulation(WindCellModulation previous, float3 o, float3 d, float tNear, float step, int cell) {
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
public float3 windSingleScatterRadiance(
    float3 o, float3 d, float tNear, float tFar, float3 lightPosition,
    out float opticalDepth, out int knotCount) {
    opticalDepth = 0.0;
    knotCount = 0;
    if (tFar <= tNear) {
        return float3(0.0);
    }

    VolumeShell shell = windShell();
    float knots[RAY_MAX_KNOTS];
    RayPuffs puffs;
    knotCount = windRayKnots(shell, o, d, tNear, tFar, knots, puffs);
    float activeLength = windActiveLength(shell, o, d, knots, knotCount, puffs);
    if (activeLength <= WIND_EMPTY_INTERVAL_EPSILON) {
        return float3(0.0);
    }
    float step = windModulationStep(d, activeLength);
    float3 viewDir = normalize(d);

    float radiance = 0.0;
    WindCellModulation modulation;
    modulation.sampled = false;
    modulation.cell = 0;
    modulation.a = 1.0;
    modulation.b = 1.0;
    for (int i = 1; i < knotCount; ++i) {
        float pieceStart = knots[i - 1];
        float pieceEnd = knots[i];
        if (!windPieceIsActive(shell, o, d, puffs, pieceStart, pieceEnd)) {
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
            float modulation0 = lerp(modulation.a, modulation.b, (s0 - cellStart) / step);
            float modulation1 = lerp(modulation.a, modulation.b, (s1 - cellStart) / step);
            float depth = shellPieceOpticalDepthModulated(shell, o, d, s0, s1, modulation0, modulation1)
                + windPuffPieceOpticalDepth(shell, puffs, o, d, s0, s1);
            if (depth <= 0.0) {
                continue;
            }

            float3 node = o + d * (0.5 * (cellStart + min(cellStart + step, tFar)));
            float source = windInScatterSource(node, lightPosition, viewDir);
            radiance += rteTransmittanceFromOpticalDepth(opticalDepth) * source
                * (1.0 - rteTransmittanceFromOpticalDepth(depth));
            opticalDepth += depth;
        }
    }
    return radiance * wind.albedo.rgb;
}
