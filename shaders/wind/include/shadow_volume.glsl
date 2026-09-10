#ifndef WIND_SHADOW_VOLUME_GLSL
#define WIND_SHADOW_VOLUME_GLSL

// Cylindrical grid (radius, height, angle) over the wind cone that caches the wall + envelope
// optical depth toward the sun (x) and the zenith (y) for every instance slot side by side
// along the radius axis. Extents are mirrored in thyllore-effect-core/src/wind/gpu/components/shadow_volume.rs.
// Must be included after shell_field.glsl.

#include "include/common.glsl"

const int WIND_SHADOW_RADIAL = 48;
const int WIND_SHADOW_HEIGHT = 48;
const int WIND_SHADOW_THETA = 64;
const int WIND_SHADOW_SLOTS = 4;

float windShadowRadiusMax() {
    return max(windEnvelopeRadius(0.0), windEnvelopeRadius(windHTop()));
}

float windShadowTopY() {
    return windHTop() * windHeight();
}

vec3 windShadowTexelPosition(ivec3 texel) {
    float radius = (float(texel.x) + 0.5) / float(WIND_SHADOW_RADIAL) * windShadowRadiusMax();
    float y = (float(texel.y) + 0.5) / float(WIND_SHADOW_HEIGHT) * windShadowTopY();
    float theta = (float(texel.z) + 0.5) / float(WIND_SHADOW_THETA) * TWO_PI - PI;
    return vec3(radius * cos(theta), y, radius * sin(theta));
}

// The radius coordinate is clamped to the slot's own texel centres so linear filtering never
// bleeds into the neighbouring instance; height clamps and angle wraps in the sampler.
vec3 windShadowVolumeUvw(vec3 local, int slot) {
    float halfTexel = 0.5 / float(WIND_SHADOW_RADIAL);
    float radial = clamp(length(local.xz) / windShadowRadiusMax(), halfTexel, 1.0 - halfTexel);
    float u = (float(slot) + radial) / float(WIND_SHADOW_SLOTS);
    float v = local.y / windShadowTopY();
    float w = (atan(local.z, local.x) + PI) / TWO_PI;
    return vec3(u, v, w);
}

#endif
