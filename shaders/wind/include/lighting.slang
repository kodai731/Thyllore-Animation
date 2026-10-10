// Single scattering along the view ray: the ray is cut into equal cells, each cell's optical
// depth is the closed-form sum over the knot pieces inside it, and the in-scatter source is
// evaluated once at the cell midpoint, shadowed toward the sun and over the sky hemisphere.
// With WIND_SHADOW_VOLUME the transmittances come from the baked volume (shadowBakeCompute.slang),
// otherwise they are integrated inline.
module wind_lighting;

import "wind/include/component.slang";
import "wind/include/field.slang";
import "wind/include/integral.slang";
import "wind/include/shadow_volume.slang";
import "include/radiative_transfer.slang";
import "include/shadow_rays.slang";
import "include/shadow_volume.slang";
import "include/volume_puffs.slang";
import "include/volume_shell.slang";

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

// Pieces are walked in ray order and each is cut by the cells it overlaps; a cell shared by two
// pieces composes exactly because the transmittance accumulates between the two parts.
struct WindScatterSink : IWindCellSink {
    float3 lightPosition;
    float3 viewDir;
    float radiance;

    [mutating] void addCell(float3 o, float3 d, float tNear, float tFar, float cellStart, float step, float opticalDepthBefore, float depth) {
        if (depth <= 0.0) {
            return;
        }
        float3 node = o + d * (0.5 * (cellStart + min(cellStart + step, tFar)));
        float source = windInScatterSource(node, lightPosition, viewDir);
        radiance += rteTransmittanceFromOpticalDepth(opticalDepthBefore) * source
            * (1.0 - rteTransmittanceFromOpticalDepth(depth));
    }
};

public float3 windSingleScatterRadiance(
    float3 o, float3 d, float tNear, float tFar, float3 lightPosition,
    out float opticalDepth, out int knotCount) {
    WindScatterSink sink;
    sink.lightPosition = lightPosition;
    sink.viewDir = normalize(d);
    sink.radiance = 0.0;
    opticalDepth = windWalkCells(o, d, tNear, tFar, sink, knotCount);
    return sink.radiance * wind.albedo.rgb;
}
