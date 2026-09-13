#ifndef SHADOW_VOLUME_GLSL
#define SHADOW_VOLUME_GLSL

// Cylindrical grid (radius, height, angle) that caches the transmittance toward the sun (x) and
// averaged over the sky (y) for every instance slot side by side along the radius axis. The
// radius axis is measured from the wall radius at the texel's height, so a shell sits on the
// same texels at every height and its steep transmittance gradient never drifts over texels.
// The including effect defines SHADOW_VOLUME_RADIAL / HEIGHT / THETA / SLOTS before this
// include and implements the three prototypes below.

#include "include/common.glsl"

float shadowVolumeTopY();
float shadowVolumeWallRadius(float y);
float shadowVolumeRadialExtent();

vec3 shadowVolumeTexelPosition(ivec3 texel) {
    float y = (float(texel.y) + 0.5) / float(SHADOW_VOLUME_HEIGHT) * shadowVolumeTopY();
    float wallOffset = ((float(texel.x) + 0.5) / float(SHADOW_VOLUME_RADIAL) * 2.0 - 1.0)
        * shadowVolumeRadialExtent();
    float radius = max(shadowVolumeWallRadius(y) + wallOffset, 0.0);
    float theta = (float(texel.z) + 0.5) / float(SHADOW_VOLUME_THETA) * TWO_PI - PI;
    return vec3(radius * cos(theta), y, radius * sin(theta));
}

// The radius coordinate is clamped to the slot's own texel centres so linear filtering never
// bleeds into the neighbouring instance; height clamps and angle wraps in the sampler.
vec3 shadowVolumeUvw(vec3 local, int slot) {
    float halfTexel = 0.5 / float(SHADOW_VOLUME_RADIAL);
    float wallOffset = length(local.xz) - shadowVolumeWallRadius(local.y);
    float radial = clamp(
        wallOffset / shadowVolumeRadialExtent() * 0.5 + 0.5, halfTexel, 1.0 - halfTexel);
    float u = (float(slot) + radial) / float(SHADOW_VOLUME_SLOTS);
    float v = local.y / shadowVolumeTopY();
    float w = (atan(local.z, local.x) + PI) / TWO_PI;
    return vec3(u, v, w);
}

// The height axis clamps to texel centres and the angle axis wraps by blending the two seam
// texels explicitly, so the result does not depend on the sampler's address modes.
vec2 shadowVolumeTransmittances(sampler3D volume, vec3 local, int slot) {
    vec3 uvw = shadowVolumeUvw(local, slot);
    float halfTexelV = 0.5 / float(SHADOW_VOLUME_HEIGHT);
    uvw.y = clamp(uvw.y, halfTexelV, 1.0 - halfTexelV);

    float halfTexelW = 0.5 / float(SHADOW_VOLUME_THETA);
    float seamSpan = 2.0 * halfTexelW;
    if (uvw.z >= halfTexelW && uvw.z <= 1.0 - halfTexelW) {
        return texture(volume, uvw).xy;
    }
    float seamOffset = uvw.z < halfTexelW ? uvw.z + halfTexelW : uvw.z - (1.0 - halfTexelW);
    vec2 lastSlice = texture(volume, vec3(uvw.xy, 1.0 - halfTexelW)).xy;
    vec2 firstSlice = texture(volume, vec3(uvw.xy, halfTexelW)).xy;
    return mix(lastSlice, firstSlice, seamOffset / seamSpan);
}

#endif
