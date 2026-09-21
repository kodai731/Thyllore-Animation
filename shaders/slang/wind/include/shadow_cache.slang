// Wall + envelope transmittance cache of the wind cone on the grid of include/shadow_volume.slang.
// Extents are mirrored in thyllore-effect-core/src/wind/gpu/components/shadow_volume.rs.
module shadow_cache;

import component;
import field;
import shadow_volume;

public struct WindShadowGrid : IShadowVolumeGrid {
    public static const int RADIAL = 48;
    public static const int HEIGHT = 48;
    public static const int THETA = 64;
    public static const int SLOTS = 4;

    public float topY() {
        return windHTop() * windHeight();
    }

    public float wallRadius(float y) {
        return sqrt(max(windWallRadiusSq(y / windHeight()), 0.0));
    }

    public float radialExtent() {
        return windShadowRadialExtent();
    }
};
