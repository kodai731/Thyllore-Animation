#ifndef WIND_SHADOW_VOLUME_GLSL
#define WIND_SHADOW_VOLUME_GLSL

// Wall + envelope transmittance cache of the wind cone; the grid is include/shadow_volume.glsl.
// Extents are mirrored in thyllore-effect-core/src/wind/gpu/components/shadow_volume.rs.
// Must be included after shell_field.glsl.

const int SHADOW_VOLUME_RADIAL = 48;
const int SHADOW_VOLUME_HEIGHT = 48;
const int SHADOW_VOLUME_THETA = 64;
const int SHADOW_VOLUME_SLOTS = 4;

#include "include/shadow_volume.glsl"

float shadowVolumeTopY() {
    return windHTop() * windHeight();
}

float shadowVolumeWallRadius(float y) {
    return sqrt(max(windWallRadiusSq(y / windHeight()), 0.0));
}

float shadowVolumeRadialExtent() {
    return windShadowRadialExtent();
}

#endif
