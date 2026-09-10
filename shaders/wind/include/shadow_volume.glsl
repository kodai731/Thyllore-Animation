#ifndef WIND_SHADOW_VOLUME_GLSL
#define WIND_SHADOW_VOLUME_GLSL

// Cylindrical grid (radius, height, angle) over the wind cone that caches the wall + envelope
// transmittance toward the sun (x) and averaged over the sky (y) for every instance slot side
// by side along the radius axis. The radius axis is measured from the wall radius at the
// texel's height, so the shell sits on the same texels at every height and the steep
// transmittance gradient across it never drifts over texel boundaries.
// Extents are mirrored in thyllore-effect-core/src/wind/gpu/components/shadow_volume.rs.
// Must be included after shell_field.glsl.

#include "include/common.glsl"

const int WIND_SHADOW_RADIAL = 48;
const int WIND_SHADOW_HEIGHT = 48;
const int WIND_SHADOW_THETA = 64;
const int WIND_SHADOW_SLOTS = 4;

float windShadowTopY() {
    return windHTop() * windHeight();
}

float windShadowWallRadius(float y) {
    return sqrt(max(windWallRadiusSq(y / windHeight()), 0.0));
}

vec3 windShadowTexelPosition(ivec3 texel) {
    float y = (float(texel.y) + 0.5) / float(WIND_SHADOW_HEIGHT) * windShadowTopY();
    float wallOffset = ((float(texel.x) + 0.5) / float(WIND_SHADOW_RADIAL) * 2.0 - 1.0)
        * windShadowRadialExtent();
    float radius = max(windShadowWallRadius(y) + wallOffset, 0.0);
    float theta = (float(texel.z) + 0.5) / float(WIND_SHADOW_THETA) * TWO_PI - PI;
    return vec3(radius * cos(theta), y, radius * sin(theta));
}

// The radius coordinate is clamped to the slot's own texel centres so linear filtering never
// bleeds into the neighbouring instance; height clamps and angle wraps in the sampler.
vec3 windShadowVolumeUvw(vec3 local, int slot) {
    float halfTexel = 0.5 / float(WIND_SHADOW_RADIAL);
    float wallOffset = length(local.xz) - windShadowWallRadius(local.y);
    float radial = clamp(
        wallOffset / windShadowRadialExtent() * 0.5 + 0.5, halfTexel, 1.0 - halfTexel);
    float u = (float(slot) + radial) / float(WIND_SHADOW_SLOTS);
    float v = local.y / windShadowTopY();
    float w = (atan(local.z, local.x) + PI) / TWO_PI;
    return vec3(u, v, w);
}

#endif
