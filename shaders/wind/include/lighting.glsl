#ifndef WIND_LIGHTING_GLSL
#define WIND_LIGHTING_GLSL

// Single scattering along the view ray: per closed-form piece the in-scatter source is
// averaged over fixed midpoint nodes weighted by the local density, each node shadowed by
// the wall + envelope field toward the sun and over the sky hemisphere. With
// WIND_SHADOW_VOLUME the transmittances come from the baked volume (shadowBake.comp),
// otherwise they are integrated inline.
// Must be included after shell_integral.glsl (and shadow_volume.glsl under WIND_SHADOW_VOLUME).

#include "include/radiative_transfer.glsl"

const int WIND_SCATTER_NODES = 4;

// x: transmittance toward the sun, y: cosine-weighted transmittance over the sky.
vec2 windShadowTransmittances(vec3 position, vec3 lightDir) {
#ifdef WIND_SHADOW_VOLUME
    return texture(shadowVolumeSampler, windShadowVolumeUvw(position, windShadowSlot())).xy;
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

// Density weights keep the piece average independent of where knots split the piece.
float windPieceInScatter(vec3 o, vec3 d, float s0, float s1, vec3 lightPosition, vec3 viewDir) {
    float pieceLength = s1 - s0;
    float weightedSum = 0.0;
    float weightSum = 0.0;
    float plainSum = 0.0;
    for (int i = 0; i < WIND_SCATTER_NODES; ++i) {
        vec3 node = o + d * (s0 + rteMidpointDistance(i, WIND_SCATTER_NODES, pieceLength));
        float source = windInScatterSource(node, lightPosition, viewDir);
        float weight = windDensityAt(node);
        weightedSum += weight * source;
        weightSum += weight;
        plainSum += source;
    }
    if (weightSum <= 0.0) {
        return plainSum / float(WIND_SCATTER_NODES);
    }
    return weightedSum / weightSum;
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
    vec3 viewDir = normalize(d);

    float radiance = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        float pieceDepth = windPieceOpticalDepth(o, d, knots[i - 1], knots[i])
            + windPuffPieceOpticalDepth(puffs, o, d, knots[i - 1], knots[i]);
        float frontTransmittance = rteTransmittanceFromOpticalDepth(opticalDepth);
        float source = windPieceInScatter(o, d, knots[i - 1], knots[i], lightPosition, viewDir);
        radiance += frontTransmittance * source * (1.0 - rteTransmittanceFromOpticalDepth(pieceDepth));
        opticalDepth += pieceDepth;
    }
    return radiance * wind.albedo.rgb;
}

#endif
