#ifndef SHADOW_RAYS_GLSL
#define SHADOW_RAYS_GLSL

// Sun and sky transmittance of a participating medium from the optical depth its shadow rays
// accumulate. The including effect implements shadowOpticalDepthToward for its own medium.

#include "include/common.glsl"
#include "include/radiative_transfer.glsl"

const int SKY_TILTED_DIRECTIONS = 6;
const float SKY_TILT_COSINE = 0.70710678;

float shadowOpticalDepthToward(vec3 origin, vec3 direction);

float sunTransmittance(vec3 position, vec3 lightDir) {
    return rteTransmittanceFromOpticalDepth(shadowOpticalDepthToward(position, lightDir));
}

// Cosine-weighted average over the zenith and six directions tilted 45 degrees, so a uniform
// sky lights the medium from every side instead of through one vertical shadow ray.
float skyTransmittance(vec3 position) {
    float weightedSum = rteTransmittanceFromOpticalDepth(
        shadowOpticalDepthToward(position, vec3(0.0, 1.0, 0.0)));
    float weightSum = 1.0;
    for (int i = 0; i < SKY_TILTED_DIRECTIONS; ++i) {
        float azimuth = TWO_PI * float(i) / float(SKY_TILTED_DIRECTIONS);
        vec3 direction = vec3(
            cos(azimuth) * SKY_TILT_COSINE, SKY_TILT_COSINE, sin(azimuth) * SKY_TILT_COSINE);
        weightedSum += SKY_TILT_COSINE
            * rteTransmittanceFromOpticalDepth(shadowOpticalDepthToward(position, direction));
        weightSum += SKY_TILT_COSINE;
    }
    return weightedSum / weightSum;
}

#endif
